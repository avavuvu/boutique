use std::{error::Error, fmt};

use axum::{body::Bytes, extract::Multipart};

use crate::error::AppError;

pub fn blank_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

pub struct Upload {
    pub file_name: Option<String>,
    pub bytes: Bytes,
}

#[derive(Debug)]
pub enum UploadError {
    Missing,
    Unreadable,
}

impl fmt::Display for UploadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UploadError::Missing => f.write_str("no file was uploaded"),
            UploadError::Unreadable => f.write_str("the upload could not be read"),
        }
    }
}

impl Error for UploadError {}

impl From<UploadError> for AppError {
    fn from(error: UploadError) -> Self {
        match error {
            UploadError::Missing => AppError::message("No file was uploaded"),
            UploadError::Unreadable => AppError::message("The upload could not be read"),
        }
    }
}

pub async fn single_file(multipart: &mut Multipart, name: &str) -> Result<Upload, UploadError> {
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => return Err(UploadError::Missing),
            Err(error) => {
                tracing::warn!(field = name, "{error}");
                return Err(UploadError::Unreadable);
            }
        };

        if field.name() != Some(name) {
            continue;
        }

        let file_name = field.file_name().map(str::to_string);
        let bytes = field.bytes().await.map_err(|error| {
            tracing::warn!(field = name, "{error}");
            UploadError::Unreadable
        })?;

        return Ok(Upload { file_name, bytes });
    }
}
