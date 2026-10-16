use axum::{
    body::{Body, HttpBody, to_bytes},
    handler::Handler,
    http::{HeaderMap, Request, StatusCode, header, response::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    error::{ErrorMessage, ErrorReport},
    headers::header_str,
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

    let page_request = copy_head(&request);
    let response = next.run(request).await;
    if !is_error(response.status()) {
        return response;
    }

    let (parts, body) = response.into_parts();
    let report = match parts.extensions.get::<ErrorReport>() {
        Some(report) => report.clone(),
        None if is_html(&parts.headers) => return Response::from_parts(parts, body),
        None => plain_report(parts.status, &parts.headers, body).await,
    };

    let page = render_page(render, state, page_request, report).await;
    replace_body(parts, page)
}

fn copy_head(request: &Request<Body>) -> Request<Body> {
    let mut copy = Request::new(Body::empty());
    *copy.method_mut() = request.method().clone();
    *copy.uri_mut() = request.uri().clone();
    *copy.version_mut() = request.version();
    *copy.headers_mut() = request.headers().clone();
    copy
}

async fn render_page<H, T, S>(render: H, state: S, mut request: Request<Body>, report: ErrorReport) -> Response
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    let headers = request.headers().clone();
    let uri = request.uri().clone();
    request.extensions_mut().insert(report.clone());

    let page = render.call(request, state).await;
    if !is_error(page.status()) {
        return page;
    }
    tracing::warn!(%uri, status = %report.status, page_status = %page.status(), "error page handler failed, using the default error page");
    views::error_page(headers, report).await.into_response()
}

fn replace_body(mut parts: Parts, page: Response) -> Response {
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
    !htmx::is_htmx(headers) && header_str(headers, header::ACCEPT).is_some_and(|value| value.contains("text/html"))
}

fn is_html(headers: &HeaderMap) -> bool {
    header_str(headers, header::CONTENT_TYPE).is_some_and(|value| value.starts_with("text/html"))
}

fn is_plain_text(headers: &HeaderMap) -> bool {
    header_str(headers, header::CONTENT_TYPE).is_some_and(|value| value.starts_with("text/plain"))
}
