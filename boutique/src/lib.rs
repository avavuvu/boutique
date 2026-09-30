pub mod assets;
pub mod config;
pub mod cookies;
pub mod error;
pub mod extractors;
pub mod form;
pub mod htmx;
pub mod ids;
pub mod middleware;
pub mod password;
pub mod reset;
pub mod run;
pub mod server;
pub mod session;
pub mod state;
pub mod store;
pub mod views;

#[cfg(feature = "cloudinary")]
pub mod cloudinary;

pub use argon2;
pub use axum;
pub use axum_extra;
pub use chrono;
pub use maud;
pub use uuid;
pub use validator;

pub use config::AuthConfig;
pub use error::{AppError, AppResult};
pub use extractors::AuthenticatedUser;
pub use run::run;
pub use server::Server;
pub use state::AuthState;
pub use store::{AuthStore, AuthUser, NewSession, StoreError};
