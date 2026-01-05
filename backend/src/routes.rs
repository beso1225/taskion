use axum::{Router, extract::State, http::StatusCode, routing::get};
use tracing::error;

use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> StatusCode {
    match sqlx::query("select 1").execute(&state.db).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            error!("health check failed: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn list_courses() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn create_course() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn list_todos() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn create_todo() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn update_todo() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn archive_todo() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}