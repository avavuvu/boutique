use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub exp: usize,
}

impl Claims {
    pub fn new(user_id: &str, email: &str, ttl_hours: i64) -> Self {
        Self {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: (Utc::now() + chrono::Duration::hours(ttl_hours)).timestamp() as usize,
        }
    }

    pub fn user_id(&self) -> &str {
        &self.sub
    }
}

pub fn generate(secret: &[u8], claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
    encode(&Header::default(), claims, &EncodingKey::from_secret(secret))
}

pub fn validate(secret: &[u8], token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret), &Validation::default())
        .map(|data| data.claims)
}

/// the hash fragment changes when the password changes, so a reset token
/// stops validating after it has been used once.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PasswordResetClaims {
    pub sub: String,
    pub password_hash_fragment: String,
    pub exp: usize,
}

const HASH_FRAGMENT_LEN: usize = 16;

pub fn hash_fragment(password_hash: &str) -> String {
    password_hash.chars().rev().take(HASH_FRAGMENT_LEN).collect()
}

impl PasswordResetClaims {
    pub fn new(user_id: &str, password_hash: &str, ttl_hours: i64) -> Self {
        Self {
            sub: user_id.to_string(),
            password_hash_fragment: hash_fragment(password_hash),
            exp: (Utc::now() + chrono::Duration::hours(ttl_hours)).timestamp() as usize,
        }
    }

    pub fn user_id(&self) -> &str {
        &self.sub
    }
}

pub fn generate_password_reset(secret: &[u8], claims: &PasswordResetClaims) -> Result<String, jsonwebtoken::errors::Error> {
    encode(&Header::default(), claims, &EncodingKey::from_secret(secret))
}

pub fn validate_password_reset(secret: &[u8], token: &str) -> Result<PasswordResetClaims, jsonwebtoken::errors::Error> {
    decode::<PasswordResetClaims>(token, &DecodingKey::from_secret(secret), &Validation::default())
        .map(|data| data.claims)
}
