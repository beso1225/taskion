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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{NewCourseRequest},
        notion::NoopNotionClient,
    };
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        sqlx::query(
            r#"
            CREATE TABLE courses (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                semester TEXT NOT NULL,
                day_of_week TEXT NOT NULL,
                period INTEGER NOT NULL,
                room TEXT,
                instructor TEXT,
                is_archived INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                sync_state TEXT NOT NULL CHECK(sync_state IN ('pending', 'synced')) DEFAULT 'pending',
                last_synced_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create courses table");

        sqlx::query(
            r#"
            CREATE TABLE todos (
                id TEXT PRIMARY KEY,
                course_id TEXT NOT NULL,
                title TEXT NOT NULL,
                due_date TEXT NOT NULL,
                status TEXT NOT NULL,
                completed_at TEXT,
                is_archived INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                sync_state TEXT NOT NULL CHECK(sync_state IN ('pending', 'synced')) DEFAULT 'pending',
                last_synced_at TEXT,
                FOREIGN KEY (course_id) REFERENCES courses(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create todos table");

        pool
    }

    #[tokio::test]
    async fn test_push_local_pending_course() {
        let db = setup_db().await;
        let notion = Arc::new(NoopNotionClient);
        let sync = SyncService::new(db.clone(), notion);

        let req = NewCourseRequest {
            title: "Rust Programming".to_string(),
            semester: "Spring".to_string(),
            day_of_week: "Monday".to_string(),
            period: 1,
            room: Some("A101".to_string()),
            instructor: Some("Prof. Smith".to_string()),
        };

        repository::insert_course(&db, req)
            .await
            .expect("Failed to insert course");

        sync.push_local_changes_to_notion()
            .await
            .expect("Failed to push");

        let courses = repository::fetch_courses(&db)
            .await
            .expect("Failed to fetch courses");

        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].sync_state, "synced", "sync_state should be updated to 'synced'");
        assert!(
            courses[0].last_synced_at.is_some(),
            "last_synced_at should be set"
        );
    }

    #[tokio::test]
    async fn test_pull_preserves_local_pending_course() {
        let db = setup_db().await;
        let notion = Arc::new(NoopNotionClient);
        let sync = SyncService::new(db.clone(), notion);

        // Insert a local pending course manually
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO courses (id, title, semester, day_of_week, period, room, instructor, is_archived, updated_at, sync_state)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind("course-1")
        .bind("Local Course")
        .bind("Spring")
        .bind("Monday")
        .bind(1)
        .bind("A101")
        .bind("Prof. Smith")
        .bind(false)
        .bind(&now)
        .bind("pending")
        .execute(&db)
        .await
        .expect("Failed to insert local course");

        // Pull from Notion (which has no courses with NoopNotionClient)
        sync.sync_courses_from_notion()
            .await
            .expect("Failed to sync courses");

        let updated = repository::find_course_by_id(&db, "course-1")
            .await
            .expect("Failed to fetch course")
            .expect("Course not found");

        assert_eq!(
            updated.sync_state, "pending",
            "sync_state should remain 'pending' to preserve local changes"
        );
        assert_eq!(
            updated.title, "Local Course",
            "Local data should not be overwritten"
        );
    }

    #[tokio::test]
    async fn test_push_skips_already_synced_course() {
        let db = setup_db().await;
        let notion = Arc::new(NoopNotionClient);
        let sync = SyncService::new(db.clone(), notion);

        let req = NewCourseRequest {
            title: "Rust Programming".to_string(),
            semester: "Spring".to_string(),
            day_of_week: "Monday".to_string(),
            period: 1,
            room: Some("A101".to_string()),
            instructor: Some("Prof. Smith".to_string()),
        };

        let course = repository::insert_course(&db, req)
            .await
            .expect("Failed to insert course");
        let course_id = course.id;

        // Mark as synced
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE courses SET sync_state = 'synced', last_synced_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&course_id)
            .execute(&db)
            .await
            .expect("Failed to update sync state");

        sync.push_local_changes_to_notion()
            .await
            .expect("Failed to push");

        let after_push = repository::find_course_by_id(&db, &course_id)
            .await
            .expect("Failed to fetch course")
            .expect("Course not found");

        assert_eq!(
            after_push.sync_state, "synced",
            "sync_state should remain 'synced'"
        );
    }

    #[tokio::test]
    async fn test_sync_all_push_then_pull_order() {
        let db = setup_db().await;
        let notion = Arc::new(NoopNotionClient);
        let sync = SyncService::new(db.clone(), notion);

        let req = NewCourseRequest {
            title: "Initial Title".to_string(),
            semester: "Spring".to_string(),
            day_of_week: "Monday".to_string(),
            period: 1,
            room: Some("A101".to_string()),
            instructor: Some("Prof. Smith".to_string()),
        };

        let course = repository::insert_course(&db, req)
            .await
            .expect("Failed to insert course");
        let course_id = course.id;

        sync.sync_all()
            .await
            .expect("Failed to sync");

        let final_state = repository::find_course_by_id(&db, &course_id)
            .await
            .expect("Failed to fetch course")
            .expect("Course not found");

        assert_eq!(
            final_state.sync_state, "synced",
            "After full sync, course should be synced"
        );
    }

    #[tokio::test]
    async fn test_archive_course_not_in_notion() {
        let db = setup_db().await;
        let notion = Arc::new(NoopNotionClient);
        let sync = SyncService::new(db.clone(), notion);

        // Insert a course
        let req = NewCourseRequest {
            title: "To Be Archived".to_string(),
            semester: "Spring".to_string(),
            day_of_week: "Monday".to_string(),
            period: 1,
            room: Some("A101".to_string()),
            instructor: Some("Prof. Smith".to_string()),
        };

        let course = repository::insert_course(&db, req)
            .await
            .expect("Failed to insert course");
        let course_id = course.id;

        // Mark as synced so it's not pending
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE courses SET sync_state = 'synced', last_synced_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&course_id)
            .execute(&db)
            .await
            .expect("Failed to update sync state");

        // Pull from Notion (which returns nothing with NoopNotionClient)
        sync.sync_courses_from_notion()
            .await
            .expect("Failed to sync courses");

        let archived = repository::find_course_by_id(&db, &course_id)
            .await
            .expect("Failed to fetch course")
            .expect("Course not found");

        assert_eq!(
            archived.is_archived, true,
            "Course not in Notion should be archived"
        );
    }
}
