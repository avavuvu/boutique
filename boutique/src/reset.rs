use sea_orm::DbErr;

use crate::{
    jwt, password,
    state::AuthState,
    store::{self, AuthUser},
};

#[derive(Debug)]
pub enum ResetError {
    InvalidToken,
    Database(DbErr),
    Hash(argon2::password_hash::Error),
}

impl From<DbErr> for ResetError {
    fn from(e: DbErr) -> Self {
        ResetError::Database(e)
    }
}

/// returns `None` for an unknown email. callers must show the same message
/// either way so the endpoint does not reveal which emails have accounts.
pub async fn request<U: AuthUser>(state: &AuthState<U>, email: &str) -> Result<Option<(U, String)>, ResetError> {
    let Some(user) = store::find_by_email::<U>(&state.db, email).await? else {
        return Ok(None);
    };

    let token = token_for(state, &user).map_err(|_| ResetError::InvalidToken)?;
    Ok(Some((user, token)))
}

pub fn token_for<U: AuthUser>(state: &AuthState<U>, user: &U) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = jwt::PasswordResetClaims::new(user.id(), user.password_hash(), state.config.reset_ttl_hours);
    jwt::generate_password_reset(state.secret(), &claims)
}

pub async fn verify_token<U: AuthUser>(state: &AuthState<U>, token: &str) -> Result<U, ResetError> {
    let claims = jwt::validate_password_reset(state.secret(), token).map_err(|_| ResetError::InvalidToken)?;

    let user = store::find_by_id::<U>(&state.db, claims.user_id())
        .await?
        .ok_or(ResetError::InvalidToken)?;

    if jwt::hash_fragment(user.password_hash()) != claims.password_hash_fragment {
        return Err(ResetError::InvalidToken);
    }

    Ok(user)
}

pub async fn complete<U: AuthUser>(state: &AuthState<U>, token: &str, new_password: &str) -> Result<U, ResetError> {
    let user = verify_token(state, token).await?;
    let hash = password::hash(new_password).map_err(ResetError::Hash)?;
    Ok(store::set_password_hash::<U>(&state.db, user, hash).await?)
}
