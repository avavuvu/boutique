mod cookies;
mod extract;
pub mod password;
pub mod reset;
#[cfg(feature = "sea-orm")]
pub mod sea_orm;
pub mod session;
mod store;
#[cfg(test)]
mod testing;

use std::time::Duration;

use chrono::{DateTime, Utc};

pub use extract::{AuthRejection, AuthenticatedUser};
pub use store::{AuthStore, AuthUser, NewSession, StoreError};

const HOUR: Duration = Duration::from_secs(60 * 60);

#[must_use]
#[derive(Clone, Debug)]
pub struct AuthConfig {
    login_url: String,
    cookie_domain: Option<String>,
    secure_cookies: bool,
    session_ttl: Duration,
    reset_ttl: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            login_url: "/login".into(),
            cookie_domain: None,
            secure_cookies: true,
            session_ttl: 30 * 24 * HOUR,
            reset_ttl: HOUR,
        }
    }
}

impl AuthConfig {
    pub fn login_url(mut self, url: impl Into<String>) -> Self {
        self.login_url = url.into();
        self
    }

    /// only needed when one login must span several subdomains, e.g. `.example.com`
    pub fn cookie_domain(mut self, domain: impl Into<String>) -> Self {
        self.cookie_domain = Some(domain.into());
        self
    }

    pub fn secure_cookies(mut self, secure: bool) -> Self {
        self.secure_cookies = secure;
        self
    }

    pub fn session_ttl(mut self, ttl: Duration) -> Self {
        self.session_ttl = ttl;
        self
    }

    pub fn reset_ttl(mut self, ttl: Duration) -> Self {
        self.reset_ttl = ttl;
        self
    }

    pub fn uses_secure_cookies(&self) -> bool {
        self.secure_cookies
    }
}

fn expiry(from: DateTime<Utc>, ttl: Duration) -> DateTime<Utc> {
    chrono::Duration::from_std(ttl)
        .ok()
        .and_then(|ttl| from.checked_add_signed(ttl))
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
}

#[derive(Clone)]
pub struct AuthState<U: AuthUser> {
    store: U::Store,
    secret: String,
    config: AuthConfig,
}

impl<U: AuthUser> AuthState<U> {
    pub fn new(store: U::Store, secret: impl Into<String>) -> Self {
        Self {
            store,
            secret: secret.into(),
            config: AuthConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AuthConfig) -> Self {
        self.config = config;
        self
    }

    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    fn secret(&self) -> &[u8] {
        self.secret.as_bytes()
    }
}
