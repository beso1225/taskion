# Backend Structure

```
src/
├── api/                      # REST API エンドポイント
│   └── mod.rs               # ルーター定義、ハンドラー実装
├── db/                      # データベース関連
│   ├── mod.rs              # db モジュール定義
│   └── repository.rs       # CRUD 操作（courses, todos）
├── models/                  # データモデル
│   ├── mod.rs              # モジュール定義
│   ├── course.rs           # Course, NewCourseRequest
│   └── todo.rs             # Todo, NewTodoRequest, UpdateTodoRequest
├── services/                # ビジネスロジック
│   ├── mod.rs              # サービスモジュール定義
│   ├── sync_service.rs     # SyncService, SyncStats (双方向同期ロジック)
│   └── scheduler.rs        # SyncScheduler (自動同期スケジューラー)
├── notion/                 # Notion API クライアント
│   ├── mod.rs              # NotionClient trait, 実装
│   └── dto.rs              # Notion API の DTO
├── error.rs                # エラーハンドリング (AppError, ErrorResponse)
├── state.rs                # アプリケーション状態管理 (AppState)
├── lib.rs                  # ライブラリ entry point
└── main.rs                 # サーバー entry point

docs/
├── AUTO_SYNC_GUIDE.md      # Auto-sync 機能のガイド
├── AUTO_SYNC_COMPLETE.md   # Auto-sync 実装サマリー
├── OPTIMIZATION.md         # 最適化記録
├── TEST_CASES.md           # テストケース
└── TEST_RESULTS.md         # テスト結果

tests/
├── fixtures/               # テストデータ
│   ├── notion_response.json
│   ├── notion_courses_response.json
│   └── notion_todos_response.json
├── sync_test.rs            # Sync テスト
├── scheduler_test.rs       # Scheduler テスト
└── repository_test.rs      # Repository テスト (lib.rs に含まれている場合あり)
```

## モジュール概要

### `api/mod.rs`
- REST API ルーター定義
- ハンドラー: `list_courses`, `create_course`, `list_todos`, `create_todo`, `update_todo`, `archive_todo`, `sync_now`
- 依存: `models`, `db::repository`, `services::SyncService`

### `db/repository.rs`
- CRUD 操作のリポジトリパターン実装
- 関数:
  - `fetch_courses()`, `insert_course()`, `find_course_by_id()`, `upsert_course()`
  - `fetch_todos()`, `insert_todo()`, `update_todo()`, `archive_todo()`, `find_todo_by_id()`, `upsert_todo()`
- 依存: `models`

### `models/{course,todo}.rs`
- データ定義
- `Course`, `NewCourseRequest`
- `Todo`, `NewTodoRequest`, `UpdateTodoRequest`

### `services/sync_service.rs`
- 双方向同期エンジン
- `SyncService::sync_all()` メソッド:
  1. Push: ローカル pending → Notion
  2. Pull: Notion → ローカル (競合検出)
  3. Archive: Notion に無いものをアーカイブ
- `SyncStats`: 同期統計

### `services/scheduler.rs`
- 自動同期スケジューラー
- `SyncScheduler::start()`: 定期実行のメイン loop
- 環境変数: `SYNC_INTERVAL_SECS` (秒単位、デフォルト: 300)

### `notion/mod.rs`
- Notion API クライアント trait 定義
- 実装: `NotionHttpClient`
- 機能: `fetch_courses()`, `fetch_todos()`, `push_course()`, `push_todo()`

### `error.rs`
- `AppError` enum: Database, NotFound, BadRequest, Conflict, InternalServerError
- `ErrorResponse`: JSON error response

### `state.rs`
- `AppState`: db pool, notion client の状態管理

## 使用方法

### API エンドポイント

```bash
# ヘルスチェック
GET /health

# コース操作
GET /courses
POST /courses
  { "title": "...", "semester": "...", "day_of_week": "...", "period": 1 }

# TODO 操作
GET /todos
POST /todos
  { "course_id": "...", "title": "...", "due_date": "2026-01-10", "status": "未着手" }
PATCH /todos/{id}
  { "title": "...", "due_date": "...", "status": "..." }
PATCH /todos/{id}/archive

# 同期操作
POST /sync
  → { "courses_pushed": 0, "courses_pulled": 37, ..., "todos_skipped": 5 }
```

### Auto-sync の実行

```bash
# デフォルト (5 分ごと)
cargo run

# カスタム間隔 (10 秒)
SYNC_INTERVAL_SECS=10 cargo run

# ログ出力
RUST_LOG=backend=debug cargo run
```

## テスト

```bash
# すべてのテスト実行
cargo test

# Scheduler テストのみ
cargo test --test scheduler_test

# ログ出力でテスト
RUST_LOG=backend=debug cargo test -- --nocapture
```

## 今後の改善予定

- [ ] グレースフルシャットダウン (実行中の同期完了待機)
- [ ] リトライロジック (exponential backoff)
- [ ] ヘルスチェック API (`GET /sync/status`)
- [ ] PostgreSQL マイグレーション
- [ ] Docker 化
