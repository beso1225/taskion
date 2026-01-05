use std::env;

use async_trait::async_trait;
use reqwest::Client;

use crate::error::AppError;

#[derive(Clone, Debug)]
pub struct NotionConfig {
    pub api_token: String,
    pub courses_db_id: String,
    pub todos_db_id: String,
}

impl NotionConfig {
    pub fn new_from_env() -> Result<Self, AppError> {
        let api_token = env::var("NOTION_TOKEN")
            .map_err(|_| AppError::BadRequest("NOTION_TOKEN is not set".to_string()))?;
        let courses_db_id = env::var("COURSES_DB_ID")
            .map_err(|_| AppError::BadRequest("COURSES_DB_ID is not set".to_string()))?;
        let todos_db_id = env::var("TODOS_DB_ID")
            .map_err(|_| AppError::BadRequest("TODOS_DB_ID is not set".to_string()))?;

        Ok(Self {
            api_token,
            courses_db_id,
            todos_db_id,
        })
    }
}

#[async_trait]
pub trait NotionClient: Send + Sync {
    async fn sync_courses(&self) -> Result<(), AppError>;
    async fn sync_todos(&self) -> Result<(), AppError>;
    async fn push_changes(&self) -> Result<(), AppError>;
}

pub struct NotionHttpClient {
    client: Client,
    config: NotionConfig,
}

impl NotionHttpClient {
    pub fn new(config: NotionConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .build()
            .map_err(|e| AppError::BadRequest(format!("Failed to build http client: {}", e)))?;
        Ok(Self { client, config })
    }
}

#[async_trait]
impl NotionClient for NotionHttpClient {
    async fn sync_courses(&self) -> Result<(), AppError> {
        // TODO: Pull courses from Notion
        Ok(())
    }

    async fn sync_todos(&self) -> Result<(), AppError> {
        // TODO: Pull todos from Notion
        Ok(())
    }

    async fn push_changes(&self) -> Result<(), AppError> {
        // TODO: Push local changes to Notion
        Ok(())
    }
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