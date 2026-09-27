use axum::{body::Bytes, extract::Multipart};

pub fn blank_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

pub struct Upload {
    pub file_name: Option<String>,
    pub bytes: Bytes,
}

pub enum UploadError {
    Missing,
    Unreadable,
}

pub async fn single_file(multipart: &mut Multipart, name: &str) -> Result<Upload, UploadError> {
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => return Err(UploadError::Missing),
            Err(error) => {
                eprintln!("[{name}] {error}");
                return Err(UploadError::Unreadable);
            }
        };

        if field.name() != Some(name) {
            continue;
        }

        let file_name = field.file_name().map(str::to_string);
        let bytes = field.bytes().await.map_err(|error| {
            eprintln!("[{name}] {error}");
            UploadError::Unreadable
        })?;

        return Ok(Upload { file_name, bytes });
    }
}
