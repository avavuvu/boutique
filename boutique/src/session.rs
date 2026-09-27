use axum::response::{IntoResponseParts, ResponseParts};
pub use axum_extra::extract::cookie::CookieJar;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DbErr, EntityTrait, QueryFilter};

use crate::{
    cookies, jwt,
    models::{refresh_token::{self, Entity as RefreshToken}, user},
    password,
    state::AuthState,
    store::{self, AuthUser},
};

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    Database(DbErr),
    Token(jsonwebtoken::errors::Error),
}

impl From<DbErr> for LoginError {
    fn from(e: DbErr) -> Self {
        LoginError::Database(e)
    }
}

#[derive(Debug)]
pub enum SessionError {
    Token(jsonwebtoken::errors::Error),
    Database(DbErr),
}

impl From<jsonwebtoken::errors::Error> for SessionError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        SessionError::Token(e)
    }
}

impl From<DbErr> for SessionError {
    fn from(e: DbErr) -> Self {
        SessionError::Database(e)
    }
}

impl From<SessionError> for LoginError {
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::Database(e) => LoginError::Database(e),
            SessionError::Token(e) => LoginError::Token(e),
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
    let user = store::find_by_email::<U>(&state.db, email).await?;

    match user {
        Some(user) if password::verify(plain_password, user.password_hash()) => Ok(user),
        _ => Err(LoginError::InvalidCredentials),
    }
}

/// an unsaved row for boutique's default `users` table
pub fn new_user(email: &str, plain_password: &str) -> Result<user::ActiveModel, argon2::password_hash::Error> {
    Ok(user::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        email: Set(email.to_string()),
        password: Set(password::hash(plain_password)?),
        created_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    })
}

pub async fn issue<U: AuthUser>(state: &AuthState<U>, user: &U) -> Result<Session, SessionError> {
    let claims = jwt::Claims::new(user.id(), user.email(), state.config.jwt_ttl_hours);
    let jwt_token = jwt::generate(state.secret(), &claims)?;

    let refresh_value = uuid::Uuid::new_v4().to_string();
    refresh_token::ActiveModel {
        id: Set(uuid::Uuid::new_v4().to_string()),
        user_id: Set(user.id().to_string()),
        token: Set(refresh_value.clone()),
        expires_at: Set((chrono::Utc::now() + chrono::Duration::hours(state.config.refresh_ttl_hours)).into()),
        created_at: Set(chrono::Utc::now().into()),
    }
    .insert(&state.db)
    .await?;

    Ok(Session(CookieJar::new()
        .add(cookies::make(cookies::JWT, jwt_token, state.config.jwt_ttl_hours, &state.config))
        .add(cookies::make(cookies::REFRESH, refresh_value, state.config.refresh_ttl_hours, &state.config))))
}

pub async fn revoke<U: AuthUser>(state: &AuthState<U>, jar: CookieJar) -> Session {
    if let Some(refresh) = jar.get(cookies::REFRESH) {
        let _ = RefreshToken::delete_many()
            .filter(refresh_token::Column::Token.eq(refresh.value()))
            .exec(&state.db)
            .await;
    }

    Session(jar
        .remove(cookies::remove(cookies::JWT, &state.config))
        .remove(cookies::remove(cookies::REFRESH, &state.config)))
}
