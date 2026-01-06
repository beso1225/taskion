use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{Course, NewCourseRequest, NewTodoRequest, Todo, UpdateTodoRequest};

pub async fn fetch_courses(db: &SqlitePool) -> Result<Vec<Course>, sqlx::Error> {
    sqlx::query_as!(
        Course,
        r#"
        SELECT
            id as "id!",
            title as "title!",
            semester as "semester!",
            day_of_week as "day_of_week!",
            period as "period: i32",
            room as "room?",
            instructor as "instructor?",
            is_archived as "is_archived: bool",
            updated_at as "updated_at!",
            sync_state as "sync_state!",
            last_synced_at as "last_synced_at?"
        FROM courses
        WHERE is_archived = 0
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(db)
    .await
}

pub async fn insert_course(
    db: &SqlitePool,
    req: NewCourseRequest,
) -> Result<Course, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let sync_state = "pending".to_string();

    sqlx::query!(
        r#"
        INSERT INTO courses
            (id, title, semester, day_of_week, period, room, instructor,
            is_archived, updated_at, sync_state, last_synced_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, NULL)
        "#,
        id,
        req.title,
        req.semester,
        req.day_of_week,
        req.period,
        req.room,
        req.instructor,
        now,
        sync_state,
    )
    .execute(db)
    .await?;

    Ok(Course {
        id,
        title: req.title,
        semester: req.semester,
        day_of_week: req.day_of_week,
        period: req.period,
        room: req.room,
        instructor: req.instructor,
        is_archived: false,
        updated_at: now,
        sync_state,
        last_synced_at: None,
    })
}

pub async fn fetch_todos(db: &SqlitePool) -> Result<Vec<Todo>, sqlx::Error> {
    sqlx::query_as!(
        Todo,
        r#"
        SELECT
            id as "id!",
            course_id as "course_id!",
            title as "title!",
            due_date as "due_date!",
            status as "status!",
            completed_at as "completed_at?",
            is_archived as "is_archived: bool",
            updated_at as "updated_at!",
            sync_state as "sync_state!",
            last_synced_at as "last_synced_at?"
        FROM todos
        WHERE is_archived = 0
        ORDER BY updated_at DESC
        "#
    )
    .fetch_all(db)
    .await
}

pub async fn insert_todo(
    db: &SqlitePool,
    req: NewTodoRequest,
) -> Result<Todo, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let sync_state = "pending".to_string();

    sqlx::query!(
        r#"
        INSERT INTO todos
            (id, course_id, title, due_date, status,
            is_archived, updated_at, sync_state, last_synced_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, NULL)
        "#,
        id,
        req.course_id,
        req.title,
        req.due_date,
        req.status,
        now,
        sync_state,
    )
    .execute(db)
    .await?;

    Ok(Todo {
        id,
        course_id: req.course_id,
        title: req.title,
        due_date: req.due_date,
        status: req.status,
        completed_at: None,
        is_archived: false,
        updated_at: now,
        sync_state,
        last_synced_at: None,
    })
}

pub async fn update_todo(
    db: &SqlitePool,
    id: &str,
    req: UpdateTodoRequest,
) -> Result<Option<Todo>, sqlx::Error> {
    let mut current = match sqlx::query_as!(
        Todo,
        r#"
        SELECT
            id as "id!",
            course_id as "course_id!",
            title as "title!",
            due_date as "due_date!",
            status as "status!",
            completed_at as "completed_at?",
            is_archived as "is_archived: bool",
            updated_at as "updated_at!",
            sync_state as "sync_state!",
            last_synced_at as "last_synced_at?"
        FROM todos
        WHERE id = ?1
        "#,
        id
    )
    .fetch_optional(db)
    .await? {
        Some(t) => t,
        None => return Ok(None),
    };

    if let Some(title) = req.title {
        current.title = title;
    }
    if let Some(due_date) = req.due_date {
        current.due_date = due_date;
    }
    if let Some(status) = req.status {
        current.status = status;
    }
    let now = Utc::now().to_rfc3339();
    current.updated_at = now.clone();
    current.sync_state = "pending".to_string();

    sqlx::query!(
        r#"
        UPDATE todos
        SET title = ?1,
            due_date = ?2,
            status = ?3,
            updated_at = ?4,
            sync_state = ?5
        WHERE id = ?6
        "#,
        current.title,
        current.due_date,
        current.status,
        now,
        current.sync_state,
        id
    )
    .execute(db)
    .await?;

    Ok(Some(current))
}

pub async fn archive_todo(db: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query!(
        r#"
        UPDATE todos
        SET is_archived = 1,
            updated_at = ?2,
            sync_state = 'pending'
        WHERE id = ?1
        "#,
        id,
        now,
    )
    .execute(db)
    .await?
    .rows_affected();

    Ok(result > 0)
}

pub async fn find_course_by_id(db: &SqlitePool, id: &str) -> Result<Option<Course>, sqlx::Error> {
    sqlx::query_as::<_, Course>(
        "SELECT id, title, semester, day_of_week, period, room, instructor, is_archived, updated_at, sync_state, last_synced_at FROM courses WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn upsert_course(db: &SqlitePool, course: &Course) -> Result<Course, sqlx::Error> {
    match find_course_by_id(db, &course.id).await? {
        Some(_) => {
            // Update
            sqlx::query(
                "UPDATE courses SET title = ?, semester = ?, day_of_week = ?, period = ?, room = ?, instructor = ?, is_archived = ?, updated_at = ?, sync_state = ?, last_synced_at = ? WHERE id = ?"
            )
            .bind(&course.title)
            .bind(&course.semester)
            .bind(&course.day_of_week)
            .bind(course.period)
            .bind(&course.room)
            .bind(&course.instructor)
            .bind(course.is_archived)
            .bind(&course.updated_at)
            .bind(&course.sync_state)
            .bind(&course.last_synced_at)
            .bind(&course.id)
            .execute(db)
            .await?;
        }
        None => {
            // Insert
            sqlx::query(
                "INSERT INTO courses (id, title, semester, day_of_week, period, room, instructor, is_archived, updated_at, sync_state, last_synced_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&course.id)
            .bind(&course.title)
            .bind(&course.semester)
            .bind(&course.day_of_week)
            .bind(course.period)
            .bind(&course.room)
            .bind(&course.instructor)
            .bind(course.is_archived)
            .bind(&course.updated_at)
            .bind(&course.sync_state)
            .bind(&course.last_synced_at)
            .execute(db)
            .await?;
        }
    }

    find_course_by_id(db, &course.id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn find_todo_by_id(db: &SqlitePool, id: &str) -> Result<Option<Todo>, sqlx::Error> {
    sqlx::query_as::<_, Todo>(
        "SELECT id, course_id, title, due_date, status, completed_at, is_archived, updated_at, sync_state, last_synced_at FROM todos WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn upsert_todo(db: &SqlitePool, todo: &Todo) -> Result<Todo, sqlx::Error> {
    match find_todo_by_id(db, &todo.id).await? {
        Some(_) => {
            sqlx::query(
                "UPDATE todos SET course_id = ?, title = ?, due_date = ?, status = ?, completed_at = ?, is_archived = ?, updated_at = ?, sync_state = ?, last_synced_at = ? WHERE id = ?"
            )
            .bind(&todo.course_id)
            .bind(&todo.title)
            .bind(&todo.due_date)
            .bind(&todo.status)
            .bind(&todo.completed_at)
            .bind(todo.is_archived)
            .bind(&todo.updated_at)
            .bind(&todo.sync_state)
            .bind(&todo.last_synced_at)
            .bind(&todo.id)
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO todos (id, course_id, title, due_date, status, completed_at, is_archived, updated_at, sync_state, last_synced_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&todo.id)
            .bind(&todo.course_id)
            .bind(&todo.title)
            .bind(&todo.due_date)
            .bind(&todo.status)
            .bind(&todo.completed_at)
            .bind(todo.is_archived)
            .bind(&todo.updated_at)
            .bind(&todo.sync_state)
            .bind(&todo.last_synced_at)
            .execute(db)
            .await?;
        }
    }

    find_todo_by_id(db, &todo.id)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite://:memory:")
            .await
            .expect("Failed to create test db");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    #[tokio::test]
    async fn test_insert_and_fetch_course() {
        let pool = setup_test_db().await;

        let req = NewCourseRequest {
            title: "数学I".to_string(),
            semester: "2A2".to_string(),
            day_of_week: "Mon".to_string(),
            period: 1,
            room: Some("101".to_string()),
            instructor: Some("田中先生".to_string()),
        };

        let course = insert_course(&pool, req).await.expect("Failed to insert course");
        assert_eq!(course.title, "数学I");
        assert_eq!(course.sync_state, "pending");
        assert!(!course.is_archived);

        let courses = fetch_courses(&pool).await.expect("Failed to fetch courses");
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].id, course.id);
    }

    #[tokio::test]
    async fn test_insert_and_fetch_todo() {
        let pool = setup_test_db().await;

        // まずコースを作成
        let course_req = NewCourseRequest {
            title: "数学I".to_string(),
            semester: "2A2".to_string(),
            day_of_week: "Mon".to_string(),
            period: 1,
            room: None,
            instructor: None,
        };
        let course = insert_course(&pool, course_req)
            .await
            .expect("Failed to insert course");

        // TODOを作成
        let todo_req = NewTodoRequest {
            course_id: course.id.clone(),
            title: "宿題1".to_string(),
            due_date: "2026-01-10".to_string(),
            status: "未着手".to_string(),
        };
        let todo = insert_todo(&pool, todo_req)
            .await
            .expect("Failed to insert todo");

        assert_eq!(todo.title, "宿題1");
        assert_eq!(todo.status, "未着手");
        assert_eq!(todo.sync_state, "pending");
        assert!(!todo.is_archived);

        let todos = fetch_todos(&pool).await.expect("Failed to fetch todos");
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].course_id, course.id);
    }

    #[tokio::test]
    async fn test_update_todo() {
        let pool = setup_test_db().await;

        // コース作成
        let course_req = NewCourseRequest {
            title: "数学I".to_string(),
            semester: "2A2".to_string(),
            day_of_week: "Mon".to_string(),
            period: 1,
            room: None,
            instructor: None,
        };
        let course = insert_course(&pool, course_req)
            .await
            .expect("Failed to insert course");

        // TODO作成
        let todo_req = NewTodoRequest {
            course_id: course.id.clone(),
            title: "宿題1".to_string(),
            due_date: "2026-01-10".to_string(),
            status: "未着手".to_string(),
        };
        let todo = insert_todo(&pool, todo_req)
            .await
            .expect("Failed to insert todo");

        // TODO更新
        let update_req = UpdateTodoRequest {
            title: Some("宿題1修正".to_string()),
            due_date: None,
            status: Some("完了".to_string()),
        };
        let updated = update_todo(&pool, &todo.id, update_req)
            .await
            .expect("Failed to update todo")
            .expect("Todo not found");

        assert_eq!(updated.title, "宿題1修正");
        assert_eq!(updated.status, "完了");
        assert_eq!(updated.sync_state, "pending");
    }

    #[tokio::test]
    async fn test_archive_todo() {
        let pool = setup_test_db().await;

        // コース作成
        let course_req = NewCourseRequest {
            title: "数学I".to_string(),
            semester: "2A2".to_string(),
            day_of_week: "Mon".to_string(),
            period: 1,
            room: None,
            instructor: None,
        };
        let course = insert_course(&pool, course_req)
            .await
            .expect("Failed to insert course");

        // TODO作成
        let todo_req = NewTodoRequest {
            course_id: course.id.clone(),
            title: "宿題1".to_string(),
            due_date: "2026-01-10".to_string(),
            status: "未着手".to_string(),
        };
        let todo = insert_todo(&pool, todo_req)
            .await
            .expect("Failed to insert todo");

        // TODO削除（論理削除）
        let result = archive_todo(&pool, &todo.id)
            .await
            .expect("Failed to archive todo");
        assert!(result);

        // アーカイブ済みTODOは取得されない
        let todos = fetch_todos(&pool).await.expect("Failed to fetch todos");
        assert_eq!(todos.len(), 0);
    }
}