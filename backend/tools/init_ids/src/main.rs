use dotenvy::dotenv;
use reqwest::Client;
use serde_json::Value;
use std::env;
use serde::Deserialize;

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

    let pages = fetch_all_pages(&client, &token, &courses_db_id).await?;

    println!("Total pages fetched: {}", pages.len());

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