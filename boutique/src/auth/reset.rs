use std::{error::Error, fmt};

use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::{
    AuthState, expiry,
    password::{self, HashError},
    store::{AuthStore, AuthUser, StoreError},
};
use crate::{error::AppError, hex};

const TOKEN_TYPE: &str = "password_reset";

#[derive(Debug)]
pub enum ResetError {
    InvalidToken,
    Store(StoreError),
    Hash(HashError),
}

impl fmt::Display for ResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResetError::InvalidToken => f.write_str("invalid or expired password reset token"),
            ResetError::Store(_) => f.write_str("password reset failed"),
            ResetError::Hash(_) => f.write_str("password reset failed"),
        }
    }
}

impl Error for ResetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ResetError::InvalidToken => None,
            ResetError::Store(error) => Some(error),
            ResetError::Hash(error) => Some(error),
        }
    }
}

impl From<StoreError> for ResetError {
    fn from(e: StoreError) -> Self {
        ResetError::Store(e)
    }
}

impl From<ResetError> for AppError {
    fn from(error: ResetError) -> Self {
        match error {
            ResetError::InvalidToken => AppError::message("This link is invalid or has expired"),
            ResetError::Store(error) => error.into(),
            ResetError::Hash(error) => AppError::internal("password reset", error),
        }
    }
}

fn mac(secret: &[u8], user_id: &str, expires: i64, password_hash: &str) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac accepts keys of any length");
    mac.update(TOKEN_TYPE.as_bytes());
    mac.update(b"\0");
    mac.update(user_id.as_bytes());
    mac.update(b"\0");
    mac.update(expires.to_string().as_bytes());
    mac.update(b"\0");
    mac.update(password_hash.as_bytes());
    mac
}

/// returns `None` for an unknown email. callers must show the same message
/// either way so the endpoint does not reveal which emails have accounts.
pub async fn request<S: AuthStore>(state: &AuthState<S>, email: &str) -> Result<Option<(S::User, String)>, ResetError> {
    let Some(user) = state.store.find_user_by_email(email).await.map_err(StoreError::new)? else {
        return Ok(None);
    };

    let token = token_for(state, &user);
    Ok(Some((user, token)))
}

pub fn token_for<S: AuthStore>(state: &AuthState<S>, user: &S::User) -> String {
    let expires = expiry(Utc::now(), state.config.reset_ttl).timestamp();
    let signature = mac(state.secret(), user.id(), expires, user.password_hash())
        .finalize()
        .into_bytes();
    format!("{}.{expires}.{}", hex::encode(user.id().as_bytes()), hex::encode(&signature))
}

pub async fn verify_token<S: AuthStore>(state: &AuthState<S>, token: &str) -> Result<S::User, ResetError> {
    let mut parts = token.split('.');
    let (Some(user_id), Some(expires), Some(signature), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(ResetError::InvalidToken);
    };

    let user_id = hex::decode(user_id)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(ResetError::InvalidToken)?;
    let expires: i64 = expires.parse().map_err(|_| ResetError::InvalidToken)?;
    let signature = hex::decode(signature).ok_or(ResetError::InvalidToken)?;

    if expires <= Utc::now().timestamp() {
        return Err(ResetError::InvalidToken);
    }

    let user = state
        .store
        .find_user_by_id(&user_id)
        .await
        .map_err(StoreError::new)?
        .ok_or(ResetError::InvalidToken)?;

    mac(state.secret(), user.id(), expires, user.password_hash())
        .verify_slice(&signature)
        .map_err(|_| ResetError::InvalidToken)?;

    Ok(user)
}

pub async fn complete<S: AuthStore>(state: &AuthState<S>, token: &str, new_password: &str) -> Result<S::User, ResetError> {
    let user = verify_token(state, token).await?;
    let hash = password::hash_off_runtime(new_password).await.map_err(ResetError::Hash)?;

    Ok(state.store.reset_password(user, hash).await.map_err(StoreError::new)?)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::auth::{
        AuthConfig,
        testing::{Store, User, block_on, state, user},
    };

    fn verify(state: &AuthState<Store>, token: &str) -> Result<User, ResetError> {
        block_on(verify_token(state, token))
    }

    #[test]
    fn valid_token() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        assert_eq!(verify(&state, &token).unwrap().id, "alice");
    }

    #[test]
    fn expired_token() {
        let state = state().with_config(AuthConfig::default().reset_ttl(Duration::ZERO));
        let token = token_for(&state, &user("alice"));
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn tampered_user_id() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        let (_, rest) = token.split_once('.').unwrap();
        let token = format!("{}.{rest}", hex::encode(b"bob"));
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn tampered_expiry() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        let mut parts = token.split('.');
        let (user_id, expires, signature) = (parts.next().unwrap(), parts.next().unwrap(), parts.next().unwrap());
        let later = expires.parse::<i64>().unwrap() + 3600;
        let token = format!("{user_id}.{later}.{signature}");
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn tampered_mac() {
        let state = state();
        let mut token = token_for(&state, &user("alice"));
        let last = if token.ends_with('0') { '1' } else { '0' };
        token.pop();
        token.push(last);
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn token_issued_before_password_change() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        state.store.0.lock().unwrap()[0].password_hash = "changed".into();
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }
}
