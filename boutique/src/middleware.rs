use axum::{
    body::Body,
    handler::Handler,
    http::{HeaderMap, Method, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{error::ErrorReport, htmx};

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

pub(crate) async fn error_page<H, T, S>(render: H, state: S, request: Request<Body>, next: Next) -> Response
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    if !wants_page(request.headers()) {
        return next.run(request).await;
    }

    let mut page_request = Request::new(Body::empty());
    *page_request.method_mut() = request.method().clone();
    *page_request.uri_mut() = request.uri().clone();
    *page_request.version_mut() = request.version();
    *page_request.headers_mut() = request.headers().clone();

    let response = next.run(request).await;
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }

    let report = match response.extensions().get::<ErrorReport>() {
        Some(report) => report.clone(),
        None if is_html(response.headers()) => return response,
        None => ErrorReport::new(status),
    };
    page_request.extensions_mut().insert(report);

    let (page, body) = render.call(page_request, state).await.into_parts();
    let (mut parts, _) = response.into_parts();
    parts.headers.remove(header::CONTENT_TYPE);
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.extend(page.headers);
    Response::from_parts(parts, body)
}

fn wants_page(headers: &HeaderMap) -> bool {
    !htmx::is_htmx(headers)
        && headers.get(header::ACCEPT).and_then(|value| value.to_str().ok()).is_some_and(|value| value.contains("text/html"))
}

fn is_html(headers: &HeaderMap) -> bool {
    headers.get(header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).is_some_and(|value| value.starts_with("text/html"))
}
