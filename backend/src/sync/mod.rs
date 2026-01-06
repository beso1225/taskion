use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::info;

use crate::{error::AppError, notion::NotionClient, repository};

pub struct SyncService {
    db: SqlitePool,
    notion: Arc<dyn NotionClient>,
}

impl SyncService {
    pub fn new(db: SqlitePool, notion: Arc<dyn NotionClient>) -> Self {
        Self { db, notion }
    }

    pub async fn sync_all(&self) -> Result<(), AppError> {
        info!("Starting sync...");
        self.sync_courses_from_notion().await?;
        self.sync_todos_from_notion().await?;
        // TODO: push local changes to Notion
        Ok(())
    }

    async  fn sync_courses_from_notion(&self) -> Result<(), AppError> {
        let courses = self.notion.fetch_courses().await?;
        for course in courses {
            info!("Upserting course: {}", course.title);
            repository::upsert_course(&self.db, &course).await?;
        }
        Ok(())
    }

    async fn sync_todos_from_notion(&self) -> Result<(), AppError> {
        let todos = self.notion.fetch_todos().await?;

        for todo in todos {
            info!("Upserting todo: {}", todo.title);
            repository::upsert_todo(&self.db, &todo).await?;
        }
        Ok(())
    }
}