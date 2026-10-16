use std::{error::Error, fmt, time::Duration};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum::response::{IntoResponseParts, ResponseParts};
pub use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use sha2::{Digest, Sha256};

use super::{
    AuthState, cookies, expiry, password,
    store::{AuthStore, AuthUser, NewSession, StoreError},
};
use crate::error::AppError;

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    Store(StoreError),
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidCredentials => f.write_str("invalid credentials"),
            LoginError::Store(_) => f.write_str("login failed"),
        }
    }
}

impl Error for LoginError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoginError::InvalidCredentials => None,
            LoginError::Store(error) => Some(error),
        }
    }
}

impl From<StoreError> for LoginError {
    fn from(e: StoreError) -> Self {
        LoginError::Store(e)
    }
}

impl From<LoginError> for AppError {
    fn from(error: LoginError) -> Self {
        match error {
            LoginError::InvalidCredentials => AppError::message("Incorrect email or password"),
            LoginError::Store(error) => error.into(),
        }
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
    let user = state.store.find_user_by_email(email).await.map_err(StoreError::new)?;

    match user {
        Some(user) if password::verify(plain_password, user.password_hash()) => Ok(user),
        _ => Err(LoginError::InvalidCredentials),
    }
}

pub async fn issue<U: AuthUser>(state: &AuthState<U>, user: &U) -> Result<Session, StoreError> {
    let now = Utc::now();
    let token = new_token();
    let ttl = state.config.session_ttl;
    let session = NewSession {
        token_hash: token_hash(&token),
        user_id: user.id().to_string(),
        created_at: now,
        expires_at: expiry(now, ttl),
    };
    state.store.create_session(session).await.map_err(StoreError::new)?;

    Ok(Session(CookieJar::new().add(cookies::make(
        cookies::SESSION,
        token,
        ttl,
        &state.config,
    ))))
}

pub async fn revoke<U: AuthUser>(state: &AuthState<U>, jar: CookieJar) -> Session {
    if let Some(cookie) = jar.get(cookies::SESSION) {
        let token_hash = token_hash(cookie.value());
        let _ = state.store.delete_session(&token_hash).await;
    }

    Session(jar.remove(cookies::remove(cookies::SESSION, &state.config)))
}

pub async fn revoke_all<U: AuthUser>(state: &AuthState<U>, user_id: &str) -> Result<(), StoreError> {
    state.store.delete_user_sessions(user_id).await.map_err(StoreError::new)
}

pub fn spawn_cleanup<U: AuthUser>(state: AuthState<U>, interval: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(error) = state.store.delete_expired_sessions(Utc::now()).await {
                tracing::warn!(?error, "failed to delete expired sessions");
            }
        }
    })
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
