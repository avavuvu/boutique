use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration;

use crate::config::AuthConfig;

pub const SESSION: &str = "session";
pub(crate) const LEGACY: [&str; 2] = ["jwt", "refresh"];

fn with_domain(mut cookie: Cookie<'static>, config: &AuthConfig) -> Cookie<'static> {
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    cookie
}

pub fn make(key: &str, value: String, duration_hrs: i64, config: &AuthConfig) -> Cookie<'static> {
    let cookie = Cookie::build((key.to_string(), value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(Duration::hours(duration_hrs))
        .secure(config.secure_cookies)
        .build();

    with_domain(cookie, config)
}

pub fn remove(key: &str, config: &AuthConfig) -> Cookie<'static> {
    let cookie = Cookie::build((key.to_string(), String::new())).path("/").build();
    with_domain(cookie, config)
}
