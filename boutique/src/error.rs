use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use maud::Markup;
use validator::ValidationErrors;

use crate::{htmx::fragments, store::StoreError};

pub enum AppError {
    NotFound,
    BadRequest,
    Status(StatusCode),
    Internal(String),
    Validation(ValidationErrors),
    Fields(Vec<(&'static str, String)>),
    Message(String),
}

pub type AppResult<T = Response> = Result<T, AppError>;

const INTERNAL_MESSAGE: &str = "Something went wrong. Please try again.";

#[derive(Clone, Debug)]
pub struct ErrorReport {
    pub status: StatusCode,
    pub messages: Vec<(Option<String>, String)>,
}

impl ErrorReport {
    pub fn new(status: StatusCode) -> Self {
        Self { status, messages: Vec::new() }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for ErrorReport {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<ErrorReport>().cloned().ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

fn validation_fields(errors: &ValidationErrors) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = errors
        .field_errors()
        .into_iter()
        .filter_map(|(field, errors)| {
            let message = errors.first()?.message.as_ref().map(|message| message.to_string()).unwrap_or_else(|| format!("Invalid {field}"));
            Some((field.to_string(), message))
        })
        .collect();
    fields.sort();
    fields
}

fn field_fragments(fields: &[(String, String)]) -> Markup {
    let fields: Vec<(&str, Option<&str>)> = fields.iter().map(|(field, message)| (field.as_str(), Some(message.as_str()))).collect();
    fragments::field_errors(&fields)
}

impl AppError {
    pub fn field(field: &'static str, message: impl Into<String>) -> Self {
        AppError::Fields(vec![(field, message.into())])
    }

    pub fn message(message: impl Into<String>) -> Self {
        AppError::Message(message.into())
    }

    pub fn internal(context: &str, error: impl std::fmt::Debug) -> Self {
        AppError::Internal(format!("{context}: {error:?}"))
    }
}

#[cfg(feature = "sea-orm")]
impl From<sea_orm::DbErr> for AppError {
    fn from(error: sea_orm::DbErr) -> Self {
        AppError::internal("database", error)
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
        let (status, messages, body) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, Vec::new(), None),
            AppError::BadRequest => (StatusCode::BAD_REQUEST, Vec::new(), None),
            AppError::Status(status) => (status, Vec::new(), None),
            AppError::Internal(message) => {
                tracing::error!("{message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    vec![(None, INTERNAL_MESSAGE.to_string())],
                    Some(fragments::error(INTERNAL_MESSAGE)),
                )
            }
            AppError::Validation(errors) => {
                let fields = validation_fields(&errors);
                let body = field_fragments(&fields);
                let messages = fields.into_iter().map(|(field, message)| (Some(field), message)).collect();
                (StatusCode::UNPROCESSABLE_ENTITY, messages, Some(body))
            }
            AppError::Fields(fields) => {
                let fields: Vec<(String, String)> = fields.into_iter().map(|(field, message)| (field.to_string(), message)).collect();
                let body = field_fragments(&fields);
                let messages = fields.into_iter().map(|(field, message)| (Some(field), message)).collect();
                (StatusCode::UNPROCESSABLE_ENTITY, messages, Some(body))
            }
            AppError::Message(message) => {
                let body = fragments::error(&message);
                (StatusCode::UNPROCESSABLE_ENTITY, vec![(None, message)], Some(body))
            }
        };

        let mut response = match body {
            Some(body) => (status, body).into_response(),
            None => status.into_response(),
        };
        response.extensions_mut().insert(ErrorReport { status, messages });
        response
    }
}
