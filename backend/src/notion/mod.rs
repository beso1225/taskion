pub mod dto;

use std::env;

use async_trait::async_trait;
use chrono::Utc;
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
    async fn fetch_courses(&self) -> Result<Vec<crate::models::Course>, AppError>;
    async fn fetch_todos(&self) -> Result<Vec<crate::models::Todo>, AppError>;
    async fn push_course(&self, course: &crate::models::Course) -> Result<(), AppError>;
    async fn push_todo(&self, todo: &crate::models::Todo) -> Result<(), AppError>;
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

    async fn query_database(&self, database_id: &str) -> Result<dto::QueryDatabaseResponse, AppError> {
        let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);

        let request_body = dto::QueryDatabaseRequest {
            filter: None,
            sorts: None,
            start_cursor: None,
            page_size: Some(100),
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Notion-Version", "2022-06-28")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::BadRequest(format!("Notion API error {}: {}", status, body)));
        }

        response
            .json::<dto::QueryDatabaseResponse>()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to parse Notion response: {}", e)))
    }

    async fn parse_coourse_from_page(&self, page: &dto::Page) -> Result<crate::models::Course, AppError> {
        let title = self.get_property_text(page, "Name")?;
        let semester = self.get_property_text(page, "Semester").unwrap_or_else(|_| "".to_string());
        let day_of_week = self.get_property_text(page, "Day").unwrap_or_else(|_| "".to_string());
        let period = self.get_property_number(page, "Period")
            .and_then(|n| u32::try_from(n as u32).ok())
            .map(|n| n as i32)
            .unwrap_or(0);
        let room = self.get_property_text(page, "Room").ok();
        let instructor = self.get_property_text(page, "Instructor").ok();

        Ok(crate::models::Course {
            id: page.id.clone(),
            title,
            semester,
            day_of_week,
            period,
            room,
            instructor,
            is_archived: page.archived,
            updated_at: page.last_edited_time.clone(),
            sync_state: "synced".to_string(),
            last_synced_at: Some(Utc::now().to_rfc3339()),
        })
    }

    async fn parse_todo_from_page(&self, page: &dto::Page) -> Result<crate::models::Todo, AppError> {
        let title = self.get_property_text(page, "Title")?;
        let due_date = self.get_property_date(page, "Due Date")
            .unwrap_or_else(|_| chrono::Local::now().format("%Y-%m-%d").to_string());
        let status = self.get_property_text(page, "Status")
            .unwrap_or_else(|_| "未着手".to_string());
        let course_id = self.get_property_relation(page, "Course")
            .unwrap_or_else(|_| "".to_string());

        Ok(crate::models::Todo {
            id: page.id.clone(),
            course_id,
            title,
            due_date,
            status,
            completed_at: None,
            is_archived: page.archived,
            updated_at: page.last_edited_time.clone(),
            sync_state: "synced".to_string(),
            last_synced_at: Some(Utc::now().to_rfc3339()),
        })
    }

    fn get_property_date(&self, page: &dto::Page, key: &str) -> Result<String, AppError> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::Date { date } => {
                    date.as_ref().map(|d| d.start.clone())
                }
                _ => None,
            })
            .ok_or_else(|| AppError::BadRequest(format!("Missing date property: {}", key)))
    }

    fn get_property_relation(&self, page: &dto::Page, key: &str) -> Result<String, AppError> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::Relation { relation } => {
                    relation.first().map(|r| r.id.clone())
                }
                _ => None,
            })
            .ok_or_else(|| AppError::BadRequest(format!("Missing relation property: {}", key)))
    }

    fn get_property_text(&self, page: &dto::Page, key: &str) -> Result<String, AppError> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::Title { title } => {
                    Some(title.iter().map(|t| t.plain_text.clone()).collect::<Vec<_>>().join(""))
                }
                dto::Property::RichText { rich_text } => {
                    Some(rich_text.iter().map(|t| t.plain_text.clone()).collect::<Vec<_>>().join(""))
                }
                _ => None,
            })
            .ok_or_else(|| AppError::BadRequest(format!("Missing property: {}", key)))
    }

    fn get_property_number(&self, page: &dto::Page, key: &str) -> Option<f64> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::Number { number } => *number,
                _ => None,
            })
    }
}

#[async_trait]
impl NotionClient for NotionHttpClient {
    async fn fetch_courses(&self) -> Result<Vec<crate::models::Course>, AppError> {
        let response = self.query_database(&self.config.courses_db_id).await?;
        let mut courses = Vec::new();

        for page in response.results {
            match self.parse_coourse_from_page(&page).await {
                Ok(course) => courses.push(course),
                Err(e) => {
                    tracing::warn!("Failed to parse course from page {}: {}", page.id, e);
                }
            }
        }
        Ok(courses)
    }

    async fn fetch_todos(&self) -> Result<Vec<crate::models::Todo>, AppError> {
        let response = self.query_database(&self.config.todos_db_id).await?;
        let mut todos = Vec::new();

        for page in response.results {
            match self.parse_todo_from_page(&page).await {
                Ok(todo) => todos.push(todo),
                Err(e) => {
                    tracing::warn!("Failed to parse todo from page {}: {}", page.id, e);
                }
            }
        }

        Ok(todos)
    }

    async fn push_course(&self, _course: &crate::models::Course) -> Result<(), AppError> {
        // TODO: Notion に course を push
        Ok(())
    }

    async fn push_todo(&self, _todo: &crate::models::Todo) -> Result<(), AppError> {
        // TODO: Notion に todo を push
        Ok(())
    }
}

pub struct NoopNotionClient;

#[async_trait]
impl NotionClient for NoopNotionClient {
    async fn fetch_courses(&self) -> Result<Vec<crate::models::Course>, AppError> {
        Ok(Vec::new())
    }

    async fn fetch_todos(&self) -> Result<Vec<crate::models::Todo>, AppError> {
        Ok(Vec::new())
    }

    async fn push_course(&self, _course: &crate::models::Course) -> Result<(), AppError> {
        Ok(())
    }

    async fn push_todo(&self, _todo: &crate::models::Todo) -> Result<(), AppError> {
        Ok(())
    }
}
