use async_trait::async_trait;

use crate::error::AppError;

#[async_trait]
pub trait NotionClient: Send + Sync {
    async fn sync_courses(&self) -> Result<(), AppError>;
    async fn sync_todos(&self) -> Result<(), AppError>;
    async fn push_changes(&self) -> Result<(), AppError>;
}

pub struct NoopNotionClient;

#[async_trait]
impl NotionClient for NoopNotionClient {
    async fn sync_courses(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn sync_todos(&self) -> Result<(), AppError> {
        Ok(())
    }

    async fn push_changes(&self) -> Result<(), AppError> {
        Ok(())
    }
}