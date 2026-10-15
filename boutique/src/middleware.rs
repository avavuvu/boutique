use axum::{
    extract::State,
    http::{Method, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;

use crate::{cookies, state::AuthState, store::AuthUser};

pub async fn cross_origin_protection(request: Request<axum::body::Body>, next: Next) -> Response {
    if is_cross_origin(&request) {
        return (StatusCode::FORBIDDEN, "Cross-origin request blocked").into_response();
    }
    next.run(request).await
}

fn is_cross_origin<B>(request: &Request<B>) -> bool {
    if matches!(*request.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return false;
    }
    let headers = request.headers();
    if let Some(site) = headers.get("sec-fetch-site") {
        return !matches!(site.as_bytes(), b"same-origin" | b"none");
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        return false;
    };
    let authority = origin.to_str().ok().and_then(|origin| origin.split_once("://")).map(|(_, authority)| authority);
    authority.is_none() || authority != headers.get(header::HOST).and_then(|host| host.to_str().ok())
}

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
