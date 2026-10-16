use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use maud::Markup;

use crate::html;

pub fn is_htmx(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

pub fn redirect(path: &str) -> Response {
    let location = HeaderValue::from_str(path).unwrap_or_else(|error| {
        tracing::error!(path, %error, "invalid HX-Redirect path, redirecting to /");
        HeaderValue::from_static("/")
    });
    (StatusCode::OK, [("HX-Redirect", location)]).into_response()
}

pub fn partial(target: &str, content: Markup) -> Markup {
    html! {
        hx-partial hx-target=(target) hx-swap="outerHTML" { (content) }
    }
}
