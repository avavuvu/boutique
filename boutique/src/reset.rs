use chrono::Utc;
use hmac::{Hmac, Mac};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{
    password,
    state::AuthState,
    store::{AuthStore, AuthUser, StoreError},
};

const TOKEN_TYPE: &str = "password_reset";

#[derive(Debug)]
pub enum ResetError {
    InvalidToken,
    Store(StoreError),
    Hash(argon2::password_hash::Error),
}

impl From<StoreError> for ResetError {
    fn from(e: StoreError) -> Self {
        ResetError::Store(e)
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    typ: String,
    fingerprint: String,
    exp: usize,
}

fn fingerprint(secret: &[u8], password_hash: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac accepts keys of any length");
    mac.update(password_hash.as_bytes());
    format!("{:x}", mac.finalize().into_bytes())
}

/// returns `None` for an unknown email. callers must show the same message
/// either way so the endpoint does not reveal which emails have accounts.
pub async fn request<U: AuthUser>(state: &AuthState<U>, email: &str) -> Result<Option<(U, String)>, ResetError> {
    let Some(user) = state.store.find_user_by_email(email).await.map_err(StoreError::from_debug)? else {
        return Ok(None);
    };

    let token = token_for(state, &user).map_err(|_| ResetError::InvalidToken)?;
    Ok(Some((user, token)))
}

pub fn token_for<U: AuthUser>(state: &AuthState<U>, user: &U) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: user.id().to_string(),
        typ: TOKEN_TYPE.to_string(),
        fingerprint: fingerprint(state.secret(), user.password_hash()),
        exp: (Utc::now() + chrono::Duration::hours(state.config.reset_ttl_hours)).timestamp() as usize,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(state.secret()))
}

pub async fn verify_token<U: AuthUser>(state: &AuthState<U>, token: &str) -> Result<U, ResetError> {
    let claims = decode::<Claims>(token, &DecodingKey::from_secret(state.secret()), &Validation::default())
        .map_err(|_| ResetError::InvalidToken)?
        .claims;

    if claims.typ != TOKEN_TYPE {
        return Err(ResetError::InvalidToken);
    }

    let user = state
        .store
        .find_user_by_id(&claims.sub)
        .await
        .map_err(StoreError::from_debug)?
        .ok_or(ResetError::InvalidToken)?;

    if fingerprint(state.secret(), user.password_hash()) != claims.fingerprint {
        return Err(ResetError::InvalidToken);
    }

    Ok(user)
}

pub async fn complete<U: AuthUser>(state: &AuthState<U>, token: &str, new_password: &str) -> Result<U, ResetError> {
    let user = verify_token(state, token).await?;
    let hash = password::hash(new_password).map_err(ResetError::Hash)?;

    Ok(state.store.reset_password(user, hash).await.map_err(StoreError::from_debug)?)
}
