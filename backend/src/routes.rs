use axum::Json;
use axum::extract::Path;
use axum::routing::patch;
use axum::{Router, extract::State, http::StatusCode, routing::get};
use tracing::error;

use crate::state::AppState;
use crate::{models::*, repository};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/courses", get(list_courses).post(create_course))
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}", patch(update_todo))
        .route("/todos/{id}/archive", patch(archive_todo))
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

async fn list_courses(State(state): State<AppState>) -> Result<Json<Vec<Course>>, StatusCode> {
    repository::fetch_courses(&state.db)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn create_course(State(state): State<AppState>, Json(req): Json<NewCourseRequest>) -> Result<Json<Course>, StatusCode> {
    repository::insert_course(&state.db, req)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn list_todos(State(state): State<AppState>) -> Result<Json<Vec<Todo>>, StatusCode> {
    repository::fetch_todos(&state.db)
        .await
        .map(Json)
        .map_err(internal_error)

}

async fn create_todo(State(state): State<AppState>, Json(req): Json<NewTodoRequest>) -> Result<Json<Todo>, StatusCode> {
    repository::insert_todo(&state.db, req)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn update_todo(State(state): State<AppState>, Path(id): Path<String>, Json(req): Json<UpdateTodoRequest>) -> Result<Json<Todo>, StatusCode> {
    match repository::update_todo(&state.db, &id, req)
        .await
        .map_err(internal_error)?
    {
        Some(todo) => Ok(Json(todo)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn archive_todo(State(state): State<AppState>, Path(id): Path<String>) -> Result<StatusCode, StatusCode> {
    let ok = repository::archive_todo(&state.db, &id)
        .await
        .map_err(internal_error)?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn internal_error(err: impl std::fmt::Display) -> StatusCode {
    error!("internal error: {}", err);
    StatusCode::INTERNAL_SERVER_ERROR
}