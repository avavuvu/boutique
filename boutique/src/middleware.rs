use axum::{
    extract::State,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::{cookies, state::AuthState, store::AuthUser};

pub async fn clear_legacy_cookies<U: AuthUser>(
    State(state): State<AuthState<U>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let jar = CookieJar::from_headers(request.headers());
    let stale: Vec<&str> = cookies::LEGACY.into_iter().filter(|name| jar.get(name).is_some()).collect();

    let response = next.run(request).await;
    if stale.is_empty() {
        return response;
    }

    let jar = stale.into_iter().fold(jar, |jar, name| jar.remove(cookies::remove(name, &state.config)));
    (jar, response).into_response()
}
