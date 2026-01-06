use std::sync::Arc;

use serde::Serialize;
use sqlx::SqlitePool;
use tracing::{info, warn};

use crate::{error::AppError, notion::NotionClient};
use crate::db::repository;

pub struct SyncService {
    db: SqlitePool,
    notion: Arc<dyn NotionClient>,
}

#[derive(Debug, Serialize)]
pub struct SyncStats {
    pub courses_pushed: usize,
    pub courses_pulled: usize,
    pub courses_skipped: usize,
    pub todos_pushed: usize,
    pub todos_pulled: usize,
    pub todos_skipped: usize,
}

impl SyncService {
    pub fn new(db: SqlitePool, notion: Arc<dyn NotionClient>) -> Self {
        Self { db, notion }
    }

    pub async fn sync_all(&self) -> Result<SyncStats, AppError> {
        info!("Starting sync...");
        let mut stats = SyncStats {
            courses_pushed: 0,
            courses_pulled: 0,
            courses_skipped: 0,
            todos_pushed: 0,
            todos_pulled: 0,
            todos_skipped: 0,
        };

        info!("Step 1: Pushing local changes to Notion");
        let (pushed_courses, pushed_todos) = self.push_local_changes_to_notion().await?;
        stats.courses_pushed = pushed_courses;
        stats.todos_pushed = pushed_todos;
        info!("Pushed {} courses, {} todos", pushed_courses, pushed_todos);

        info!("Step 2: Syncing courses from Notion");
        let (pulled_courses, skipped_courses) = self.sync_courses_from_notion().await?;
        stats.courses_pulled = pulled_courses;
        stats.courses_skipped = skipped_courses;
        info!("Pulled {} courses, skipped {} (local pending)", pulled_courses, skipped_courses);

        info!("Step 3: Syncing todos from Notion");
        let (pulled_todos, skipped_todos) = self.sync_todos_from_notion().await?;
        stats.todos_pulled = pulled_todos;
        stats.todos_skipped = skipped_todos;
        info!("Pulled {} todos, skipped {} (local pending)", pulled_todos, skipped_todos);

        info!("Sync completed successfully: {:?}", stats);
        Ok(stats)
    }

    async fn sync_courses_from_notion(&self) -> Result<(usize, usize), AppError> {
        let notion_courses = self.notion.fetch_courses().await?;
        let notion_ids: Vec<String> = notion_courses.iter().map(|c| c.id.clone()).collect();
        
        let mut pulled = 0;
        let mut skipped = 0;

        // Fetch all local courses once
        let local_courses_map: std::collections::HashMap<String, crate::models::Course> = 
            repository::fetch_courses(&self.db)
                .await?
                .into_iter()
                .map(|c| (c.id.clone(), c))
                .collect();

        // Upsert from Notion with conflict detection
        for course in notion_courses {
            if let Some(existing) = local_courses_map.get(&course.id) {
                if existing.sync_state == "pending" {
                    warn!("Skipping course (local pending): {}", course.title);
                    skipped += 1;
                    continue;
                }
                // Check if local is newer (avoid overwriting recent local changes)
                if let (Some(local_updated), Some(notion_updated)) = 
                    (parse_timestamp(&existing.updated_at), parse_timestamp(&course.updated_at)) {
                    if local_updated > notion_updated {
                        warn!("Skipping course (local newer): {} local={:?} notion={:?}", 
                              course.title, local_updated, notion_updated);
                        skipped += 1;
                        continue;
                    }
                }
            }
            
            repository::upsert_course(&self.db, &course).await?;
            pulled += 1;
        }

        // Archive courses not in Notion (batch update)
        let courses_to_archive: Vec<String> = local_courses_map
            .keys()
            .filter(|id| !notion_ids.contains(id))
            .cloned()
            .collect();

        if !courses_to_archive.is_empty() {
            for id in courses_to_archive {
                sqlx::query!("UPDATE courses SET is_archived = 1 WHERE id = ?", id)
                    .execute(&self.db)
                    .await?;
            }
        }

        Ok((pulled, skipped))
    }

    async fn sync_todos_from_notion(&self) -> Result<(usize, usize), AppError> {
        let notion_todos = self.notion.fetch_todos().await?;
        let notion_ids: Vec<String> = notion_todos.iter().map(|t| t.id.clone()).collect();
        
        let mut pulled = 0;
        let mut skipped = 0;

        // Fetch all local todos once
        let local_todos_map: std::collections::HashMap<String, crate::models::Todo> = 
            repository::fetch_todos(&self.db)
                .await?
                .into_iter()
                .map(|t| (t.id.clone(), t))
                .collect();

        // Upsert from Notion with conflict detection
        for todo in notion_todos {
            if let Some(existing) = local_todos_map.get(&todo.id) {
                if existing.sync_state == "pending" {
                    warn!("Skipping todo (local pending): {}", todo.title);
                    skipped += 1;
                    continue;
                }
                // Check if local is newer
                if let (Some(local_updated), Some(notion_updated)) = 
                    (parse_timestamp(&existing.updated_at), parse_timestamp(&todo.updated_at)) {
                    if local_updated > notion_updated {
                        warn!("Skipping todo (local newer): {}", todo.title);
                        skipped += 1;
                        continue;
                    }
                }
            }
            
            repository::upsert_todo(&self.db, &todo).await?;
            pulled += 1;
        }

        // Archive todos not in Notion (batch update)
        let todos_to_archive: Vec<String> = local_todos_map
            .keys()
            .filter(|id| !notion_ids.contains(id))
            .cloned()
            .collect();

        if !todos_to_archive.is_empty() {
            for id in todos_to_archive {
                sqlx::query!("UPDATE todos SET is_archived = 1 WHERE id = ?", id)
                    .execute(&self.db)
                    .await?;
            }
        }

        Ok((pulled, skipped))
    }

    async fn push_local_changes_to_notion(&self) -> Result<(usize, usize), AppError> {
        let courses = repository::fetch_courses(&self.db).await?;
        let mut pushed_count = 0;

        // Only push courses with sync_state != 'synced'
        for course in courses {
            if course.sync_state != "synced" {
                self.notion.push_course(&course).await?;
                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query!(
                    "UPDATE courses SET sync_state = 'synced', last_synced_at = ? WHERE id = ?",
                    now,
                    course.id
                )
                .execute(&self.db)
                .await?;
                pushed_count += 1;
            }
        }

        let todos = repository::fetch_todos(&self.db).await?;
        let mut todo_count = 0;
        
        for todo in todos {
            if todo.sync_state != "synced" {
                self.notion.push_todo(&todo).await?;
                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query!(
                    "UPDATE todos SET sync_state = 'synced', last_synced_at = ? WHERE id = ?",
                    now,
                    todo.id
                )
                .execute(&self.db)
                .await?;
                todo_count += 1;
            }
        }

        Ok((pushed_count, todo_count))
    }
}

/// Parse RFC3339 timestamp to comparable format
fn parse_timestamp(ts: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}
