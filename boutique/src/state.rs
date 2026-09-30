use crate::{config::AuthConfig, store::AuthUser};

#[derive(Clone)]
pub struct AuthState<U: AuthUser> {
    pub store: U::Store,
    pub secret: String,
    pub config: AuthConfig,
}

impl<U: AuthUser> AuthState<U> {
    pub fn new(store: U::Store, secret: impl Into<String>) -> Self {
        Self { store, secret: secret.into(), config: AuthConfig::default() }
    }

    pub fn with_config(store: U::Store, secret: impl Into<String>, config: AuthConfig) -> Self {
        Self::new(store, secret).config(config)
    }

    pub fn config(mut self, config: AuthConfig) -> Self {
        self.config = config;
        self
    }

    pub fn secret(&self) -> &[u8] {
        self.secret.as_bytes()
    }
}
