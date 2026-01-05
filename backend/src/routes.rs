use axum::Json;
use axum::{Router, extract::State, http::StatusCode, routing::get};
use tracing::error;

use crate::state::AppState;
use crate::models::*;

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

async fn list_courses() -> Json<Vec<Course>> {
    Json(vec![])
}

async fn create_course(Json(_req): Json<NewCourseRequest>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn list_todos() -> Json<Vec<Todo>> {
    Json(vec![])
}

async fn create_todo(Json(_req): Json<NewTodoRequest>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn update_todo(Json(_req): Json<UpdateTodoRequest>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn archive_todo() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}