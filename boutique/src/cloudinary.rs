use std::{env, fmt};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Clone)]
pub struct Cloudinary {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

#[derive(Serialize)]
pub struct Signature {
    pub signature: String,
    pub timestamp: i64,
    pub api_key: String,
    pub cloud_name: String,
}

#[derive(Debug)]
pub enum Error {
    Request(reqwest::Error),
    Rejected(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Request(error) => write!(f, "cloudinary request: {error}"),
            Error::Rejected(message) => write!(f, "cloudinary rejected the upload: {message}"),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Request(error)
    }
}

#[derive(Deserialize)]
struct UploadResponse {
    public_id: Option<String>,
    error: Option<UploadError>,
}

#[derive(Deserialize)]
struct UploadError {
    message: String,
}

impl Cloudinary {
    pub fn new(cloud_name: impl Into<String>, api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self { cloud_name: cloud_name.into(), api_key: api_key.into(), api_secret: api_secret.into() }
    }

    pub fn from_env() -> Option<Self> {
        if let Ok(raw) = env::var("CLOUDINARY_URL") {
            return Self::from_url(&raw);
        }

        match (env::var("CLOUDINARY_CLOUD_NAME"), env::var("CLOUDINARY_API_KEY"), env::var("CLOUDINARY_API_SECRET")) {
            (Ok(cloud_name), Ok(api_key), Ok(api_secret)) => Some(Self::new(cloud_name, api_key, api_secret)),
            _ => None,
        }
    }

    pub fn from_url(raw: &str) -> Option<Self> {
        let url = url::Url::parse(raw).ok()?;
        let cloud_name = url.host_str()?;
        let api_secret = url.password()?;
        Some(Self::new(cloud_name, url.username(), api_secret))
    }

    pub fn cloud_name(&self) -> &str {
        &self.cloud_name
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn url(&self, public_id: &str, transform: &str) -> String {
        format!("https://res.cloudinary.com/{}/image/upload/{transform}/{public_id}", self.cloud_name)
    }

    pub fn sign(&self, parameters: &[(&str, &str)]) -> String {
        let mut sorted: Vec<&(&str, &str)> = parameters.iter().collect();
        sorted.sort_by(|left, right| left.0.cmp(right.0));
        let joined = sorted.iter().map(|(name, value)| format!("{name}={value}")).collect::<Vec<_>>().join("&");
        let digest = Sha1::digest(format!("{joined}{}", self.api_secret).as_bytes());
        format!("{digest:x}")
    }

    pub fn signature(&self) -> Signature {
        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.sign(&[("timestamp", &timestamp.to_string())]);
        Signature { signature, timestamp, api_key: self.api_key.clone(), cloud_name: self.cloud_name.clone() }
    }

    pub async fn upload(&self, bytes: Vec<u8>, public_id: &str, content_type: &str) -> Result<String, Error> {
        let timestamp = chrono::Utc::now().timestamp().to_string();

        let parameters = [
            ("invalidate", "true"),
            ("overwrite", "true"),
            ("public_id", public_id),
            ("timestamp", timestamp.as_str()),
        ];
        let signature = self.sign(&parameters);

        let file = reqwest::multipart::Part::bytes(bytes).file_name("upload").mime_str(content_type)?;

        let mut form = reqwest::multipart::Form::new()
            .text("api_key", self.api_key.clone())
            .text("signature", signature)
            .part("file", file);
        for (name, value) in parameters {
            form = form.text(name, value.to_string());
        }

        let endpoint = format!("https://api.cloudinary.com/v1_1/{}/image/upload", self.cloud_name);
        let response: UploadResponse = reqwest::Client::new().post(endpoint).multipart(form).send().await?.json().await?;

        match (response.public_id, response.error) {
            (Some(public_id), _) => Ok(public_id),
            (None, Some(error)) => Err(Error::Rejected(error.message)),
            (None, None) => Err(Error::Rejected("no public_id in response".to_string())),
        }
    }
}
