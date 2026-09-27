use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    context::UserContext,
    models::user,
    state::AuthState,
    store::{self, AuthUser},
};

pub struct AuthenticatedUser<U: AuthUser = user::Model>(pub U);

impl<St, U> FromRequestParts<St> for AuthenticatedUser<U>
where
    St: Send + Sync,
    U: AuthUser,
    AuthState<U>: FromRef<St>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &St) -> Result<Self, Self::Rejection> {
        let auth = AuthState::<U>::from_ref(state);
        let to_login = || Redirect::to(&auth.config.login_url).into_response();

        let user_id = parts
            .extensions
            .get::<UserContext>()
            .and_then(|ctx| ctx.user_id.clone())
            .ok_or_else(to_login)?;

        let user = store::find_by_id::<U>(&auth.db, &user_id)
            .await
            .map_err(|_| to_login())?
            .ok_or_else(to_login)?;

        Ok(AuthenticatedUser(user))
    }
}
