use std::{borrow::Cow, error::Error, fmt};

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use validator::ValidationErrors;

use crate::{htmx::fragments, store::StoreError};

#[derive(Debug)]
pub enum AppError {
    NotFound,
    BadRequest,
    Status(StatusCode),
    Internal(String),
    Validation(ValidationErrors),
    Fields(Vec<ErrorMessage>),
    Message(String),
}

pub type AppResult<T = Response> = Result<T, AppError>;

const INTERNAL_MESSAGE: &str = "Something went wrong. Please try again.";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ErrorMessage {
    pub field: Option<Cow<'static, str>>,
    pub text: String,
}

impl ErrorMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            field: None,
            text: text.into(),
        }
    }

    pub fn field(field: impl Into<Cow<'static, str>>, text: impl Into<String>) -> Self {
        Self {
            field: Some(field.into()),
            text: text.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ErrorReport {
    pub status: StatusCode,
    pub messages: Vec<ErrorMessage>,
}

impl ErrorReport {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            messages: Vec::new(),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ErrorReport {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<ErrorReport>()
            .cloned()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

fn validation_messages(errors: &ValidationErrors) -> Vec<ErrorMessage> {
    let mut messages: Vec<ErrorMessage> = errors
        .field_errors()
        .into_iter()
        .filter_map(|(field, errors)| {
            let text = errors
                .first()?
                .message
                .as_ref()
                .map(|message| message.to_string())
                .unwrap_or_else(|| format!("Invalid {field}"));
            Some(ErrorMessage::field(field.to_string(), text))
        })
        .collect();
    messages.sort();
    messages
}

impl AppError {
    pub fn field(field: impl Into<Cow<'static, str>>, text: impl Into<String>) -> Self {
        AppError::Fields(vec![ErrorMessage::field(field, text)])
    }

    pub fn message(message: impl Into<String>) -> Self {
        AppError::Message(message.into())
    }

    pub fn internal(context: &str, error: impl fmt::Debug) -> Self {
        AppError::Internal(format!("{context}: {error:?}"))
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound => f.write_str("not found"),
            AppError::BadRequest => f.write_str("bad request"),
            AppError::Status(status) => write!(f, "{status}"),
            AppError::Internal(message) => write!(f, "internal error: {message}"),
            AppError::Validation(_) => f.write_str("validation failed"),
            AppError::Fields(messages) => {
                f.write_str("invalid fields:")?;
                for message in messages {
                    write!(f, " {}", message.field.as_deref().unwrap_or("?"))?;
                }
                Ok(())
            }
            AppError::Message(message) => f.write_str(message),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Validation(errors) => Some(errors),
            _ => None,
        }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        AppError::internal("store", error)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::internal("json", error)
    }
}

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        AppError::Validation(errors)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, messages, with_body) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, Vec::new(), false),
            AppError::BadRequest => (StatusCode::BAD_REQUEST, Vec::new(), false),
            AppError::Status(status) => (status, Vec::new(), false),
            AppError::Internal(error) => {
                tracing::error!(%error, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, vec![ErrorMessage::new(INTERNAL_MESSAGE)], true)
            }
            AppError::Validation(errors) => (StatusCode::UNPROCESSABLE_ENTITY, validation_messages(&errors), true),
            AppError::Fields(messages) => (StatusCode::UNPROCESSABLE_ENTITY, messages, true),
            AppError::Message(message) => (StatusCode::UNPROCESSABLE_ENTITY, vec![ErrorMessage::new(message)], true),
        };

        let mut response = if with_body {
            (status, fragments::messages(&messages)).into_response()
        } else {
            status.into_response()
        };
        response.extensions_mut().insert(ErrorReport { status, messages });
        response
    }
}
