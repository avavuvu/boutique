pub mod assets;
pub mod config;
pub mod context;
pub mod cookies;
pub mod extractors;
pub mod htmx;
pub mod jwt;
pub mod middleware;
pub mod models;
pub mod password;
pub mod reset;
pub mod run;
pub mod server;
pub mod session;
pub mod state;
pub mod store;
pub mod views;

#[cfg(feature = "migration")]
pub mod migrations;

pub use argon2;
pub use axum;
pub use axum_extra;
pub use chrono;
pub use maud;
pub use uuid;
pub use sea_orm;
pub use validator;

#[cfg(feature = "migration")]
pub use sea_orm_migration;

pub use config::AuthConfig;
pub use context::UserContext;
pub use extractors::AuthenticatedUser;
pub use run::run;
pub use server::Server;
pub use state::AuthState;
pub use store::AuthUser;
