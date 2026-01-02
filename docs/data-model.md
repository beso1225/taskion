# Notion Database Schema（Final）

This document defines the final Notion database schemas used by Taskion.
Notion acts as the single source of truth. Deletions are handled via
logical deletion (`is_archived`).

---

## Courses Database（授業）

### Purpose (Courses)

- Represents a university course or lecture
- Acts as master data referenced by Todos
- Never physically deleted

---

### Properties (Courses)

| 表示名 | Type | Required | Notes |
| --- | --- | --- | --- |
| 授業名 | Title | ✓ | Notion title |
| セメスター | Select | ✓ | e.g. `2A2` |
| 曜日 | Select | ✓ | Mon / Tue / ... |
| 時限 | Select | ✓ | 1 / 2 / 3 / 4 / 5 / 6 |
| 教室 | Text | | Optional |
| 担当教員 | Text | | Optional |
| course_id | Text | ✓ | UUID, immutable |
| is_archived | Checkbox | ✓ | Logical deletion flag |
| updated_at | Last edited time | ✓ | For synchronization |

---

### Notes

- `course_id` is generated once and never changed
- `is_archived = true` means the course is no longer active
- Archived courses are excluded from normal views but kept for history

---

## Todos Database（課題）

### Purpose (Todos)

- Represents assignments or tasks
- Always belongs to exactly one Course
- Uses Status for progress tracking

---

### Properties (Todos)

| 表示名 | Type | Required | Notes |
| --- | --- | --- | --- |
| 課題名 | Title | ✓ | Notion title |
| 授業 | Relation → Courses | ✓ | Exactly one course |
| 締め切り | Date | ✓ | Date only |
| 進捗 | Status | ✓ | See below |
| todo_id | Text | ✓ | UUID, immutable |
| completed_at | Date | | Set when completed |
| is_archived | Checkbox | ✓ | Logical deletion flag |
| updated_at | Last edited time | ✓ | For synchronization |

---

### Status Definition

| Status | Meaning |
| --- | --- |
| 未着手 | Not started |
| 進行中 | In progress |
| 最終確認 | Review |
| 完了 | Done |

Rules:

- Completion is primarily determined by `進捗 = 完了`
- `completed_at` may be set automatically by the backend

---

## Relations

- Todos have a Relation property pointing to Courses
- One Course can have many Todos
- A Todo must reference exactly one Course

---

## Logical Deletion Strategy

Physical deletion is avoided due to limitations of the Notion API.

### Rules

- Deletion is represented by setting `is_archived = true`
- Archived items:
  - Are ignored by normal queries
  - Are not synchronized to clients as active items
  - Remain in Notion for history and recovery

### Sync Behavior

- `is_archived = true` is treated as a delete operation locally
- Local cache removes or hides archived records

---

## Synchronization Implications

- Courses are synchronized before Todos
- Archived Courses automatically imply archived Todos
- `updated_at` is used to resolve conflicts

---

## Summary

- Notion is the single source of truth
- UUID-based identifiers ensure stable synchronization
- Logical deletion keeps the system simple and robust

---
