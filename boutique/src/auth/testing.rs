use std::{
    convert::Infallible,
    pin::pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use chrono::{DateTime, Utc};

use super::{
    AuthState,
    store::{AuthStore, AuthUser, NewSession},
};

#[derive(Clone)]
pub(crate) struct User {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) password_hash: String,
}

#[derive(Clone)]
pub(crate) struct Store(pub(crate) Arc<Mutex<Vec<User>>>);

impl AuthUser for User {
    fn id(&self) -> &str {
        &self.id
    }

    fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

impl AuthStore for Store {
    type User = User;
    type Error = Infallible;

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, Infallible> {
        Ok(self.0.lock().unwrap().iter().find(|user| user.email == email).cloned())
    }

    async fn find_user_by_id(&self, id: &str) -> Result<Option<User>, Infallible> {
        Ok(self.0.lock().unwrap().iter().find(|user| user.id == id).cloned())
    }

    async fn create_session(&self, _: NewSession) -> Result<(), Infallible> {
        Ok(())
    }

    async fn find_session_user(&self, _: &str, _: DateTime<Utc>) -> Result<Option<User>, Infallible> {
        Ok(None)
    }

    async fn delete_session(&self, _: &str) -> Result<(), Infallible> {
        Ok(())
    }

    async fn delete_user_sessions(&self, _: &str) -> Result<(), Infallible> {
        Ok(())
    }

    async fn delete_expired_sessions(&self, _: DateTime<Utc>) -> Result<(), Infallible> {
        Ok(())
    }

    async fn reset_password(&self, mut user: User, password_hash: String) -> Result<User, Infallible> {
        user.password_hash = password_hash;
        Ok(user)
    }
}

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test store futures are always ready"),
    }
}

pub(crate) fn user(id: &str) -> User {
    User {
        id: id.into(),
        email: format!("{id}@example.com"),
        password_hash: "hash".into(),
    }
}

pub(crate) fn state() -> AuthState<Store> {
    AuthState::new(Store(Arc::new(Mutex::new(vec![user("alice"), user("bob")]))), "secret")
}
