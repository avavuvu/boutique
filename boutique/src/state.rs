use std::marker::PhantomData;

use sea_orm::DatabaseConnection;

use crate::{config::AuthConfig, models::user, store::AuthUser};

#[derive(Clone)]
pub struct AuthState<U: AuthUser = user::Model> {
    pub db: DatabaseConnection,
    pub jwt_secret: String,
    pub config: AuthConfig,
    _user: PhantomData<U>,
}

impl<U: AuthUser> AuthState<U> {
    pub fn new(db: DatabaseConnection, jwt_secret: impl Into<String>) -> Self {
        Self { db, jwt_secret: jwt_secret.into(), config: AuthConfig::default(), _user: PhantomData }
    }

    pub fn with_config(db: DatabaseConnection, jwt_secret: impl Into<String>, config: AuthConfig) -> Self {
        Self::new(db, jwt_secret).config(config)
    }

    pub fn config(mut self, config: AuthConfig) -> Self {
        self.config = config;
        self
    }

    pub fn secret(&self) -> &[u8] {
        self.jwt_secret.as_bytes()
    }
}
