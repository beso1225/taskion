use axum::Json;
use axum::extract::{Path, Query};
use axum::routing::{patch, post};
use axum::{Router, extract::State, http::StatusCode, routing::get};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;
use crate::services::{SyncService, SyncStats};
use crate::models::*;
use crate::db::repository;

#[derive(Deserialize)]
struct TodoQueryParams {
    #[serde(default)]
    include_archived: bool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/courses", get(list_courses).post(create_course))
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}", patch(update_todo))
        .route("/todos/{id}/archive", patch(archive_todo))
        .route("/todos/{id}/unarchive", patch(unarchive_todo))
        .route("/sync", post(sync_now))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    sqlx::query("select 1").execute(&state.db).await?;
    Ok(StatusCode::OK)
}

async fn list_courses(State(state): State<AppState>) -> Result<Json<Vec<Course>>, AppError> {
    let courses = repository::fetch_courses(&state.db).await?;
    Ok(Json(courses))
}

async fn create_course(
    State(state): State<AppState>,
    Json(req): Json<NewCourseRequest>
) -> Result<Json<Course>, AppError> {
    let course = repository::insert_course(&state.db, req).await?;
    Ok(Json(course))
}

async fn list_todos(
    State(state): State<AppState>,
    Query(params): Query<TodoQueryParams>
) -> Result<Json<Vec<Todo>>, AppError> {
    let todos = if params.include_archived {
        repository::fetch_all_todos(&state.db).await?
    } else {
        repository::fetch_todos(&state.db).await?
    };
    Ok(Json(todos))
}

async fn create_todo(
    State(state): State<AppState>,
    Json(req): Json<NewTodoRequest>
) -> Result<Json<Todo>, AppError> {
    let todo = repository::insert_todo(&state.db, req).await?;
    Ok(Json(todo))
}

async fn update_todo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTodoRequest>
) -> Result<Json<Todo>, AppError> {
    let todo = repository::update_todo(&state.db, &id, req)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(todo))
}

async fn archive_todo(
    State(state): State<AppState>,
    Path(id): Path<String>
) -> Result<StatusCode, AppError> {
    let ok = repository::archive_todo(&state.db, &id).await?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

async fn unarchive_todo(
    State(state): State<AppState>,
    Path(id): Path<String>
) -> Result<StatusCode, AppError> {
    let ok = repository::unarchive_todo(&state.db, &id).await?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

async fn sync_now(State(state): State<AppState>) -> Result<Json<SyncStats>, AppError> {
    let service = SyncService::new(state.db.clone(), state.notion.clone());
    let stats = service.sync_all().await?;
    Ok(Json(stats))
}
