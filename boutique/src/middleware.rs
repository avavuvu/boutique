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

#[cfg(test)]
mod tests {
    use super::is_cross_origin;
    use axum::http::Request;

    fn request(method: &str, headers: &[(&str, &str)]) -> Request<()> {
        let mut builder = Request::builder().method(method).uri("/").header("host", "app.room.lc");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(()).unwrap()
    }

    #[test]
    fn safe_methods_pass() {
        assert!(!is_cross_origin(&request("GET", &[("sec-fetch-site", "cross-site")])));
    }

    #[test]
    fn fetch_metadata_decides_when_present() {
        assert!(!is_cross_origin(&request("POST", &[("sec-fetch-site", "same-origin")])));
        assert!(!is_cross_origin(&request("POST", &[("sec-fetch-site", "none")])));
        assert!(is_cross_origin(&request("POST", &[("sec-fetch-site", "same-site")])));
        assert!(is_cross_origin(&request("PUT", &[("sec-fetch-site", "cross-site"), ("origin", "https://app.room.lc")])));
    }

    #[test]
    fn origin_must_match_the_host_without_fetch_metadata() {
        assert!(!is_cross_origin(&request("POST", &[("origin", "https://app.room.lc")])));
        assert!(is_cross_origin(&request("POST", &[("origin", "https://blog.room.lc")])));
        assert!(is_cross_origin(&request("DELETE", &[("origin", "null")])));
    }

    #[test]
    fn requests_without_browser_headers_pass() {
        assert!(!is_cross_origin(&request("POST", &[])));
    }
}
