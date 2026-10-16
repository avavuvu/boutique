use std::{error::Error, fmt, future::Future};

use chrono::{DateTime, Utc};

pub trait AuthUser: Clone + Send + Sync + 'static {
    type Store: AuthStore<Self>;

    fn id(&self) -> &str;
    fn password_hash(&self) -> &str;
}

pub struct NewSession {
    pub token_hash: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub trait AuthStore<U>: Clone + Send + Sync + 'static {
    type Error: Error + Send + Sync + 'static;

    fn find_user_by_email(&self, email: &str) -> impl Future<Output = Result<Option<U>, Self::Error>> + Send;

    fn find_user_by_id(&self, id: &str) -> impl Future<Output = Result<Option<U>, Self::Error>> + Send;

    fn create_session(&self, session: NewSession) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn find_session_user(&self, token_hash: &str, now: DateTime<Utc>) -> impl Future<Output = Result<Option<U>, Self::Error>> + Send;

    fn delete_session(&self, token_hash: &str) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn delete_user_sessions(&self, user_id: &str) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn delete_expired_sessions(&self, now: DateTime<Utc>) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn reset_password(&self, user: U, password_hash: String) -> impl Future<Output = Result<U, Self::Error>> + Send;
}

#[derive(Debug)]
pub struct StoreError(Box<dyn Error + Send + Sync>);

impl StoreError {
    pub(crate) fn new(error: impl Error + Send + Sync + 'static) -> Self {
        StoreError(Box::new(error))
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the auth store failed")
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.0)
    }
}
