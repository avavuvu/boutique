use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

use super::ErrorMessage;

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
