use std::sync::Arc;
use std::time::Duration;
use sqlx::SqlitePool;
use tracing::info;

use crate::notion::NotionClient;
use crate::services::sync_service::SyncService;

/// Auto-sync スケジューラー
/// 定期的に Notion との同期を実行
pub struct SyncScheduler {
    db: SqlitePool,
    notion: Arc<dyn NotionClient>,
    interval: Duration,
}

impl SyncScheduler {
    pub fn new(
        db: SqlitePool,
        notion: Arc<dyn NotionClient>,
        interval_secs: u64,
    ) -> Self {
        Self {
            db,
            notion,
            interval: Duration::from_secs(interval_secs),
        }
    }

    /// 同期を無限ループで定期実行
    pub async fn start(self) {
        info!("Starting auto-sync scheduler (interval: {:?})", self.interval);
        
        loop {
            // 最初は指定時間待機
            tokio::time::sleep(self.interval).await;

            // 同期を実行
            match self.run_sync().await {
                Ok(stats) => {
                    info!(
                        "Auto-sync completed - Pushed: {} courses, {} todos | Pulled: {} courses, {} todos",
                        stats.courses_pushed,
                        stats.todos_pushed,
                        stats.courses_pulled,
                        stats.todos_pulled
                    );
                }
                Err(e) => {
                    tracing::warn!("Auto-sync failed: {:?}", e);
                    // エラーが発生してもループは継続
                }
            }
        }
    }

    /// 同期を実行
    async fn run_sync(&self) -> Result<crate::services::SyncStats, crate::error::AppError> {
        let service = SyncService::new(self.db.clone(), self.notion.clone());
        service.sync_all().await
    }
}
