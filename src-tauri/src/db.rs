// SQLite database module for task history and settings persistence
// Uses rusqlite with bundled SQLite (no system dependency)

use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

/// Initialize the database, run migrations, store connection globally
pub fn init(db_path: &str) -> anyhow::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = PathBuf::from(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    run_migrations(&conn)?;

    DB.set(Mutex::new(conn))
        .map_err(|_| anyhow::anyhow!("DB already initialized"))?;
    Ok(())
}

fn conn() -> &'static Mutex<Connection> {
    DB.get().expect("DB not initialized. Call db::init() first.")
}

// ============================================================
// Schema migrations
// ============================================================

fn run_migrations(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER NOT NULL DEFAULT 0,
            file_format TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending',
            engine TEXT NOT NULL DEFAULT 'fast',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS transcriptions (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL UNIQUE REFERENCES tasks(id) ON DELETE CASCADE,
            engine TEXT NOT NULL,
            full_text TEXT NOT NULL DEFAULT '',
            duration_secs REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS segments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            start_time REAL NOT NULL,
            end_time REAL NOT NULL,
            text TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );"
    )
}

// ============================================================
// Data types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub file_size: i64,
    pub file_format: String,
    pub status: String,
    pub engine: String,
    pub created_at: String,
    pub updated_at: String,
    pub result: Option<TranscriptionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionRecord {
    pub task_id: String,
    pub engine: String,
    pub full_text: String,
    pub duration_secs: f64,
    pub created_at: String,
    pub segments: Vec<SegmentRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentRecord {
    pub start_time: f64,
    pub end_time: f64,
    pub text: String,
}

// ============================================================
// CRUD: Tasks
// ============================================================

pub fn list_tasks(limit: i64, offset: i64) -> anyhow::Result<Vec<TaskRecord>> {
    let c = conn().lock().unwrap();
    let mut stmt = c.prepare(
        "SELECT t.id, t.name, t.file_path, t.file_size, t.file_format, t.status, t.engine, t.created_at, t.updated_at,
                tr.task_id, tr.engine, tr.full_text, tr.duration_secs, tr.created_at
         FROM tasks t
         LEFT JOIN transcriptions tr ON t.id = tr.task_id
         ORDER BY t.created_at DESC
         LIMIT ?1 OFFSET ?2"
    )?;

    let rows = stmt.query_map(params![limit, offset], |row| {
        let task_id: String = row.get(0)?;
        let tr_task_id: Option<String> = row.get(9)?;
        let has_transcription = tr_task_id.is_some();

        Ok(TaskRecord {
            id: task_id,
            name: row.get(1)?,
            file_path: row.get(2)?,
            file_size: row.get(3)?,
            file_format: row.get(4)?,
            status: row.get(5)?,
            engine: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            result: if has_transcription {
                Some(TranscriptionRecord {
                    task_id: tr_task_id.unwrap_or_default(),
                    engine: row.get(10)?,
                    full_text: row.get(11)?,
                    duration_secs: row.get(12)?,
                    created_at: row.get(13)?,
                    segments: vec![],
                })
            } else {
                None
            },
        })
    })?;

    let mut tasks: Vec<TaskRecord> = Vec::new();
    for row in rows {
        tasks.push(row?);
    }
    Ok(tasks)
}

pub fn delete_task(id: &str) -> anyhow::Result<()> {
    let c = conn().lock().unwrap();
    c.execute("DELETE FROM segments WHERE task_id=?1", params![id])?;
    c.execute("DELETE FROM transcriptions WHERE task_id=?1", params![id])?;
    c.execute("DELETE FROM tasks WHERE id=?1", params![id])?;
    Ok(())
}

// ============================================================
// CRUD: Transcriptions
// ============================================================

// ============================================================
// Settings
// ============================================================

