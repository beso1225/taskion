use std::sync::Arc;
use backend::{
    models::{Course, NewCourseRequest},
    notion::{NotionHttpClient, NotionConfig, NotionClient},
};
use sqlx::SqlitePool;

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored --test-threads=1
async fn test_push_course_to_notion() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Create in-memory database
    let db = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create database");

    // Create schema
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
    .execute(&db)
    .await
    .expect("Failed to create courses table");

    // Initialize Notion client
    let config = NotionConfig::new_from_env().expect("Failed to load Notion config");
    let notion = Arc::new(NotionHttpClient::new(config).expect("Failed to create Notion client"));

    // Create a test course
    let test_course_id = "d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4";
    let course = Course {
        id: test_course_id.to_string(),
        title: format!("Integration Test Course - {}", chrono::Utc::now().timestamp()),
        semester: "Spring".to_string(),
        day_of_week: "Monday".to_string(),
        period: 1,
        room: Some("Test Room 101".to_string()),
        instructor: Some("Test Instructor".to_string()),
        is_archived: false,
        updated_at: chrono::Utc::now().to_rfc3339(),
        sync_state: "pending".to_string(),
        last_synced_at: None,
    };

    // Insert into local DB
    sqlx::query(
        r#"
        INSERT INTO courses (id, title, semester, day_of_week, period, room, instructor, is_archived, updated_at, sync_state)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
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
    .execute(&db)
    .await
    .expect("Failed to insert course");

    // Push to Notion
    let result = notion.push_course(&course).await;
    println!("Push result: {:?}", result);
    assert!(result.is_ok(), "Failed to push course to Notion");

    // Fetch courses from Notion to verify
    let courses = notion.fetch_courses().await.expect("Failed to fetch courses");
    println!("Fetched {} courses from Notion", courses.len());

    let pushed_course = courses
        .iter()
        .find(|c| c.id == test_course_id)
        .expect("Pushed course not found in Notion");

    assert_eq!(pushed_course.title, course.title, "Title mismatch");
    assert_eq!(pushed_course.semester, course.semester, "Semester mismatch");
    assert_eq!(pushed_course.day_of_week, course.day_of_week, "Day of week mismatch");
    println!("✓ Course successfully pushed and verified in Notion!");
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored --test-threads=1
async fn test_push_course_title_update() {
    dotenvy::dotenv().ok();

    let config = NotionConfig::new_from_env().expect("Failed to load Notion config");
    let notion = Arc::new(NotionHttpClient::new(config).expect("Failed to create Notion client"));

    // Use a real page ID from your Notion database
    // This test assumes you have a test course in your Notion database
    let test_page_id = "d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4";

    let updated_course = Course {
        id: test_page_id.to_string(),
        title: format!("Updated Title - {}", chrono::Utc::now().timestamp()),
        semester: "Summer".to_string(),
        day_of_week: "Wednesday".to_string(),
        period: 3,
        room: Some("Updated Room 202".to_string()),
        instructor: Some("Updated Professor".to_string()),
        is_archived: false,
        updated_at: chrono::Utc::now().to_rfc3339(),
        sync_state: "pending".to_string(),
        last_synced_at: None,
    };

    // Push update to Notion
    let result = notion.push_course(&updated_course).await;
    println!("Update result: {:?}", result);
    assert!(result.is_ok(), "Failed to update course in Notion");

    // Verify the update
    let courses = notion.fetch_courses().await.expect("Failed to fetch courses");
    let fetched = courses
        .iter()
        .find(|c| c.id == test_page_id)
        .expect("Updated course not found");

    assert_eq!(fetched.title, updated_course.title, "Title not updated");
    assert_eq!(fetched.semester, updated_course.semester, "Semester not updated");
    println!("✓ Course update successfully verified!");
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored --test-threads=1
async fn test_fetch_and_verify_courses_from_notion() {
    dotenvy::dotenv().ok();

    let config = NotionConfig::new_from_env().expect("Failed to load Notion config");
    let notion = Arc::new(NotionHttpClient::new(config).expect("Failed to create Notion client"));

    // Fetch all courses
    let courses = notion.fetch_courses().await.expect("Failed to fetch courses");
    println!("Fetched {} courses from Notion", courses.len());

    // Print all courses for inspection
    for course in &courses {
        println!(
            "ID: {}, Title: {}, Semester: {}, Day: {}, Period: {}, Room: {}, Instructor: {}",
            course.id,
            course.title,
            course.semester,
            course.day_of_week,
            course.period,
            course.room.as_deref().unwrap_or("N/A"),
            course.instructor.as_deref().unwrap_or("N/A")
        );
    }

    assert!(!courses.is_empty(), "No courses found in Notion");
    
    // Verify structure
    for course in courses {
        assert!(!course.id.is_empty(), "Course ID should not be empty");
        // Skip courses with empty titles (they may be drafts or test pages)
        if !course.title.is_empty() {
            assert!(!course.semester.is_empty(), "Course semester should not be empty");
            assert!(!course.day_of_week.is_empty(), "Course day_of_week should not be empty");
        }
    }

    println!("✓ All courses verified!");
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored --test-threads=1
async fn test_push_and_pull_roundtrip() {
    dotenvy::dotenv().ok();

    let db = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create database");

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
    .execute(&db)
    .await
    .expect("Failed to create courses table");

    let config = NotionConfig::new_from_env().expect("Failed to load Notion config");
    let notion = Arc::new(NotionHttpClient::new(config).expect("Failed to create Notion client"));

    // Step 1: Fetch from Notion
    let courses = notion.fetch_courses().await.expect("Failed to fetch");
    println!("Step 1: Fetched {} courses from Notion", courses.len());

    // Step 2: Store in local DB
    for course in &courses {
        sqlx::query(
            r#"
            INSERT INTO courses (id, title, semester, day_of_week, period, room, instructor, is_archived, updated_at, sync_state)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
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
        .bind("synced")
        .execute(&db)
        .await
        .expect("Failed to insert");
    }
    println!("Step 2: Stored {} courses in local DB", courses.len());

    // Step 3: Modify a course locally
    if !courses.is_empty() {
        let mut modified = courses[0].clone();
        modified.title = format!("Modified - {}", chrono::Utc::now().timestamp());
        modified.instructor = Some("New Instructor".to_string());

        // Step 4: Push back to Notion
        let result = notion.push_course(&modified).await;
        println!("Step 4: Pushed modified course - {:?}", result);
        assert!(result.is_ok(), "Failed to push modified course");

        // Step 5: Fetch again and verify
        let courses_after = notion.fetch_courses().await.expect("Failed to fetch after push");
        let verified = courses_after
            .iter()
            .find(|c| c.id == modified.id)
            .expect("Modified course not found after push");

        println!(
            "Step 5: Verified - Title: {}, Instructor: {}",
            verified.title,
            verified.instructor.as_deref().unwrap_or("N/A")
        );

        assert_eq!(verified.title, modified.title, "Title not persisted");
        println!("✓ Roundtrip test successful!");
    }
}
