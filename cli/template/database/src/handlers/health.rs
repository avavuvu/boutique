use boutique::{AppResult, axum::{extract::State, http::StatusCode}};

use crate::state::AppState;

pub async fn check(State(state): State<AppState>) -> AppResult<StatusCode> {
    state.db.ping().await?;
    Ok(StatusCode::OK)
}
