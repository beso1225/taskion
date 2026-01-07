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

pub async fn fetch_pending_todos(db: &SqlitePool) -> Result<Vec<Todo>, sqlx::Error> {
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
        WHERE sync_state != 'synced'
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
