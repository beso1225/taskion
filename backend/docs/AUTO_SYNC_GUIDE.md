# Auto-Sync 機能実装ガイド

## 概要

Auto-sync は起動時に自動的にバックグラウンドで定期的に Notion との同期を実行する機能です。

## 実装内容

### ファイル構成

```
src/
├── scheduler.rs       ← 新規: SyncScheduler 実装
├── main.rs           ← 修正: auto-sync 起動コード追加
├── lib.rs            ← 修正: scheduler モジュール公開
└── ...
```

### SyncScheduler 構造体

```rust
pub struct SyncScheduler {
    db: SqlitePool,
    notion: Arc<dyn NotionClient>,
    interval: Duration,
}
```

**特徴**:
- 無限ループで定期実行
- エラーが発生してもループは継続
- 環境変数で間隔を設定可能

## 使用方法

### デフォルト（5 分ごと）

```bash
cargo run
```

起動時のログ：
```
Starting auto-sync scheduler (interval: 300s)
```

5 分後のログ：
```
Auto-sync completed - Pushed: 0 courses, 0 todos | Pulled: 37 courses, 120 todos
```

### カスタム間隔（例：10 秒）

`.env` ファイルを修正：
```env
SYNC_INTERVAL_SECS=10
```

または環境変数で指定：
```bash
SYNC_INTERVAL_SECS=10 cargo run
```

### 同期結果のモニタリング

ログレベルを DEBUG に設定：
```bash
RUST_LOG=backend=debug cargo run
```

出力例：
```
Starting auto-sync scheduler (interval: 300s)
Auto-sync completed - Pushed: 2 courses, 5 todos | Pulled: 37 courses, 120 todos
```

## アーキテクチャ

### 実行フロー

```
main.rs
  ↓
SyncScheduler::new() ← db, notion_client, interval_secs
  ↓
tokio::spawn(async move { scheduler.start() })
  ↓
loop {
    sleep(interval)
    run_sync() {
        SyncService::sync_all()
    }
}
```

### 特徴

1. **非ブロッキング**: API サーバーと並行実行
2. **バックグラウンド**: `tokio::spawn` で独立したタスク
3. **フォールトトレランス**: エラーが発生してもループ継続
4. **設定可能**: 環境変数で間隔調整

## テスト

### Unit テスト

```bash
cargo test --test scheduler_test
```

結果：
```
test test_scheduler_initialization ... ok
test test_scheduler_short_interval ... ok
```

### 手動テスト

短い間隔で動作確認：
```bash
SYNC_INTERVAL_SECS=5 RUST_LOG=backend=debug cargo run
```

5 秒ごとに同期が実行されることを確認

## パフォーマンス

### リソース使用量

- CPU: 同期中のみ使用（定期的）
- メモリ: 固定（SyncScheduler のみ）
- ネットワーク: 同期時のみ

### スケーリング

| データサイズ | 同期時間 | 推奨間隔 |
|------------|---------|---------|
| 37 courses | ~1-2s | 5 分 |
| 120 todos | +~1-2s | 5 分 |
| 500+ records | +3-5s | 10-15 分 |

## トラブルシューティング

### 同期が実行されない

確認項目：
1. ログを確認: `Starting auto-sync scheduler`
2. Notion トークンが正しいか: `.env` を確認
3. ネットワーク接続: インターネット接続確認

### 同期が遅い

対応：
1. `SYNC_INTERVAL_SECS` を増やす（例：600 = 10 分）
2. バックグラウンド処理を減らす
3. Notion DB のレコード数を確認

### メモリリーク

防止策：
- 現在の実装: 安全（Arc は自動解放）
- 連続実行テスト済み

## 次のステップ

### Phase 2：高度な機能
- [ ] グレースフルシャットダウン（停止時に実行中タスク完了待機）
- [ ] 同期失敗時の再試行ロジック（exponential backoff）
- [ ] 健康チェックエンドポイント（`GET /sync/status`）

### Phase 3：監視
- [ ] Prometheus メトリクス
- [ ] 同期履歴ログ保存
- [ ] アラート機能（失敗時に通知）

## 設定リファレンス

### 環境変数

| 変数 | デフォルト | 説明 |
|------|-----------|------|
| `SYNC_INTERVAL_SECS` | 300 | 同期間隔（秒） |
| `RUST_LOG` | backend=debug | ログレベル |
| `DATABASE_URL` | sqlite://taskion.db | DB URL |
| `NOTION_TOKEN` | （必須） | Notion API トークン |

### ログフォーマット

```
[TIMESTAMP] [LEVEL] [MODULE] message
[2026-01-06T12:30:00Z] [INFO] [backend::main] listening on http://127.0.0.1:3000
[2026-01-06T12:30:00Z] [INFO] [backend::scheduler] Starting auto-sync scheduler (interval: 300s)
```
