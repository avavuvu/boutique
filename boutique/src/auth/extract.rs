use std::{error::Error, fmt};

use axum::{
    extract::{FromRequestParts, OptionalFromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;

use super::{
    AuthState, cookies, session,
    store::{AuthStore, AuthUser, StoreError},
};
use crate::{error::AppError, htmx};

pub struct AuthenticatedUser<U: AuthUser>(pub U);

pub trait HasAuth: Send + Sync {
    type Store: AuthStore;

    fn auth(&self) -> &AuthState<Self::Store>;
}

impl<S: AuthStore> HasAuth for AuthState<S> {
    type Store = S;

    fn auth(&self) -> &AuthState<S> {
        self
    }
}

#[derive(Debug)]
pub enum AuthRejection {
    Unauthenticated { login_url: String, htmx: bool },
    Store(StoreError),
}

impl fmt::Display for AuthRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthRejection::Unauthenticated { .. } => f.write_str("not logged in"),
            AuthRejection::Store(_) => f.write_str("could not load the session"),
        }
    }
}

impl Error for AuthRejection {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AuthRejection::Unauthenticated { .. } => None,
            AuthRejection::Store(error) => Some(error),
        }
    }
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        match self {
            AuthRejection::Unauthenticated { login_url, htmx: true } => htmx::redirect(&login_url),
            AuthRejection::Unauthenticated { login_url, htmx: false } => Redirect::to(&login_url).into_response(),
            AuthRejection::Store(error) => AppError::from(error).into_response(),
        }
    }
}

impl From<AuthRejection> for Response {
    fn from(rejection: AuthRejection) -> Self {
        rejection.into_response()
    }
}

impl From<AuthRejection> for AppError {
    fn from(rejection: AuthRejection) -> Self {
        match rejection {
            AuthRejection::Unauthenticated { .. } => AppError::Status(StatusCode::UNAUTHORIZED),
            AuthRejection::Store(error) => error.into(),
        }
    }
}

#[derive(Clone)]
struct Resolved<U>(Option<U>);

async fn resolve<S: AuthStore>(parts: &mut Parts, auth: &AuthState<S>) -> Result<Option<S::User>, AuthRejection> {
    if let Some(Resolved(user)) = parts.extensions.get::<Resolved<S::User>>() {
        return Ok(user.clone());
    }

    let token_hash = CookieJar::from_headers(&parts.headers)
        .get(cookies::SESSION)
        .map(|cookie| session::token_hash(cookie.value()));
    let user = match token_hash {
        Some(token_hash) => auth
            .store
            .find_session_user(&token_hash, Utc::now())
            .await
            .map_err(|error| AuthRejection::Store(StoreError::new(error)))?,
        None => None,
    };

    parts.extensions.insert(Resolved(user.clone()));
    Ok(user)
}

impl<St, U> FromRequestParts<St> for AuthenticatedUser<U>
where
    St: HasAuth,
    St::Store: AuthStore<User = U>,
    U: AuthUser,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &St) -> Result<Self, Self::Rejection> {
        let auth = state.auth();
        resolve(parts, auth)
            .await?
            .map(AuthenticatedUser)
            .ok_or_else(|| AuthRejection::Unauthenticated {
                login_url: auth.config.login_url.clone(),
                htmx: htmx::is_htmx(&parts.headers),
            })
    }
}

impl<St, U> OptionalFromRequestParts<St> for AuthenticatedUser<U>
where
    St: HasAuth,
    St::Store: AuthStore<User = U>,
    U: AuthUser,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &St) -> Result<Option<Self>, Self::Rejection> {
        Ok(resolve(parts, state.auth()).await?.map(AuthenticatedUser))
    }
}
