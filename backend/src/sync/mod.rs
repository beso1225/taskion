use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{error::AppError, notion::NotionClient};

pub struct SyncServie {
    db: SqlitePool,
    notion: Arc<dyn NotionClient>,
}

impl SyncServie {
    pub fn new(db: SqlitePool, notion: Arc<dyn NotionClient>) -> Self {
        Self { db, notion }
    }

    pub async fn sync_all(&self) -> Result<(), AppError> {
        // TODO: Implement Pull/Push logic
        self.notion.sync_courses().await?;
        self.notion.sync_todos().await?;
        self.notion.push_changes().await?;
        Ok(())
    }
}