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
    if let Some(site) = headers.get("sec-fetch-site") {
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
    authority.is_none() || authority != headers.get(header::HOST).and_then(|host| host.to_str().ok())
}

pub(crate) async fn https_redirect(req: Request<Body>, next: Next) -> Response {
    let is_http = req
        .headers()
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "http")
        .unwrap_or(false);

    if is_http {
        let host = req.headers().get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
        let path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
        return Redirect::permanent(&format!("https://{host}{path}")).into_response();
    }

    next.run(req).await
}
