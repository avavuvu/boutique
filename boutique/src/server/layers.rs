use crate::headers::{SEC_FETCH_SITE, X_FORWARDED_PROTO, header_str};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_http::compression::{
    CompressionLayer, Predicate,
    predicate::{DefaultPredicate, NotForContentType},
};

pub(crate) fn compression() -> CompressionLayer<impl Predicate> {
    CompressionLayer::new().compress_when(
        DefaultPredicate::new()
            .and(NotForContentType::const_new("video/"))
            .and(NotForContentType::const_new("audio/"))
            .and(NotForContentType::const_new("font/woff")),
    )
}

pub(crate) async fn cross_origin_protection(request: Request<Body>, next: Next) -> Response {
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
    if let Some(site) = headers.get(SEC_FETCH_SITE) {
        return !matches!(site.as_bytes(), b"same-origin" | b"none");
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        return false;
    };
    let authority = origin
        .to_str()
        .ok()
        .and_then(|origin| origin.split_once("://"))
        .map(|(_, authority)| authority);
    authority.is_none() || authority != header_str(headers, header::HOST)
}

pub(crate) async fn https_redirect(request: Request<Body>, next: Next) -> Response {
    let headers = request.headers();
    if header_str(headers, X_FORWARDED_PROTO) == Some("http") {
        let host = header_str(headers, header::HOST).unwrap_or("");
        let path = request.uri().path_and_query().map_or("/", |path| path.as_str());
        return Redirect::permanent(&format!("https://{host}{path}")).into_response();
    }

    next.run(request).await
}
