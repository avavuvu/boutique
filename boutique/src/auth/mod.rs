mod cookies;
mod extract;
pub mod password;
pub mod reset;
#[cfg(feature = "sea-orm")]
pub mod sea_orm;
pub mod session;
mod store;

pub use extract::{AuthRejection, AuthenticatedUser};
pub use store::{AuthStore, AuthUser, NewSession, StoreError};

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub login_url: String,
    pub cookie_domain: Option<String>,
    pub secure_cookies: bool,
    pub session_ttl_hours: i64,
    pub reset_ttl_hours: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            login_url: "/login".into(),
            cookie_domain: None,
            secure_cookies: true,
            session_ttl_hours: 30 * 24,
            reset_ttl_hours: 1,
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

    pub fn session_ttl_hours(mut self, hours: i64) -> Self {
        self.session_ttl_hours = hours;
        self
    }

    pub fn reset_ttl_hours(mut self, hours: i64) -> Self {
        self.reset_ttl_hours = hours;
        self
    }
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
