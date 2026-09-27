pub mod fragments;

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use maud::{Markup, html};

pub fn is_htmx(headers: &HeaderMap) -> bool {
    headers.contains_key("hx-request")
}

pub fn redirect(path: &str) -> Response {
    (
        StatusCode::OK,
        [("HX-Redirect", HeaderValue::from_str(path).expect("invalid redirect path"))],
    )
        .into_response()
}

pub fn partial(target: &str, content: Markup) -> Markup {
    html! {
        hx-partial hx-target=(target) hx-swap="outerHTML" { (content) }
    }
}

pub fn oob_only(markup: Markup) -> Response {
    markup.into_response()
}
