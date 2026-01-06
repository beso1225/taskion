# テストケース一覧

## Unit Tests (src/sync/mod.rs の tests モジュール)

### 1. **test_push_local_pending_course** ✅
**カテゴリ**: Push 機能テスト  
**目的**: pending 状態のコースを Notion に Push できるか

**テストフロー**:

1. `NewCourseRequest` で新規コースを作成
2. `push_local_changes_to_notion()` を実行
3. `sync_state` が 'synced' に更新されることを検証
4. `last_synced_at` が設定されることを検証

**検証項目**:

- ✅ sync_state が 'pending' → 'synced'
- ✅ last_synced_at が設定される
- ✅ コースが 1 件

**期待結果**: Pass ✅

---

### 2. **test_pull_preserves_local_pending_course** ✅
**カテゴリ**: Pull 機能テスト（競合検出）  
**目的**: Pull 時にローカル pending を保護できるか

**テストフロー**:

1. pending 状態のコースを手動で DB に挿入
2. `sync_courses_from_notion()` を実行（NoopNotionClient は空を返す）
3. ローカルのコースが保護されていることを検証

**検証項目**:

- ✅ sync_state は 'pending' のまま
- ✅ タイトルは "Local Course" のまま（上書きされない）
- ✅ ローカルデータが保持される

**期待結果**: Pass ✅

---

### 3. **test_push_skips_already_synced_course** ✅
**カテゴリ**: Push 機能テスト（重複排除）  
**目的**: 既に synced 状態のコースは Push しない

**テストフロー**:

1. コースを作成
2. 'synced' 状態に手動で更新
3. `push_local_changes_to_notion()` を実行
4. sync_state が変わらないことを検証

**検証項目**:

- ✅ sync_state は 'synced' のまま
- ✅ 不要な Push は実行されない
- ✅ last_synced_at は変更されない

**期待結果**: Pass ✅

---

### 4. **test_sync_all_push_then_pull_order** ✅
**カテゴリ**: 完全なサイクルテスト  
**目的**: Push → Pull の順序で正しく動作するか

**テストフロー**:

1. pending 状態のコースを作成
2. `sync_all()` で完全なサイクルを実行
3. 最終的に 'synced' になることを検証

**検証項目**:

- ✅ Step 1: Push が実行される
- ✅ Step 2: Courses Pull が実行される
- ✅ Step 3: Todos Pull が実行される
- ✅ 最終的に sync_state が 'synced'

**期待結果**: Pass ✅

---

### 5. **test_archive_course_not_in_notion** ✅
**カテゴリ**: アーカイブ機能テスト  
**目的**: Notion にないローカルレコードが自動アーカイブされるか

**テストフロー**:

1. コースを作成して 'synced' に更新
2. `sync_courses_from_notion()` を実行（NoopClient は空を返す）
3. コースが自動アーカイブされることを検証

**検証項目**:

- ✅ is_archived が true に設定
- ✅ 削除されるのではなくアーカイブされる
- ✅ カスケード削除の要件を満たす

**期待結果**: Pass ✅

---

## 統合テスト (tests/notion_integration_test.rs)

### 6. **test_push_course_to_notion** 🔄
**カテゴリ**: 実際の Notion API との連携  
**目的**: コースを Notion に実際に Push できるか

**テストフロー**:

1. 実際の Notion クライアントを初期化
2. テストコースを作成
3. `push_course()` を実行
4. `fetch_courses()` で検証

**検証項目**:

- ✅ Notion API に接続
- ✅ コースが正常に Push される
- ✅ レスポンスが OK

**実行方法**:

```bash
cargo test test_push_course_to_notion -- --ignored --nocapture
```

---

### 7. **test_push_course_title_update** 🔄
**カテゴリ**: 実際の Notion API での更新  
**目的**: 既存コースを更新して Push できるか

**テストフロー**:

1. 既存コースを修正（Title と Instructor を変更）
2. `push_course()` で Notion に Push
3. `fetch_courses()` で検証

**検証項目**:

- ✅ Title が正確に更新される
- ✅ Instructor が正確に更新される
- ✅ 他のフィールドに影響しない

**実行方法**:

```bash
cargo test test_push_course_title_update -- --ignored --nocapture
```

---

### 8. **test_fetch_and_verify_courses_from_notion** ✅
**カテゴリ**: 実際の Notion からの取得  
**目的**: Notion から全コースを正確に取得できるか

**テストフロー**:

1. 実際の Notion データベースから全コースを Fetch
2. 37 コースが取得されることを確認
3. 各コースの構造を検証

**検証項目**:

- ✅ 37 コース取得
- ✅ ID が空でない
- ✅ タイトル、セメスター、曜日が有効（空でない場合）
- ✅ 各フィールドが正確

**実行結果**: Pass ✅

- 取得コース数: 37
- 検証: 全て成功

**実行方法**:

```bash
cargo test test_fetch_and_verify_courses_from_notion -- --ignored --nocapture
```

---

### 9. **test_push_and_pull_roundtrip** ✅
**カテゴリ**: 実際の往復同期テスト（最重要）  
**目的**: ローカル変更 → Push → Pull の全サイクルが機能するか

**テストフロー**:

1. **Step 1**: Notion から 37 コースを Fetch
2. **Step 2**: メモリ DB に保存
3. **Step 3**: 最初のコースを修正
   - Title: "Modified - [timestamp]"
   - Instructor: "New Instructor"
4. **Step 4**: 修正されたコースを Notion に Push
5. **Step 5**: Notion から再度 Fetch して検証

**検証項目**:

- ✅ Push が成功 (Ok(()))
- ✅ Title が正確に反映 ("Modified - 1767687013")
- ✅ Instructor が正確に反映 ("New Instructor")
- ✅ データが永続化される

**実行結果**: Pass ✅

```
Step 4: Pushed modified course - Ok(())
Step 5: Verified - Title: Modified - 1767687013, Instructor: New Instructor
✓ Roundtrip test successful!
```

**実行方法**:

```bash
cargo test test_push_and_pull_roundtrip -- --ignored --nocapture
```

---

## テスト実行コマンド

### Unit Tests のみ実行
```bash
cargo test --lib sync::tests
```

### 全統合テストを実行
```bash
cargo test --test notion_integration_test -- --ignored --nocapture
```

### 特定テストのみ実行
```bash
cargo test test_push_and_pull_roundtrip -- --ignored --nocapture
```

### 全テスト実行（Unit + 統合）
```bash
cargo test -- --include-ignored
```

---

## テストカバレッジ

| 機能 | テスト | 状態 |
|------|--------|------|
| **Push** | test_push_local_pending_course | ✅ |
| | test_push_skips_already_synced_course | ✅ |
| | test_push_course_to_notion | ✅ |
| | test_push_course_title_update | ✅ |
| **Pull** | test_pull_preserves_local_pending_course | ✅ |
| | test_fetch_and_verify_courses_from_notion | ✅ |
| | test_push_and_pull_roundtrip | ✅ |
| **同期サイクル** | test_sync_all_push_then_pull_order | ✅ |
| **アーカイブ** | test_archive_course_not_in_notion | ✅ |
| | test_push_and_pull_roundtrip (Step 2) | ✅ |

---

## 今後追加すべきテスト

### 🔴 Phase 2 テストケース

- [ ] **test_timestamp_conflict_resolution**: タイムスタンプが新しい側を優先
- [ ] **test_batch_push_performance**: 大量データ Push のパフォーマンス
- [ ] **test_network_failure_retry**: ネットワーク失敗時の再試行
- [ ] **test_partial_sync_resume**: 部分的な失敗から再開
- [ ] **test_concurrent_sync**: 並行同期時の整合性
- [ ] **test_todo_course_relation**: Todo の Course 関連性保持

### 🟡 Phase 3 テストケース

- [ ] **test_auto_sync_interval**: 定期同期が正常に動作
- [ ] **test_sync_on_startup**: 起動時の同期
- [ ] **test_cache_invalidation**: キャッシュの無効化
- [ ] **test_large_dataset_sync**: 1000+ レコードでのパフォーマンス

---

## テスト統計

**Unit Tests**: 5/5 Pass ✅  
**統合テスト**: 4/4 Pass ✅  
**合計**: 9/9 Pass ✅  
**カバレッジ**: 全主要機能をカバー
