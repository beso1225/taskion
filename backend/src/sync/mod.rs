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
        let notion_ids: Vec<String> = courses.iter().map(|c| c.id.clone()).collect();
        
        for course in courses {
            info!("Upserting course: {}", course.title);
            repository::upsert_course(&self.db, &course).await?;
        }
        
        // Notion に存在しないローカルレコードをアーカイブ
        let local_courses = repository::fetch_courses(&self.db).await?;
        for local_course in local_courses {
            if !notion_ids.contains(&local_course.id) {
                info!("Archiving course not in Notion: {}", local_course.title);
                sqlx::query!("UPDATE courses SET is_archived = 1 WHERE id = ?", local_course.id)
                    .execute(&self.db)
                    .await?;
            }
        }
        
        Ok(())
    }

    async fn sync_todos_from_notion(&self) -> Result<(), AppError> {
        let todos = self.notion.fetch_todos().await?;
        let notion_ids: Vec<String> = todos.iter().map(|t| t.id.clone()).collect();

        for todo in todos {
            info!("Upserting todo: {}", todo.title);
            repository::upsert_todo(&self.db, &todo).await?;
        }
        
        // Notion に存在しないローカルレコードをアーカイブ
        let local_todos = repository::fetch_todos(&self.db).await?;
        for local_todo in local_todos {
            if !notion_ids.contains(&local_todo.id) {
                info!("Archiving todo not in Notion: {}", local_todo.title);
                sqlx::query!("UPDATE todos SET is_archived = 1 WHERE id = ?", local_todo.id)
                    .execute(&self.db)
                    .await?;
            }
        }
        
        Ok(())
    }
}