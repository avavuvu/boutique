mod auth;
mod error;
mod server;

#[cfg(feature = "cloudinary")]
pub mod cloudinary;
pub mod forms;
pub mod htmx;
pub mod ids;
pub mod prelude;
pub mod views;

#[cfg(feature = "sea-orm")]
pub mod store {
    pub use crate::auth::sea_orm;
}

pub use argon2;
pub use axum;
pub use axum_extra;
pub use bq_components as components;
pub use bq_components::{Attrs, attrs, component, html};
pub use chrono;
pub use maud;
pub use uuid;
pub use validator;

pub use auth::{
    AuthConfig, AuthRejection, AuthState, AuthStore, AuthUser, AuthenticatedUser, NewSession, StoreError, password, reset, session,
};
pub use error::{AppError, AppResult, ErrorMessage, ErrorReport};
pub use server::{Cache, ManifestError, Server, run};
