use axum::{
    body::{Body, HttpBody, to_bytes},
    handler::Handler,
    http::{HeaderMap, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    error::{ErrorMessage, ErrorReport},
    htmx, views,
};

const PLAIN_TEXT_LIMIT: usize = 1024;

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
    if !is_error(status) {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let report = match parts.extensions.get::<ErrorReport>() {
        Some(report) => report.clone(),
        None if is_html(&parts.headers) => return Response::from_parts(parts, body),
        None => plain_report(status, &parts.headers, body).await,
    };
    let headers = page_request.headers().clone();
    let uri = page_request.uri().clone();
    page_request.extensions_mut().insert(report.clone());

    let mut page = render.call(page_request, state).await;
    if is_error(page.status()) {
        tracing::warn!(%uri, %status, page_status = %page.status(), "error page handler failed, using the default error page");
        page = views::error_page(headers, report).await.into_response();
    }

    let (page, body) = page.into_parts();
    parts.headers.remove(header::CONTENT_TYPE);
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.extend(page.headers);
    Response::from_parts(parts, body)
}

async fn plain_report(status: StatusCode, headers: &HeaderMap, body: Body) -> ErrorReport {
    let mut report = ErrorReport::new(status);
    let short = body.size_hint().upper().is_some_and(|length| length <= PLAIN_TEXT_LIMIT as u64);
    if status.is_client_error()
        && short
        && is_plain_text(headers)
        && let Ok(bytes) = to_bytes(body, PLAIN_TEXT_LIMIT).await
        && let Ok(text) = std::str::from_utf8(&bytes)
        && !text.trim().is_empty()
    {
        report.messages.push(ErrorMessage::new(text.trim()));
    }
    report
}

fn is_error(status: StatusCode) -> bool {
    status.is_client_error() || status.is_server_error()
}

fn wants_page(headers: &HeaderMap) -> bool {
    !htmx::is_htmx(headers)
        && headers
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("text/html"))
}

fn is_html(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"))
}

fn is_plain_text(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/plain"))
}
