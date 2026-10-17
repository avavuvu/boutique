use boutique::{
    AppError, AppResult, AuthenticatedUser, CookieJar,
    axum::{
        Form,
        extract::State,
        response::{IntoResponse, Redirect, Response},
    },
    htmx, session,
};
use sea_orm::{ActiveModelTrait, SqlErr};
use serde::Deserialize;
use validator::Validate;

use crate::{models::user, state::AppState, views};

#[derive(Deserialize, Validate)]
pub struct SignUpForm {
    #[validate(email(message = "Enter a valid email address"))]
    email: String,
    #[validate(length(min = 8, message = "Use at least 8 characters"))]
    password: String,
}

#[derive(Deserialize, Validate)]
pub struct SignInForm {
    #[validate(email(message = "Enter a valid email address"))]
    email: String,
    #[validate(length(min = 1, message = "Enter your password"))]
    password: String,
}

pub async fn sign_up_page(user: Option<AuthenticatedUser<user::Model>>) -> Response {
    match user {
        Some(_) => Redirect::to("/").into_response(),
        None => views::auth::sign_up().into_response(),
    }
}

pub async fn sign_up(State(state): State<AppState>, Form(form): Form<SignUpForm>) -> AppResult {
    form.validate()?;

    let email = form.email.trim().to_lowercase();
    let new_user = user::new(&email, &form.password).await.map_err(|error| AppError::internal("password hash", error))?;
    let user = new_user.insert(&state.db).await.map_err(|error| match error.sql_err() {
        Some(SqlErr::UniqueConstraintViolation(_)) => AppError::field("email", "An account with this email already exists"),
        _ => error.into(),
    })?;

    let session = session::issue(&state.auth, &user).await?;
    Ok((session, htmx::redirect("/")).into_response())
}

pub async fn sign_in_page(user: Option<AuthenticatedUser<user::Model>>) -> Response {
    match user {
        Some(_) => Redirect::to("/").into_response(),
        None => views::auth::sign_in().into_response(),
    }
}

pub async fn sign_in(State(state): State<AppState>, Form(form): Form<SignInForm>) -> AppResult {
    form.validate()?;

    let (_, session) = session::login(&state.auth, &form.email.trim().to_lowercase(), &form.password).await?;
    Ok((session, htmx::redirect("/")).into_response())
}

pub async fn sign_out(State(state): State<AppState>, jar: CookieJar) -> Response {
    (session::revoke(&state.auth, jar).await, Redirect::to("/")).into_response()
}
