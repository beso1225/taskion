use dotenvy::dotenv;
use reqwest::Client;
use serde_json::Value;
use std::env;
use serde::Deserialize;

fn is_dry_run() -> bool {
    !std::env::args().any(|a| a == "--apply")
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    results: Vec<Page>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Page {
    id: String,
    properties: Value,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let token = env::var("NOTION_TOKEN")?;
    let courses_db_id = env::var("COURSES_DB_ID")?;

    let client = Client::new();

    let course_pages = fetch_all_pages(&client, &token, &courses_db_id).await?;

    let dry_run = is_dry_run();

    let mut course_updated = 0;

    for page in &course_pages {
        if is_id_missing(&page.properties, "course_id") {
            let new_id = uuid::Uuid::new_v4().to_string();

            if dry_run {
                println!(
                    "[DRY RUN] Would update page {} -> {}",
                    page.id, new_id
                );
            } else {
                update_page_id(&client, &token, &page.id, "course_id", &new_id).await?;
                println!("Updated page {} -> {}", page.id, new_id);
            }

            course_updated += 1;
        }
    }

    println!(
        "Courses updated: {} / {}",
        course_updated,
        course_pages.len()
    );



    let todos_db_id = env::var("TODOS_DB_ID")?;
    let todo_pages = fetch_all_pages(&client, &token, &todos_db_id).await?;

    let mut todo_updated = 0;

    for page in &todo_pages {
        if is_id_missing(&page.properties, "todo_id") {
            let new_id = uuid::Uuid::new_v4().to_string();

            if dry_run {
                println!(
                    "[DRY RUN] Would update todo {} -> {}",
                    page.id, new_id
                );
            } else {
                update_page_id(
                    &client,
                    &token,
                    &page.id,
                    "todo_id",
                    &new_id,
                )
                .await?;
                println!("Updated todo {} -> {}", page.id, new_id);
            }

            todo_updated += 1;
        }
    }

    println!(
        "Todos updated: {} / {}",
        todo_updated,
        todo_pages.len()
    );


    Ok(())
}

async fn fetch_all_pages(
    client: &Client,
    token: &str,
    db_id: &str,
) -> Result<Vec<Page>, Box<dyn std::error::Error>> {
    let mut pages = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut body = serde_json::json!({});
        if let Some(c) = &cursor {
            body["start_cursor"] = serde_json::json!(c);
        }

        let url = format!("https://api.notion.com/v1/databases/{}/query", db_id);
        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Notion-Version", "2022-06-28")
            .json(&body)
            .send()
            .await?;

        let data: QueryResponse = res.json().await?;

        pages.extend(data.results);

        if !data.has_more {
            break;
        }

        cursor = data.next_cursor;
    }

    Ok(pages)
}

fn is_id_missing(properties: &Value, key: &str) -> bool {
    let prop = match properties.get(key) {
        Some(p) => p,
        None => return true,
    };

    if prop.get("type").and_then(|t| t.as_str()) != Some("rich_text") {
        return true;
    }

    let rich_text = match prop.get("rich_text").and_then(|v| v.as_array()) {
        Some(v) => v,
        None => return true,
    };

    rich_text.is_empty()
}

async fn update_page_id(
    client: &reqwest::Client,
    token: &str,
    page_id: &str,
    prop_name: &str,
    new_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("https://api.notion.com/v1/pages/{}", page_id);

    let body = serde_json::json!({
        "properties": {
            prop_name: {
                "rich_text": [
                    { "text": { "content": new_id } }
                ]
            }
        }
    });

    client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Notion-Version", "2022-06-28")
        .json(&body)
        .send()
        .await?
        .error_for_status()?; // 失敗したら即エラー

    Ok(())
}