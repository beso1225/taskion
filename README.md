# Taskion

Taskion is a native TODO app for Apple devices (iOS, macOS, watchOS),
synchronized with a Notion database.

## Concept

- Notion is a single source of truth
- Fast, offline-first native UI
- Designed for students managing courses and assignments
- Apple Watch support for quick completion

## Architecture

Taskion uses a local-first architecture with Notion as the single source of truth.
The system is composed of three main layers: client, backend, and storage.

### High-Level Architecture

The following diagram shows how data flows through the system:

```text
    +--------------------------------------------------+
    |                SwiftUI Client                    |
    |                                                  |
    |  iOS / macOS / watchOS                           |
    |                                                  |
    |  - Displays courses and todos                    |
    |  - Handles user interactions                     |
    |  - Works offline                                 |
    +------------------------+-------------------------+
                             |
                             | HTTP (localhost)
                             |
    +------------------------v-------------------------+
    |              Rust Sync Engine                    |
    |                                                  |
    |  axum + tokio                                    |
    |                                                  |
    |  - REST API for clients                          |
    |  - Sync logic                                    |
    |  - Conflict resolution                           |
    +------------------------+-------------------------+
                             |
                             | SQL
                             |
    +------------------------v-------------------------+
    |              Local SQLite Database               |
    |                                                  |
    |  - courses                                       |
    |  - todos                                         |
    |                                                  |
    |  Local cache and offline storage                 |
    +------------------------+-------------------------+
                             |
                             | HTTPS (Notion API)
                             |
    +------------------------v-------------------------+
    |                 Notion                           |
    |                                                  |
    |  - Courses database                              |
    |  - Tasks (Todos) database                        |
    |                                                  |
    |  Single Source of Truth                          |
    +--------------------------------------------------+
```

## Responsibilities

Each layer has a clear responsibility.

### SwiftUI Client

- Renders the user interface
- Provides fast, native interactions
- Displays courses and todos
- Handles user interactions
- Works offline
- Sends user actions to the local backend
- Never communicates with Notion directly

### Rust Sync Engine

- Exposes a local REST API
- Acts as the central synchronization layer
- Stores data in a local SQLite database
- Synchronizes data with Notion
- Resolves conflicts between local and remote changes

### Notion

- Stores authoritative data
- Manages relations between courses and todos
- Acts as the single source of truth

---

## Data Model Overview

There are two core entities in Taskion.

### Course

- Represents a class or lecture
- Stored in a dedicated Notion database
- Contains metadata such as course name and semester

### Todo

- Represents an individual task or assignment
- Stored in a separate Notion database
- Always belongs to exactly one Course

### Relationship

```text
Course
  |
  | 1-to-many
  v
Todo
```

- A Course can have many Todos
- Each Todo references one Course
- In Notion, this is implemented using a Relation property
- Locally, this is implemented using a foreign key `course_id`

## Synchronization Flow

Synchronization is bidirectional and asynchronous.

### Notion to Local

1. Fetch courses from the Notion Courses database
2. Update or create local course records
3. Fetch todos from the Notion Tasks database
4. Resolve course relations
5. Update or create local todo records

Courses are always synchronized before todos.

### Local to Notion

1. Apply user changes to the local database immediately
2. Schedule a synchronization task
3. Push local changes to Notion
4. Update relation properties if necessary

---

## Conflict Resolution

- Each entity has a last-updated timestamp
- When conflicts occur, the most recently updated version wins
- This simple strategy keeps the system predictable and debuggable

---

## Offline Behavior

- All user actions are first applied locally
- The app remains usable without network access
- Pending changes are synchronized once connectivity is restored

---

## Security and Scope

- The backend only listens on `localhost`
- Notion API tokens are stored in environment variables
- No external servers or cloud services are required

## Design Documents

Detailed design documents are available in the `docs/` directory:

- Architecture
- Data model
- Synchronization strategy
- Roadmap
