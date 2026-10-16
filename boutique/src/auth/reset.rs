use std::{error::Error, fmt};

use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::{
    AuthState,
    password::{self, HashError},
    store::{AuthStore, AuthUser, StoreError},
};
use crate::error::AppError;

const TOKEN_TYPE: &str = "password_reset";

#[derive(Debug)]
pub enum ResetError {
    InvalidToken,
    Store(StoreError),
    Hash(HashError),
}

impl fmt::Display for ResetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResetError::InvalidToken => f.write_str("invalid or expired password reset token"),
            ResetError::Store(_) => f.write_str("password reset failed"),
            ResetError::Hash(_) => f.write_str("password reset failed"),
        }
    }
}

impl Error for ResetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ResetError::InvalidToken => None,
            ResetError::Store(error) => Some(error),
            ResetError::Hash(error) => Some(error),
        }
    }
}

impl From<StoreError> for ResetError {
    fn from(e: StoreError) -> Self {
        ResetError::Store(e)
    }
}

impl From<ResetError> for AppError {
    fn from(error: ResetError) -> Self {
        match error {
            ResetError::InvalidToken => AppError::message("This link is invalid or has expired"),
            ResetError::Store(error) => error.into(),
            ResetError::Hash(error) => AppError::internal("password reset", error),
        }
    }
}

fn mac(secret: &[u8], user_id: &str, expires: i64, password_hash: &str) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac accepts keys of any length");
    mac.update(TOKEN_TYPE.as_bytes());
    mac.update(b"\0");
    mac.update(user_id.as_bytes());
    mac.update(b"\0");
    mac.update(expires.to_string().as_bytes());
    mac.update(b"\0");
    mac.update(password_hash.as_bytes());
    mac
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

/// returns `None` for an unknown email. callers must show the same message
/// either way so the endpoint does not reveal which emails have accounts.
pub async fn request<U: AuthUser>(state: &AuthState<U>, email: &str) -> Result<Option<(U, String)>, ResetError> {
    let Some(user) = state.store.find_user_by_email(email).await.map_err(StoreError::new)? else {
        return Ok(None);
    };

    let token = token_for(state, &user);
    Ok(Some((user, token)))
}

pub fn token_for<U: AuthUser>(state: &AuthState<U>, user: &U) -> String {
    let expires = (Utc::now() + chrono::Duration::hours(state.config.reset_ttl_hours)).timestamp();
    let signature = mac(state.secret(), user.id(), expires, user.password_hash())
        .finalize()
        .into_bytes();
    format!("{}.{expires}.{}", encode_hex(user.id().as_bytes()), encode_hex(&signature))
}

pub async fn verify_token<U: AuthUser>(state: &AuthState<U>, token: &str) -> Result<U, ResetError> {
    let mut parts = token.split('.');
    let (Some(user_id), Some(expires), Some(signature), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(ResetError::InvalidToken);
    };

    let user_id = decode_hex(user_id)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(ResetError::InvalidToken)?;
    let expires: i64 = expires.parse().map_err(|_| ResetError::InvalidToken)?;
    let signature = decode_hex(signature).ok_or(ResetError::InvalidToken)?;

    if expires <= Utc::now().timestamp() {
        return Err(ResetError::InvalidToken);
    }

    let user = state
        .store
        .find_user_by_id(&user_id)
        .await
        .map_err(StoreError::new)?
        .ok_or(ResetError::InvalidToken)?;

    mac(state.secret(), user.id(), expires, user.password_hash())
        .verify_slice(&signature)
        .map_err(|_| ResetError::InvalidToken)?;

    Ok(user)
}

pub async fn complete<U: AuthUser>(state: &AuthState<U>, token: &str, new_password: &str) -> Result<U, ResetError> {
    let user = verify_token(state, token).await?;
    let hash = password::hash(new_password).map_err(ResetError::Hash)?;

    Ok(state.store.reset_password(user, hash).await.map_err(StoreError::new)?)
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        pin::pin,
        sync::{Arc, Mutex},
        task::{Context, Poll, Waker},
    };

    use chrono::{DateTime, Utc};

    use super::*;
    use crate::auth::{AuthConfig, store::NewSession};

    #[derive(Clone)]
    struct User {
        id: String,
        email: String,
        password_hash: String,
    }

    #[derive(Clone)]
    struct Store(Arc<Mutex<Vec<User>>>);

    impl AuthUser for User {
        type Store = Store;

        fn id(&self) -> &str {
            &self.id
        }

        fn password_hash(&self) -> &str {
            &self.password_hash
        }
    }

    impl AuthStore<User> for Store {
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

    fn block_on<F: Future>(future: F) -> F::Output {
        match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test store futures are always ready"),
        }
    }

    fn user(id: &str) -> User {
        User {
            id: id.into(),
            email: format!("{id}@example.com"),
            password_hash: "hash".into(),
        }
    }

    fn state() -> AuthState<User> {
        AuthState::new(Store(Arc::new(Mutex::new(vec![user("alice"), user("bob")]))), "secret")
    }

    fn verify(state: &AuthState<User>, token: &str) -> Result<User, ResetError> {
        block_on(verify_token(state, token))
    }

    #[test]
    fn valid_token() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        assert_eq!(verify(&state, &token).unwrap().id, "alice");
    }

    #[test]
    fn expired_token() {
        let state = state().with_config(AuthConfig::default().reset_ttl_hours(-1));
        let token = token_for(&state, &user("alice"));
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn tampered_user_id() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        let (_, rest) = token.split_once('.').unwrap();
        let token = format!("{}.{rest}", encode_hex(b"bob"));
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn tampered_expiry() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        let mut parts = token.split('.');
        let (user_id, expires, signature) = (parts.next().unwrap(), parts.next().unwrap(), parts.next().unwrap());
        let later = expires.parse::<i64>().unwrap() + 3600;
        let token = format!("{user_id}.{later}.{signature}");
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn tampered_mac() {
        let state = state();
        let mut token = token_for(&state, &user("alice"));
        let last = if token.ends_with('0') { '1' } else { '0' };
        token.pop();
        token.push(last);
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }

    #[test]
    fn token_issued_before_password_change() {
        let state = state();
        let token = token_for(&state, &user("alice"));
        state.store.0.lock().unwrap()[0].password_hash = "changed".into();
        assert!(matches!(verify(&state, &token), Err(ResetError::InvalidToken)));
    }
}
