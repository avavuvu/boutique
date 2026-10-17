use std::{error, fmt, sync::LazyLock};

use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

#[derive(Debug)]
pub struct HashError(argon2::password_hash::Error);

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("could not hash the password")
    }
}

impl error::Error for HashError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

pub fn hash(password: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(HashError)?
        .to_string())
}

static UNKNOWN_USER_HASH: LazyLock<String> = LazyLock::new(|| hash("unknown user").unwrap_or_default());

pub async fn hash_off_runtime(password: &str) -> Result<String, HashError> {
    let password = password.to_owned();
    off_runtime(move || hash(&password)).await
}

pub(crate) async fn verify_off_runtime(password: &str, hash: Option<&str>) -> bool {
    let password = password.to_owned();
    let is_known = hash.is_some();
    let hash = hash.map_or_else(|| UNKNOWN_USER_HASH.clone(), str::to_owned);
    off_runtime(move || verify(&password, &hash)).await && is_known
}

async fn off_runtime<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    match tokio::task::spawn_blocking(work).await {
        Ok(output) => output,
        Err(error) => std::panic::resume_unwind(error.into_panic()),
    }
}

pub fn verify(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}
