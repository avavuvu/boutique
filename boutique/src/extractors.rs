use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;

use crate::{
    cookies,
    error::AppError,
    session,
    state::AuthState,
    store::{AuthStore, AuthUser, StoreError},
};

pub struct AuthenticatedUser<U: AuthUser>(pub U);

#[derive(Clone)]
struct Resolved<U>(Option<U>);

async fn resolve<U: AuthUser>(parts: &mut Parts, auth: &AuthState<U>) -> Result<Option<U>, StoreError> {
    if let Some(Resolved(user)) = parts.extensions.get::<Resolved<U>>() {
        return Ok(user.clone());
    }

    let token_hash = CookieJar::from_headers(&parts.headers).get(cookies::SESSION).map(|cookie| session::hash(cookie.value()));
    let user = match token_hash {
        Some(token_hash) => auth.store.find_session_user(&token_hash, Utc::now()).await.map_err(StoreError::from_debug)?,
        None => None,
    };

    parts.extensions.insert(Resolved(user.clone()));
    Ok(user)
}

impl<St, U> FromRequestParts<St> for AuthenticatedUser<U>
where
    St: Send + Sync,
    U: AuthUser,
    AuthState<U>: FromRef<St>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &St) -> Result<Self, Self::Rejection> {
        let auth = AuthState::<U>::from_ref(state);

        match resolve(parts, &auth).await {
            Ok(Some(user)) => Ok(AuthenticatedUser(user)),
            Ok(None) => Err(Redirect::to(&auth.config.login_url).into_response()),
            Err(error) => Err(AppError::from(error).into_response()),
        }
    }
}

impl<St, U> OptionalFromRequestParts<St> for AuthenticatedUser<U>
where
    St: Send + Sync,
    U: AuthUser,
    AuthState<U>: FromRef<St>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &St) -> Result<Option<Self>, Self::Rejection> {
        let auth = AuthState::<U>::from_ref(state);
        Ok(resolve(parts, &auth).await?.map(AuthenticatedUser))
    }
}
