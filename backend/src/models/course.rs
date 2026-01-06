use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Course {
    pub id: String,
    pub title: String,
    pub semester: String,
    pub day_of_week: String,
    pub period: i32,
    pub room: Option<String>,
    pub instructor: Option<String>,
    pub is_archived: bool,
    pub updated_at: String,
    pub sync_state: String,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCourseRequest {
    pub title: String,
    pub semester: String,
    pub day_of_week: String,
    pub period: i32,
    pub room: Option<String>,
    pub instructor: Option<String>,
}
