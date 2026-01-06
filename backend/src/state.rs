use std::sync::Arc;

use sqlx::SqlitePool;

use crate::notion::NotionClient;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub notion: Arc<dyn NotionClient>,
}