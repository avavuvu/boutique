use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use validator::ValidationErrors;

use crate::htmx::fragments;
use sea_orm::DbErr;

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

impl From<DbErr> for AppError {
    fn from(error: DbErr) -> Self {
        AppError::internal("database", error)
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
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND.into_response(),
            AppError::BadRequest => StatusCode::BAD_REQUEST.into_response(),
            AppError::Status(status) => status.into_response(),
            AppError::Internal(message) => {
                eprintln!("[error] {message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    fragments::error("Something went wrong. Please try again."),
                )
                    .into_response()
            }
            AppError::Validation(errors) => {
                (StatusCode::UNPROCESSABLE_ENTITY, fragments::from_errors(errors)).into_response()
            }
            AppError::Fields(fields) => {
                let fields: Vec<(&str, Option<&str>)> =
                    fields.iter().map(|(field, message)| (*field, Some(message.as_str()))).collect();
                (StatusCode::UNPROCESSABLE_ENTITY, fragments::field_errors(&fields)).into_response()
            }
            AppError::Message(message) => {
                (StatusCode::UNPROCESSABLE_ENTITY, fragments::error(&message)).into_response()
            }
        }
    }
}
