use axum::{
    extract::{Extension, State},
    http::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    context::UserContext,
    cookies, jwt,
    models::refresh_token::{self, Entity as RefreshToken},
    state::AuthState,
    store::{self, AuthUser},
};

pub async fn base<U: AuthUser>(
    State(state): State<AuthState<U>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let mut jar = CookieJar::from_headers(request.headers());
    let mut context = UserContext::default();

    if let Some(jwt_cookie) = jar.get(cookies::JWT) {
        match jwt::validate(state.secret(), jwt_cookie.value()) {
            Ok(claims) => {
                context.user_id = Some(claims.sub);
                context.email = Some(claims.email);
            }
            Err(_) => {
                jar = jar
                    .remove(cookies::remove(cookies::JWT, &state.config))
                    .remove(cookies::remove(cookies::REFRESH, &state.config));
            }
        }
    } else if let Some(refresh_cookie) = jar.get(cookies::REFRESH) {
        let token_value = refresh_cookie.value().to_string();
        let record = RefreshToken::find()
            .filter(refresh_token::Column::Token.eq(&token_value))
            .filter(refresh_token::Column::ExpiresAt.gt(chrono::Utc::now()))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if let Some(record) = record {
            if let Ok(Some(user)) = store::find_by_id::<U>(&state.db, &record.user_id).await {
                let claims = jwt::Claims::new(user.id(), user.email(), state.config.jwt_ttl_hours);
                if let Ok(token) = jwt::generate(state.secret(), &claims) {
                    context.user_id = Some(user.id().to_string());
                    context.email = Some(user.email().to_string());
                    jar = jar.add(cookies::make(cookies::JWT, token, state.config.jwt_ttl_hours, &state.config));
                }
            }
        }
    }

    request.extensions_mut().insert(context);
    let response = next.run(request).await;
    (jar, response).into_response()
}

pub async fn required_auth<U: AuthUser>(
    State(state): State<AuthState<U>>,
    Extension(ctx): Extension<UserContext>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if !ctx.is_authenticated() {
        return Redirect::to(&state.config.login_url).into_response();
    }
    next.run(request).await
}
