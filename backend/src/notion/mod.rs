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

        let body_text = response.text().await.unwrap_or_default();
        let filename = if database_id == &self.config.courses_db_id {
            "notion_courses_response.json"
        } else {
            "notion_todos_response.json"
        };

        use std::fs;
        fs::write(filename, &body_text).ok();
        tracing::info!("Notion response saved to {}", filename);

        serde_json::from_str::<dto::QueryDatabaseResponse>(&body_text)
        .map_err(|e| {
            tracing::error!("Failed to parse: {}", e);
            AppError::BadRequest(format!("Failed to parse Notion response: {}", e))
        })
    }

    async fn find_page_id_by_text_property(
        &self,
        database_id: &str,
        property_name: &str,
        value: &str,
    ) -> Result<Option<String>, AppError> {
        let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);

        let filter = serde_json::json!({
            "property": property_name,
            "rich_text": { "equals": value }
        });

        let request_body = dto::QueryDatabaseRequest {
            filter: Some(filter),
            sorts: None,
            start_cursor: None,
            page_size: Some(1),
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_token))
            .header("Notion-Version", "2022-06-28")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| AppError::InternalServerError)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::BadRequest(format!("Notion API error {}: {}", status, body)));
        }

        let body_text = response.text().await.unwrap_or_default();
        let parsed: dto::QueryDatabaseResponse = serde_json::from_str(&body_text)
            .map_err(|e| AppError::BadRequest(format!("Failed to parse Notion response: {}", e)))?;

        Ok(parsed.results.first().map(|p| p.id.clone()))
    }

    async fn parse_coourse_from_page(&self, page: &dto::Page) -> Result<crate::models::Course, AppError> {
        let id = page.id.clone();
        let title = self.get_property_text(page, "Name")?;
        let semester = self.get_property_multi_select(page, "Semester")
            .map(|items| items.join(", "))
            .unwrap_or_else(|_| "".to_string());
        let day_of_week = self.get_property_select(page, "Day").unwrap_or_else(|_| "".to_string());
        let period = self.get_property_multi_select(page, "Period")
            .ok()
            .and_then(|items| items.first().cloned())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let room = self.get_property_text(page, "Room").ok();
        let instructor = self.get_property_multi_select(page, "Instructor")
            .map(|items| items.join(", "))
            .ok();

        Ok(crate::models::Course {
            id: id,
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
        let id = self.get_property_text(page, "todo_id")
            .unwrap_or_else(|_| page.id.clone());
        
        let title = self.get_property_text(page, "Title")?;
        
        let due_date = self.get_property_date(page, "Due Date")
            .unwrap_or_else(|_| chrono::Local::now().format("%Y-%m-%d").to_string());
        
        let status = self.get_property_status(page, "Status")
            .unwrap_or_else(|_| "未着手".to_string());
        
        let course_id = self.get_property_relation(page, "Course")
            .unwrap_or_else(|_| "".to_string());
        
        let completed_at = self.get_property_date(page, "completed_at").ok();
        
        let is_archived = page.properties
            .get("is_archived")
            .and_then(|prop| match prop {
                dto::Property::Checkbox { checkbox } => Some(*checkbox),
                _ => None,
            })
            .unwrap_or(page.archived);

        Ok(crate::models::Todo {
            id,
            course_id,
            title,
            due_date,
            status,
            completed_at,
            is_archived,
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

    fn get_property_status(&self, page: &dto::Page, key: &str) -> Result<String, AppError> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::Status { status } => {
                    status.as_ref().map(|s| s.name.clone())
                }
                _ => None,
            })
            .ok_or_else(|| AppError::BadRequest(format!("Missing status property: {}", key)))
    }
    fn get_property_select(&self, page: &dto::Page, key: &str) -> Result<String, AppError> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::Select { select } => {
                    select.as_ref().map(|s| s.name.clone())
                }
                _ => None,
            })
            .ok_or_else(|| AppError::BadRequest(format!("Missing select property: {}", key)))
    }

    fn get_property_multi_select(&self, page: &dto::Page, key: &str) -> Result<Vec<String>, AppError> {
        page.properties
            .get(key)
            .and_then(|prop| match prop {
                dto::Property::MultiSelect { multi_select } => {
                    Some(multi_select.iter().map(|s| s.name.clone()).collect())
                }
                _ => None,
            })
            .ok_or_else(|| AppError::BadRequest(format!("Missing multi_select property: {}", key)))
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

    async fn push_course(&self, course: &crate::models::Course) -> Result<(), AppError> {
        // Try to locate the Notion page by course_id property first
        let existing_page_id = self
            .find_page_id_by_text_property(&self.config.courses_db_id, "course_id", &course.id)
            .await?;

        let mut properties = serde_json::json!({});

        properties["Name"] = serde_json::json!({
            "title": [{
                "text": {
                    "content": course.title
                }
            }]
        });

        let semester_items: Vec<serde_json::Value> = course.semester
            .split(", ")
            .map(|s| serde_json::json!({ "name": s.trim() }))
            .collect();
        properties["Semester"] = serde_json::json!({
            "multi_select": semester_items
        });

        if !course.day_of_week.is_empty() {
            properties["Day"] = serde_json::json!({
                "select": { "name": course.day_of_week }
            });
        }

        if course.period > 0 {
            properties["Period"] = serde_json::json!({
                "multi_select": [{ "name": course.period.to_string() }]
            });
        }

        if let Some(room) = &course.room {
            properties["Room"] = serde_json::json!({
                "rich_text": [{
                    "text": { "content": room }
                }]
            });
        }

        if let Some(instructor) = &course.instructor {
            let instructor_items: Vec<serde_json::Value> = instructor
                .split(", ")
                .map(|s| serde_json::json!({ "name": s.trim() }))
                .collect();
            properties["Instructor"] = serde_json::json!({
                "multi_select": instructor_items
            });
        }

        // Always ensure course_id rich_text property mirrors local id
        properties["course_id"] = serde_json::json!({
            "rich_text": [{
                "text": { "content": course.id }
            }]
        });

        if let Some(page_id) = existing_page_id {
            // Update existing page
            let url = format!("https://api.notion.com/v1/pages/{}", page_id);
            let request_body = dto::UpdatePageRequest { properties };

            let response = self.client
                .patch(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_token))
                .header("Notion-Version", "2022-06-28")
                .json(&request_body)
                .send()
                .await
                .map_err(|_| AppError::InternalServerError)?;

            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::info!("Notion API response (update course): {} - {}", status, body);

            if !status.is_success() {
                return Err(AppError::BadRequest(format!("Failed to push course to Notion: {} {}", status, body)));
            }
        } else {
            // Create new page in Courses database
            let url = "https://api.notion.com/v1/pages";
            let request_body = serde_json::json!({
                "parent": { "database_id": self.config.courses_db_id },
                "properties": properties
            });

            let response = self.client
                .post(url)
                .header("Authorization", format!("Bearer {}", self.config.api_token))
                .header("Notion-Version", "2022-06-28")
                .json(&request_body)
                .send()
                .await
                .map_err(|_| AppError::InternalServerError)?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AppError::BadRequest(format!("Failed to create course in Notion: {} {}", status, body)));
            }
        }

        Ok(())
    }

    async fn push_todo(&self, todo: &crate::models::Todo) -> Result<(), AppError> {
        // Locate the Notion page by todo_id property (rich_text)
        let existing_page_id = self
            .find_page_id_by_text_property(&self.config.todos_db_id, "todo_id", &todo.id)
            .await?;

        let mut properties = serde_json::json!({});

        properties["Title"] = serde_json::json!({
            "title": [{
                "text": {
                    "content": todo.title
                }
            }]
        });

        properties["Due Date"] = serde_json::json!({
            "date": {
                "start": todo.due_date
            }
        });

        properties["Status"] = serde_json::json!({
            "status": { "name": todo.status }
        });

        // Set is_archived checkbox
        properties["is_archived"] = serde_json::json!({
            "checkbox": todo.is_archived
        });

        // Ensure todo_id rich_text is set to local id
        properties["todo_id"] = serde_json::json!({
            "rich_text": [{
                "text": { "content": todo.id }
            }]
        });

        if let Some(page_id) = existing_page_id {
            // Update existing page
            let url = format!("https://api.notion.com/v1/pages/{}", page_id);
            let request_body = dto::UpdatePageRequest { properties };

            let response = self.client
                .patch(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_token))
                .header("Notion-Version", "2022-06-28")
                .json(&request_body)
                .send()
                .await
                .map_err(|_| AppError::InternalServerError)?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AppError::BadRequest(format!("Failed to push todo to Notion: {} {}", status, body)));
            }
        } else {
            // Create new page in Todos database
            let url = "https://api.notion.com/v1/pages";
            // Build properties manually including relation to Course
            let mut full_props = properties.clone();
            full_props["Course"] = serde_json::json!({ "relation": [{ "id": todo.course_id }] });
            let request_body = serde_json::json!({
                "parent": { "database_id": self.config.todos_db_id },
                "properties": full_props
            });

            let response = self.client
                .post(url)
                .header("Authorization", format!("Bearer {}", self.config.api_token))
                .header("Notion-Version", "2022-06-28")
                .json(&request_body)
                .send()
                .await
                .map_err(|_| AppError::InternalServerError)?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(AppError::BadRequest(format!("Failed to create todo in Notion: {} {}", status, body)));
            }
        }

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
