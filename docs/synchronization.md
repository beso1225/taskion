# Synchronization and Conflict Resolution Strategy

This document defines how Taskion synchronizes data between the local SQLite database
and Notion, including conflict resolution rules and edge case handling.

---

## Overview

Taskion uses a **local-first, pull-based synchronization model**:

- **Local changes are applied immediately** to provide a responsive user experience
- **Periodic pulls from Notion** ensure the local cache stays up-to-date
- **Push operations** send local changes to Notion asynchronously
- **Notion is the single source of truth** - in case of conflicts, Notion's state prevails

---

## Synchronization Direction

### Pull (Notion → Local)

Pulls bring data from Notion into the local database. This is the **authoritative direction**.

**Trigger:**

- Startup (full sync)
- Periodic interval (default: 5 minutes)
- Manual trigger (user-initiated)

**Process:**

1. Fetch all courses from Notion Courses database
2. For each course:
   - If `is_archived = true` locally but `is_archived = false` in Notion → update to unarchived
   - If `is_archived = false` locally but `is_archived = true` in Notion → mark as archived locally
   - Otherwise, apply Notion's state (use `updated_at` for conflict resolution)
3. Fetch all todos from Notion Todos database
4. For each todo:
   - Verify the referenced course exists locally
   - If `is_archived = true` locally but `is_archived = false` in Notion → update to unarchived
   - Otherwise, apply Notion's state

**Conflict Resolution During Pull:**

- **Rule:** Notion version always wins in pull operations
- **Timestamp comparison:** If local `updated_at > Notion updated_at`, log a warning but still apply Notion's version
- **Rationale:** Notion is the authoritative source; local changes will be pushed in the next push cycle

### Push (Local → Notion)

Pushes send local changes to Notion. These are changes made locally that haven't been synced yet.

**Trigger:**

- After user modifies a todo locally (immediate scheduling)
- Periodic push cycle (default: 5 minutes)
- Manual trigger (user-initiated)

**Process:**

1. Query local database for records with `sync_state = pending`
2. For each pending record:
   - Fetch the current state from Notion
   - Compare timestamps: if Notion's `updated_at > local updated_at`, apply Notion's state first (conflict)
   - If no conflict, push the local change to Notion
   - Update `sync_state = synced` and `last_synced_at = now()`
3. If push fails, keep `sync_state = pending` and retry on next cycle

**Conflict Resolution During Push:**

- **Rule:** If Notion has been updated after our last pull, use Notion's version
- **Step back:** Fetch Notion's current state, merge with local, then re-push if needed
- **Logging:** Log all conflicts for debugging

---

## Synchronization Timing

### Startup Sync

**When the backend starts:**

1. Execute a full **Pull** from Notion
2. Load all courses and todos into local database
3. Mark sync as complete, set `last_pulled_at = now()`

**Expected behavior:**

- Client waits for sync to complete before showing data
- If sync fails, retry with exponential backoff (1s, 2s, 4s, max 30s)
- If all retries fail, show offline mode with cached data

### Periodic Sync

**Default interval:** 5 minutes (configurable)

**Behavior:**

- **Pull Phase:** Fetch from Notion, apply changes locally
- **Push Phase:** Push pending local changes to Notion
- Both phases run in sequence; push waits for pull to complete

**During periodic sync:**

- Client continues to work; new local changes are queued
- If sync is still running when next interval triggers, skip the interval
- Sync duration is logged for monitoring

### Manual Sync Trigger

**Initiated by:** User action (pull-to-refresh, sync button)

**Behavior:**

- Prioritize this sync over periodic cycles
- Execute both Pull and Push phases immediately
- Return status to client (success/failure, number of changes)

---

## Conflict Resolution Rules

### Timestamp-Based Resolution

Each record has:

- `updated_at` - Last modification timestamp (from Notion, in ISO 8601 UTC)
- `sync_state` - Local tracking: `pending`, `synced`, `conflict`
- `local_version` - Incremental version for local-only conflicts

### Rule 1: Pull-Time Conflicts

**Scenario:** Same record updated both locally and in Notion since last sync

**Resolution:**

```text
if Notion.updated_at > local.updated_at:
    # Notion is newer
    apply Notion version locally
    set sync_state = synced
else if Notion.updated_at < local.updated_at:
    # Local is newer (will be pushed in next push cycle)
    keep local version
    set sync_state = pending
else:
    # Same timestamp (extremely rare)
    # Use record ID as tiebreaker (deterministic)
    apply the version with lexicographically smaller ID
    log warning
```

### Rule 2: Push-Time Conflicts

**Scenario:** Attempting to push a local change, but Notion has been updated

**Resolution:**

```text
if Notion.updated_at > local.last_synced_at:
    # Notion was updated after our last sync
    conflict detected:
        1. Fetch Notion's current state
        2. Determine which fields changed locally vs in Notion
        3. For each field:
           - If only local changed: push to Notion
           - If only Notion changed: pull from Notion
           - If both changed: Notion wins (use Notion value)
        4. Update local record and set updated_at from Notion response
        5. Log the conflict
else:
    # Safe to push
    send local change to Notion
    update updated_at from response
    set sync_state = synced
```

### Rule 3: Deletion Conflicts (Logical Deletion)

**Scenario:** Record marked `is_archived = true` locally but `is_archived = false` in Notion, or vice versa

**Resolution:**

```text
During Pull:
    if local.is_archived != Notion.is_archived:
        apply Notion.is_archived value
        log if this contradicts sync_state = pending

During Push:
    if local.is_archived = true but Notion.is_archived = false:
        push is_archived = true to Notion
    if local.is_archived = false but Notion.is_archived = true:
        pull Notion's value (Notion always wins for deletions)
```

---

## Edge Cases and Special Handling

### Case 1: Simultaneous Edits

**Scenario:** User edits field A locally, and field B is edited in Notion

**Handling:**

- During next sync, detect that both have changed
- Treat as field-level merge (see Rule 2 above)
- Local value for A, Notion value for B
- Log this merge operation

### Case 2: Network Failure During Push

**Scenario:** Local change is made, push is attempted but fails

**Handling:**

- Keep `sync_state = pending`
- Retry on next periodic sync or manual trigger
- Client is informed of pending state via API response
- If pending for more than 24 hours, flag as stale and warn user

### Case 3: Offline Mode

**Scenario:** No network connectivity

**Handling:**

- Local changes proceed normally, marked `sync_state = pending`
- Pull operations are skipped; use cached data
- When connectivity returns, trigger full sync
- Push all pending changes
- Detect any conflicts that occurred while offline (Notion version wins)

### Case 4: Large Sync (Many Records)

**Scenario:** Notion database has thousands of records

**Handling:**

- Use cursor-based pagination (Notion API supports `start_cursor`)
- Fetch in batches (default: 100 records per request)
- Process and insert into local DB in transactions
- Track progress; allow cancellation via client
- Estimated sync time shown to client

### Case 5: Archived Course, Active Todos

**Scenario:** Course is marked archived in Notion, but todos still exist

**Handling:**

- During pull, sync courses first
- When syncing todos, check if parent course is archived
- If parent course is archived:
  - Mark all related todos as archived locally (cascade)
  - Log this action
- During push, prevent creating new todos for archived courses

---

## Sync State Machine

Each record has a `sync_state` field with possible values:

```text
┌──────────┐
│ pending  │  Local change waiting to be pushed
└─────┬────┘
      │ (push succeeds)
      v
┌──────────┐
│ synced   │  Synchronized with Notion
└─────┬────┘
      │ (pull detects Notion change)
      v
┌──────────┐
│ conflict │  Local and Notion both changed
└─────┬────┘
      │ (conflict resolved via rules)
      v
┌──────────┐
│ synced   │
└──────────┘

Special case: Failed push
  synced → pending (if push fails, revert)
```

---

## Logging and Observability

### Sync Events to Log

1. **Sync Start/Complete**
   - Timestamp, sync type (pull/push/both), duration
   - Records pulled, records pushed, conflicts detected

2. **Conflicts**
   - Record ID, type (pull/push/merge)
   - Local state, Notion state, resolution applied
   - Timestamp

3. **Errors**
   - Notion API errors (status, message)
   - Network timeouts
   - Database errors
   - Retry attempts

4. **Warnings**
   - Orphaned todos (no parent course)
   - Stale pending changes (> 24h)
   - Large sync operations (> 1000 records)

### Debug API Endpoint

Expose a debug endpoint to query sync status:

```text
GET /api/debug/sync-status

Response:
{
  "last_pull_at": "2026-01-02T10:30:00Z",
  "last_push_at": "2026-01-02T10:35:00Z",
  "pending_count": 3,
  "conflict_count": 0,
  "last_sync_duration_ms": 245,
  "next_sync_at": "2026-01-02T10:45:00Z"
}
```

---

## Summary

| Aspect | Decision |
| --- | --- |
| **Primary Direction** | Pull (Notion → Local) is authoritative |
| **Conflict Winner** | Notion's version (via timestamp comparison) |
| **Timing** | Startup + periodic (5 min) + manual |
| **Deletion** | Logical deletion via `is_archived` flag |
| **Offline** | Local-first; pending changes sync when online |
| **Cascade** | Archiving course archives related todos |

---

## Next Steps

- [ ] Implement sync state machine in backend
- [ ] Add `sync_state` and `last_synced_at` columns to local schema
- [ ] Implement Pull logic with conflict detection
- [ ] Implement Push logic with conflict resolution
- [ ] Add debug API endpoint for observability
- [ ] Write integration tests for each conflict scenario
