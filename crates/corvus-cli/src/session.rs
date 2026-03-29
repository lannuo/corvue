//! Session history persistence
//!
//! SQLite-based persistent storage for chat sessions and messages.

use chrono::{DateTime, Utc};
use dirs::home_dir;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Unique session ID
    pub id: String,
    /// Session name/title
    pub name: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Message count
    pub message_count: u32,
}

/// A complete session export including messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    /// Session metadata
    pub session: ChatSession,
    /// Session messages
    pub messages: Vec<ChatMessage>,
}

/// A single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message ID
    pub id: String,
    /// Session ID this message belongs to
    pub session_id: String,
    /// Message role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Optional tool call JSON
    pub tool_calls: Option<String>,
    /// Optional tool response JSON
    pub tool_response: Option<String>,
}

/// Session storage manager
pub struct SessionStorage {
    conn: Connection,
}

impl SessionStorage {
    /// Get the default storage path
    pub fn default_path() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("corvus")
            .join("sessions.db")
    }

    /// Open or create storage at default path
    pub fn open_default() -> anyhow::Result<Self> {
        Self::open(Self::default_path())
    }

    /// Open or create storage at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path_ref)?;

        // SQLite performance optimizations
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA foreign_keys = ON;"
        )?;

        Self::initialize_tables(&conn)?;

        Ok(Self { conn })
    }

    /// Create in-memory storage for testing
    pub fn in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_tables(&conn)?;
        Ok(Self { conn })
    }

    /// Initialize database tables
    fn initialize_tables(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                tool_calls TEXT,
                tool_response TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp)", [])?;

        Ok(())
    }

    /// Create a new session
    pub fn create_session(&mut self, name: Option<String>) -> anyhow::Result<ChatSession> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let session_name = name.unwrap_or_else(|| format!("Session {}", now.format("%Y-%m-%d %H:%M")));

        self.conn.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at, message_count)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![id, session_name, now.timestamp(), now.timestamp()],
        )?;

        Ok(ChatSession {
            id,
            name: session_name,
            created_at: now,
            updated_at: now,
            message_count: 0,
        })
    }

    /// Get a session by ID
    pub fn get_session(&mut self, id: &str) -> anyhow::Result<Option<ChatSession>> {
        let row = self.conn.query_row(
            "SELECT id, name, created_at, updated_at, message_count
             FROM sessions WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, u32>(4)?,
                ))
            },
        ).optional()?;

        row.map(|(id, name, created_at, updated_at, message_count)| {
            Ok(ChatSession {
                id,
                name,
                created_at: DateTime::from_timestamp(created_at, 0).unwrap_or(Utc::now()),
                updated_at: DateTime::from_timestamp(updated_at, 0).unwrap_or(Utc::now()),
                message_count,
            })
        }).transpose()
    }

    /// List all sessions, most recent first
    pub fn list_sessions(&mut self, limit: Option<usize>) -> anyhow::Result<Vec<ChatSession>> {
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
        let query = format!(
            "SELECT id, name, created_at, updated_at, message_count
             FROM sessions ORDER BY updated_at DESC{}",
            limit_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let sessions = stmt.query_map([], |row: &rusqlite::Row<'_>| {
            Ok(ChatSession {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: DateTime::from_timestamp(row.get::<_, i64>(2)?, 0).unwrap_or(Utc::now()),
                updated_at: DateTime::from_timestamp(row.get::<_, i64>(3)?, 0).unwrap_or(Utc::now()),
                message_count: row.get(4)?,
            })
        })?
        .filter_map(|r: Result<ChatSession, _>| r.ok())
        .collect();

        Ok(sessions)
    }

    /// Update session name
    pub fn rename_session(&mut self, id: &str, name: &str) -> anyhow::Result<bool> {
        let now = Utc::now().timestamp();
        let changes = self.conn.execute(
            "UPDATE sessions SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, now, id],
        )?;
        Ok(changes > 0)
    }

    /// Delete a session
    pub fn delete_session(&mut self, id: &str) -> anyhow::Result<bool> {
        let changes = self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(changes > 0)
    }

    /// Add a message to a session
    pub fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> anyhow::Result<ChatMessage> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, tool_calls, tool_response)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)",
            params![id, session_id, role, content, now.timestamp()],
        )?;

        // Update session
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1, message_count = message_count + 1 WHERE id = ?2",
            params![now.timestamp(), session_id],
        )?;

        Ok(ChatMessage {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
            tool_calls: None,
            tool_response: None,
        })
    }

    /// Get all messages for a session
    pub fn get_messages(&mut self, session_id: &str) -> anyhow::Result<Vec<ChatMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, timestamp, tool_calls, tool_response
             FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC",
        )?;

        let messages = stmt.query_map(params![session_id], |row: &rusqlite::Row<'_>| {
            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: session_id.to_string(),
                role: row.get(1)?,
                content: row.get(2)?,
                timestamp: DateTime::from_timestamp(row.get::<_, i64>(3)?, 0).unwrap_or(Utc::now()),
                tool_calls: row.get(4)?,
                tool_response: row.get(5)?,
            })
        })?
        .filter_map(|r: Result<ChatMessage, _>| r.ok())
        .collect();

        Ok(messages)
    }

    /// Get the most recent session (if any)
    pub fn get_last_session(&mut self) -> anyhow::Result<Option<ChatSession>> {
        let sessions = self.list_sessions(Some(1))?;
        Ok(sessions.into_iter().next())
    }

    /// Search sessions by name or message content
    pub fn search_sessions(&mut self, query: &str, limit: usize) -> anyhow::Result<Vec<ChatSession>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT s.id, s.name, s.created_at, s.updated_at, s.message_count
             FROM sessions s
             LEFT JOIN messages m ON s.id = m.session_id
             WHERE s.name LIKE ?1 OR m.content LIKE ?1
             ORDER BY s.updated_at DESC
             LIMIT ?2",
        )?;

        let sessions = stmt.query_map(params![search_pattern, limit as i64], |row: &rusqlite::Row<'_>| {
            Ok(ChatSession {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: DateTime::from_timestamp(row.get::<_, i64>(2)?, 0).unwrap_or(Utc::now()),
                updated_at: DateTime::from_timestamp(row.get::<_, i64>(3)?, 0).unwrap_or(Utc::now()),
                message_count: row.get(4)?,
            })
        })?
        .filter_map(|r: Result<ChatSession, _>| r.ok())
        .collect();

        Ok(sessions)
    }

    /// Export a session to JSON
    pub fn export_session(&mut self, session_id: &str) -> anyhow::Result<SessionExport> {
        let session = self.get_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        let messages = self.get_messages(session_id)?;

        Ok(SessionExport {
            session,
            messages,
        })
    }

    /// Import a session from JSON
    pub fn import_session(&mut self, export: SessionExport) -> anyhow::Result<ChatSession> {
        let now = Utc::now();
        let new_id = uuid::Uuid::new_v4().to_string();

        // Insert the session with new ID
        self.conn.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at, message_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                new_id,
                export.session.name,
                export.session.created_at.timestamp(),
                now.timestamp(),
                export.session.message_count,
            ],
        )?;

        // Insert all messages with new IDs and new session ID
        for msg in export.messages {
            let msg_id = uuid::Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO messages (id, session_id, role, content, timestamp, tool_calls, tool_response)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    msg_id,
                    new_id,
                    msg.role,
                    msg.content,
                    msg.timestamp.timestamp(),
                    msg.tool_calls,
                    msg.tool_response,
                ],
            )?;
        }

        // Get the created session
        self.get_session(&new_id)?
            .ok_or_else(|| anyhow::anyhow!("Failed to create session"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_session() -> anyhow::Result<()> {
        let mut storage = SessionStorage::in_memory()?;

        let session = storage.create_session(Some("Test Session".to_string()))?;
        assert_eq!(session.name, "Test Session");

        let retrieved = storage.get_session(&session.id)?.unwrap();
        assert_eq!(retrieved.id, session.id);
        assert_eq!(retrieved.name, "Test Session");

        Ok(())
    }

    #[test]
    fn test_add_messages() -> anyhow::Result<()> {
        let mut storage = SessionStorage::in_memory()?;

        let session = storage.create_session(None)?;

        storage.add_message(&session.id, "user", "Hello!")?;
        storage.add_message(&session.id, "assistant", "Hi there!")?;

        let messages = storage.get_messages(&session.id)?;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");

        let updated_session = storage.get_session(&session.id)?.unwrap();
        assert_eq!(updated_session.message_count, 2);

        Ok(())
    }

    #[test]
    fn test_list_sessions() -> anyhow::Result<()> {
        let mut storage = SessionStorage::in_memory()?;

        storage.create_session(Some("Session 1".to_string()))?;
        storage.create_session(Some("Session 2".to_string()))?;

        let sessions = storage.list_sessions(None)?;
        assert_eq!(sessions.len(), 2);

        Ok(())
    }

    #[test]
    fn test_search_sessions() -> anyhow::Result<()> {
        let mut storage = SessionStorage::in_memory()?;

        let session1 = storage.create_session(Some("Rust Session".to_string()))?;
        storage.add_message(&session1.id, "user", "Rust is great!")?;

        let session2 = storage.create_session(Some("Python Session".to_string()))?;
        storage.add_message(&session2.id, "user", "Python is easy!")?;

        let results = storage.search_sessions("Rust", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, session1.id);

        Ok(())
    }
}
