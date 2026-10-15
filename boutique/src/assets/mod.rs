pub mod manifest;

use axum::{
    Router,
    http::header,
    response::IntoResponse,
    routing::get,
};

pub const HTMX_PATH: &str = "/boutique/htmx.js";
pub const BUILD_ROUTE: &str = "/build";
pub const BUILD_DIR: &str = "public/build";


fn script(body: &'static str) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        body,
    )
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route(HTMX_PATH, get(|| async { script(include_str!("../../assets/htmx.js")) }))
}
