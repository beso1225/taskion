use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct QueryDatabaseResponse {
    pub results: Vec<Page>,
    pub has_more: bool,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub id: String,
    pub properties: HashMap<String, Property>,
    pub created_time: String,
    pub last_edited_time: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Property {
    Title { title: Vec<RichText> },
    RichText { rich_text: Vec<RichText> },
    Number { number: Option<f64> },
    Select { select: Option<SelectOption> },
    MultiSelect { multi_select: Vec<SelectOption> },
    Date { date: Option<DateValue> },
    Checkbox { checkbox: bool },
    Relation { relation: Vec<Relation> },
    Url { url: Option<String> },
    LastEditedTime { last_edited_time: String },
    Status { status: Option<SelectOption> },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct RichText {
    pub plain_text: String,
}

#[derive(Debug, Deserialize)]
pub struct SelectOption {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct DateValue {
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Relation {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct QueryDatabaseRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorts: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UpdatePageRequest {
    pub properties: serde_json::Value,
}