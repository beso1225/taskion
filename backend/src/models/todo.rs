use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Todo {
    pub id: String,
    pub course_id: String,
    pub title: String,
    pub due_date: String,
    pub status: String,
    pub completed_at: Option<String>,
    pub is_archived: bool,
    pub updated_at: String,
    pub sync_state: String,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTodoRequest {
    pub course_id: String,
    pub title: String,
    pub due_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTodoRequest {
    pub title: Option<String>,
    pub due_date: Option<String>,
    pub status: Option<String>,
}
