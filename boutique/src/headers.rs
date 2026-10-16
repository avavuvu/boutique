use axum::http::{HeaderMap, header::AsHeaderName};

pub(crate) const HX_REQUEST: &str = "hx-request";
pub(crate) const HX_REDIRECT: &str = "hx-redirect";
pub(crate) const SEC_FETCH_SITE: &str = "sec-fetch-site";
pub(crate) const X_FORWARDED_PROTO: &str = "x-forwarded-proto";

pub(crate) fn header_str<K: AsHeaderName>(headers: &HeaderMap, name: K) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}
