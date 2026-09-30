use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::response::{IntoResponseParts, ResponseParts};
pub use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    cookies, password,
    state::AuthState,
    store::{AuthStore, AuthUser, NewSession, StoreError},
};

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    Store(StoreError),
}

impl From<StoreError> for LoginError {
    fn from(e: StoreError) -> Self {
        LoginError::Store(e)
    }
}

/// the set-cookie headers for a session. return it alongside a response body.
pub struct Session(CookieJar);

impl IntoResponseParts for Session {
    type Error = <CookieJar as IntoResponseParts>::Error;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        self.0.into_response_parts(res)
    }
}

pub async fn login<U: AuthUser>(state: &AuthState<U>, email: &str, plain_password: &str) -> Result<(U, Session), LoginError> {
    let user = authenticate(state, email, plain_password).await?;
    let session = issue(state, &user).await?;
    Ok((user, session))
}

pub async fn authenticate<U: AuthUser>(state: &AuthState<U>, email: &str, plain_password: &str) -> Result<U, LoginError> {
    let user = state.store.find_user_by_email(email).await.map_err(StoreError::from_debug)?;

    match user {
        Some(user) if password::verify(plain_password, user.password_hash()) => Ok(user),
        _ => Err(LoginError::InvalidCredentials),
    }
}

pub async fn issue<U: AuthUser>(state: &AuthState<U>, user: &U) -> Result<Session, StoreError> {
    let now = Utc::now();
    state.store.delete_expired_sessions(now).await.map_err(StoreError::from_debug)?;

    let token = new_token();
    let ttl = state.config.session_ttl_hours;
    let session = NewSession {
        token_hash: hash(&token),
        user_id: user.id().to_string(),
        created_at: now,
        expires_at: now + chrono::Duration::hours(ttl),
    };
    state.store.create_session(session).await.map_err(StoreError::from_debug)?;

    Ok(Session(CookieJar::new().add(cookies::make(cookies::SESSION, token, ttl, &state.config))))
}

pub async fn revoke<U: AuthUser>(state: &AuthState<U>, jar: CookieJar) -> Session {
    if let Some(cookie) = jar.get(cookies::SESSION) {
        let token_hash = hash(cookie.value());
        let _ = state.store.delete_session(&token_hash).await;
    }

    Session(jar.remove(cookies::remove(cookies::SESSION, &state.config)))
}

pub async fn revoke_all<U: AuthUser>(state: &AuthState<U>, user_id: &str) -> Result<(), StoreError> {
    state.store.delete_user_sessions(user_id).await.map_err(StoreError::from_debug)
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
