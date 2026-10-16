use axum_extra::extract::cookie::{Cookie, SameSite};
use std::time::Duration;

use super::AuthConfig;

pub(crate) const SESSION: &str = "session";

fn with_domain(mut cookie: Cookie<'static>, config: &AuthConfig) -> Cookie<'static> {
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    cookie
}

pub(crate) fn make(key: &str, value: String, ttl: Duration, config: &AuthConfig) -> Cookie<'static> {
    let cookie = Cookie::build((key.to_string(), value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::try_from(ttl).unwrap_or(time::Duration::MAX))
        .secure(config.secure_cookies)
        .build();

    with_domain(cookie, config)
}

pub(crate) fn remove(key: &str, config: &AuthConfig) -> Cookie<'static> {
    let cookie = Cookie::build((key.to_string(), String::new())).path("/").build();
    with_domain(cookie, config)
}
