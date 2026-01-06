mod error;
mod models;
mod db;
mod api;
mod state;
mod notion;
mod services;

use std::net::SocketAddr;
use std::sync::Arc;
use sqlx::sqlite::SqlitePoolOptions;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::router;
use crate::state::AppState;
use crate::notion::{NotionClient, NoopNotionClient, NotionConfig, NotionHttpClient};
use crate::services::SyncScheduler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://taskion.db".to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let notion_client: Arc<dyn NotionClient> = match NotionConfig::new_from_env() {
        Ok(cfg) => Arc::new(NotionHttpClient::new(cfg)?),
        Err(e) => {
            warn!("Notion config missing or invalid: {}. Falling back to Noop client.", e);
            Arc::new(NoopNotionClient)
        }
    };
    let state = AppState { db: pool.clone(), notion: notion_client.clone() };

    // Auto-sync scheduler を環境変数で設定可能にする
    let sync_interval_secs = std::env::var("SYNC_INTERVAL_SECS")
        .unwrap_or_else(|_| "300".to_string()) // デフォルト: 5分
        .parse::<u64>()
        .unwrap_or(300);

    // Auto-sync をバックグラウンドで実行
    let scheduler = SyncScheduler::new(pool.clone(), notion_client, sync_interval_secs);
    tokio::spawn(async move {
        scheduler.start().await;
    });

    let app = router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}