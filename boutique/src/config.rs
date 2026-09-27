#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub login_url: String,
    pub cookie_domain: Option<String>,
    pub secure_cookies: bool,
    pub jwt_ttl_hours: i64,
    pub refresh_ttl_hours: i64,
    pub reset_ttl_hours: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            login_url: "/login".into(),
            cookie_domain: None,
            secure_cookies: !cfg!(debug_assertions),
            jwt_ttl_hours: 1,
            refresh_ttl_hours: 30 * 24,
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

    pub fn jwt_ttl_hours(mut self, hours: i64) -> Self {
        self.jwt_ttl_hours = hours;
        self
    }

    pub fn refresh_ttl_hours(mut self, hours: i64) -> Self {
        self.refresh_ttl_hours = hours;
        self
    }

    pub fn reset_ttl_hours(mut self, hours: i64) -> Self {
        self.reset_ttl_hours = hours;
        self
    }
}
