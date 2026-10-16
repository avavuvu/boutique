use axum::http::{HeaderMap, header};
use maud::Markup;

use crate::{
    error::ErrorReport,
    html,
    views::{Head, base},
};

pub fn back_url(headers: &HeaderMap) -> &str {
    headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("/")
}

pub fn error_content(report: &ErrorReport, back: &str) -> Markup {
    html! {
        section.error-page {
            h1 { (title(report)) }
            @if !report.messages.is_empty() {
                ul {
                    @for (_, message) in &report.messages {
                        li { (message) }
                    }
                }
            }
            a href=(back) { "Back" }
        }
    }
}

pub async fn error_page(headers: HeaderMap, report: ErrorReport) -> Markup {
    base(&Head::new(title(&report)), error_content(&report, back_url(&headers)))
}

fn title(report: &ErrorReport) -> String {
    let code = report.status.as_u16();
    match report.status.canonical_reason() {
        Some(reason) => format!("{code} {reason}"),
        None => code.to_string(),
    }
}
