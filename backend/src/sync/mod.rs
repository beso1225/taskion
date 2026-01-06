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
        info!("Step 1: Pushing local changes to Notion");
        self.push_local_changes_to_notion().await?;
        info!("Step 2: Syncing courses from Notion");
        self.sync_courses_from_notion().await?;
        info!("Step 3: Syncing todos from Notion");
        self.sync_todos_from_notion().await?;
        info!("Sync completed successfully");
        Ok(())
    }

    async  fn sync_courses_from_notion(&self) -> Result<(), AppError> {
        let courses = self.notion.fetch_courses().await?;
        let notion_ids: Vec<String> = courses.iter().map(|c| c.id.clone()).collect();

        for course in courses {
            if let Ok(Some(existing)) = repository::find_course_by_id(&self.db, &course.id).await {
                if existing.sync_state == "pending" {
                    info!("Skipping course (local pending): {}", course.title);
                    continue;
                }
            }
            info!("Upserting course: {}", course.title);
            repository::upsert_course(&self.db, &course).await?;
        }
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
            if let Ok(Some(existing)) = repository::find_todo_by_id(&self.db, &todo.id).await {
                if existing.sync_state == "pending" {
                    info!("Skipping todo (local pending): {}", todo.title);
                    continue;
                }
            }
            info!("Upserting todo: {}", todo.title);
            repository::upsert_todo(&self.db, &todo).await?;
        }
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

    async fn push_local_changes_to_notion(&self) -> Result<(), AppError> {
        let courses = repository::fetch_courses(&self.db).await?;
        info!("Found {} courses to check for push", courses.len());
        
        for course in courses {
            info!("Course: {} - sync_state: {}", course.title, course.sync_state);
            if course.sync_state != "synced" {
                info!("Pushing course to Notion: {}", course.title);
                self.notion.push_course(&course).await?;
                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query!(
                    "UPDATE courses SET sync_state = 'synced', last_synced_at = ? WHERE id = ?",
                    now,
                    course.id
                )
                .execute(&self.db)
                .await?;
                info!("Successfully pushed and updated sync_state for: {}", course.title);
            }
        }

        let todos = repository::fetch_todos(&self.db).await?;
        info!("Found {} todos to check for push", todos.len());
        
        for todo in todos {
            if todo.sync_state != "synced" {
                info!("Pushing todo to Notion: {}", todo.title);
                self.notion.push_todo(&todo).await?;
                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query!(
                    "UPDATE todos SET sync_state = 'synced', last_synced_at = ? WHERE id = ?",
                    now,
                    todo.id
                )
                .execute(&self.db)
                .await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{Course, NewCourseRequest, Todo, NewTodoRequest},
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