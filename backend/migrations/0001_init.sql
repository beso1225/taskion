-- courses
CREATE TABLE IF NOT EXISTS courses (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    semester TEXT NOT NULL,
    day_of_week TEXT NOT NULL,
    period INTEGER NOT NULL,
    room TEXT,
    instructor TEXT,
    is_archived INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    sync_state TEXT NOT NULL DEFAULT 'synced' CHECK (sync_state IN ('synced', 'pending', 'conflict')),
    last_synced_at TEXT
);

-- todos
CREATE TABLE IF NOT EXISTS todos (
    id TEXT PRIMARY KEY,
    course_id TEXT,
    title TEXT NOT NULL,
    due_date TEXT NOT NULL,
    status TEXT NOT NULL,
    completed_at TEXT,
    is_archived INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    sync_state TEXT NOT NULL DEFAULT 'pending' CHECK (sync_state IN ('pending','synced','conflict')),
    last_synced_at TEXT,
    FOREIGN KEY(course_id) REFERENCES courses(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_todos_course_id ON todos(course_id);
CREATE INDEX IF NOT EXISTS idx_todos_status ON todos(status);
CREATE INDEX IF NOT EXISTS idx_todos_sync_state ON todos(sync_state);
CREATE INDEX IF NOT EXISTS idx_courses_sync_state ON courses(sync_state);