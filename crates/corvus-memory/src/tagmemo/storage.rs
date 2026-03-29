//! TagMemo storage backend
//!
//! SQLite-based persistent storage for TagMemo memories, tags,
//! and co-occurrence data.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// A stored memory record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Unique memory ID
    pub id: String,
    /// Memory content/text
    pub content: String,
    /// Embedding vector (if available)
    pub embedding: Option<Vec<f32>>,
    /// Associated tags
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: i64,
    /// Last accessed timestamp
    pub last_accessed: i64,
    /// Access count
    pub access_count: u64,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// TagMemo SQLite storage
pub struct TagMemoStorage {
    conn: Connection,
    path: String,
}

impl TagMemoStorage {
    /// Create a new in-memory storage
    pub fn in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_tables(&conn)?;

        Ok(Self {
            conn,
            path: ":memory:".to_string(),
        })
    }

    /// Open or create a storage at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let conn = Connection::open(&path)?;
        Self::initialize_tables(&conn)?;

        Ok(Self {
            conn,
            path: path_str,
        })
    }

    /// Initialize database tables
    fn initialize_tables(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_accessed INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_embeddings (
                memory_id TEXT PRIMARY KEY,
                embedding BLOB,
                FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                is_core BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_tags (
                memory_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (memory_id, tag_id),
                FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tag_cooccurrences (
                tag1_id TEXT NOT NULL,
                tag2_id TEXT NOT NULL,
                count INTEGER NOT NULL DEFAULT 1,
                last_occurred INTEGER NOT NULL,
                PRIMARY KEY (tag1_id, tag2_id),
                FOREIGN KEY (tag1_id) REFERENCES tags(id) ON DELETE CASCADE,
                FOREIGN KEY (tag2_id) REFERENCES tags(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_metadata (
                memory_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (memory_id, key),
                FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Indexes
        conn.execute("CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_memory_tags_memory ON memory_tags(memory_id)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_memory_tags_tag ON memory_tags(tag_id)", [])?;

        Ok(())
    }

    /// Get current timestamp
    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    /// Store a memory
    pub fn store_memory(&mut self, record: MemoryRecord) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO memories (id, content, created_at, last_accessed, access_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.id, record.content, record.created_at, record.last_accessed, record.access_count],
        )?;

        // Store embedding
        if let Some(embedding) = &record.embedding {
            let embedding_bytes = serde_json::to_vec(embedding)?;
            self.conn.execute(
                "INSERT OR REPLACE INTO memory_embeddings (memory_id, embedding) VALUES (?1, ?2)",
                params![record.id, embedding_bytes],
            )?;
        }

        // Store tags
        for tag_name in &record.tags {
            let tag_id = self.get_or_create_tag_inner(tag_name, false)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO memory_tags (memory_id, tag_id) VALUES (?1, ?2)",
                params![record.id, tag_id],
            )?;
        }

        // Store metadata
        for (key, value) in &record.metadata {
            self.conn.execute(
                "INSERT OR REPLACE INTO memory_metadata (memory_id, key, value) VALUES (?1, ?2, ?3)",
                params![record.id, key, value],
            )?;
        }

        Ok(())
    }

    /// Get a memory by ID
    pub fn get_memory(&mut self, id: &str) -> anyhow::Result<Option<MemoryRecord>> {
        let row = self.conn.query_row(
            "SELECT id, content, created_at, last_accessed, access_count
             FROM memories WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, u64>(4)?,
                ))
            },
        ).optional()?;

        let (id, content, created_at, _last_accessed, access_count) = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        // Update access
        let now = Self::now();
        self.conn.execute(
            "UPDATE memories SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
            params![now, id],
        )?;

        // Get embedding
        let embedding = self.conn.query_row(
            "SELECT embedding FROM memory_embeddings WHERE memory_id = ?1",
            params![id],
            |row| row.get::<_, Vec<u8>>(0),
        ).optional()?;

        let embedding = embedding.and_then(|bytes| {
            serde_json::from_slice::<Vec<f32>>(&bytes).ok()
        });

        // Get tags
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM tags t
             JOIN memory_tags mt ON t.id = mt.tag_id
             WHERE mt.memory_id = ?1",
        )?;

        let tags: Vec<String> = stmt.query_map(params![id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Get metadata
        let mut stmt = self.conn.prepare(
            "SELECT key, value FROM memory_metadata WHERE memory_id = ?1",
        )?;

        let metadata: HashMap<String, String> = stmt.query_map(params![id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(Some(MemoryRecord {
            id,
            content,
            embedding,
            tags,
            created_at,
            last_accessed: now,
            access_count: access_count + 1,
            metadata,
        }))
    }

    /// Search memories by tag
    pub fn search_by_tag(&mut self, tag: &str, limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        let tag_id = match self.get_tag_id_inner(tag)? {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        let mut memory_ids = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT m.id FROM memories m
                 JOIN memory_tags mt ON m.id = mt.memory_id
                 WHERE mt.tag_id = ?1
                 ORDER BY m.last_accessed DESC
                 LIMIT ?2",
            )?;

            let mut rows = stmt.query(params![tag_id, limit as i64])?;
            while let Some(row) = rows.next()? {
                let id: String = row.get(0)?;
                memory_ids.push(id);
            }
        }

        let mut memories = Vec::new();
        for id in memory_ids {
            if let Some(memory) = self.get_memory(&id)? {
                memories.push(memory);
            }
        }

        Ok(memories)
    }

    /// Get or create a tag (internal)
    fn get_or_create_tag_inner(&mut self, name: &str, is_core: bool) -> anyhow::Result<String> {
        if let Some(id) = self.get_tag_id_inner(name)? {
            return Ok(id);
        }

        let id = format!("tag_{}", uuid::Uuid::new_v4());
        let now = Self::now();

        self.conn.execute(
            "INSERT INTO tags (id, name, is_core, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, is_core, now],
        )?;

        Ok(id)
    }

    /// Get or create a tag
    pub fn get_or_create_tag(&mut self, name: &str, is_core: bool) -> anyhow::Result<String> {
        self.get_or_create_tag_inner(name, is_core)
    }

    /// Get tag ID by name (internal)
    fn get_tag_id_inner(&mut self, name: &str) -> anyhow::Result<Option<String>> {
        let id = self.conn.query_row(
            "SELECT id FROM tags WHERE name = ?1",
            params![name],
            |row| row.get(0),
        ).optional()?;

        Ok(id)
    }

    /// Get tag ID by name
    pub fn get_tag_id(&mut self, name: &str) -> anyhow::Result<Option<String>> {
        self.get_tag_id_inner(name)
    }

    /// Record tag co-occurrence
    pub fn record_cooccurrence(&mut self, tag1: &str, tag2: &str) -> anyhow::Result<()> {
        let id1 = self.get_or_create_tag_inner(tag1, false)?;
        let id2 = self.get_or_create_tag_inner(tag2, false)?;

        let (tag1_id, tag2_id) = if id1 < id2 {
            (id1, id2)
        } else {
            (id2, id1)
        };

        let now = Self::now();

        self.conn.execute(
            "INSERT INTO tag_cooccurrences (tag1_id, tag2_id, count, last_occurred)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT (tag1_id, tag2_id) DO UPDATE SET
             count = count + 1, last_occurred = ?3",
            params![tag1_id, tag2_id, now],
        )?;

        Ok(())
    }

    /// List all tags
    pub fn list_tags(&mut self) -> anyhow::Result<Vec<(String, bool)>> {
        let mut stmt = self.conn.prepare("SELECT name, is_core FROM tags ORDER BY name")?;
        let tags = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tags)
    }

    /// Delete a memory
    pub fn delete_memory(&mut self, id: &str) -> anyhow::Result<bool> {
        let changes = self.conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(changes > 0)
    }

    /// Get recent memories
    pub fn get_recent_memories(&mut self, limit: usize) -> anyhow::Result<Vec<MemoryRecord>> {
        let mut ids: Vec<String> = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM memories ORDER BY last_accessed DESC LIMIT ?1",
            )?;

            let mut rows = stmt.query(params![limit as i64])?;
            while let Some(row) = rows.next()? {
                let id: String = row.get(0)?;
                ids.push(id);
            }
        }

        let mut memories = Vec::new();
        for id in ids {
            if let Some(memory) = self.get_memory(&id)? {
                memories.push(memory);
            }
        }

        Ok(memories)
    }

    /// Get storage path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Close connection
    pub fn close(self) -> anyhow::Result<()> {
        self.conn.close().map_err(|(_, e)| e)?;
        Ok(())
    }
}

impl MemoryRecord {
    /// Create a new memory record
    pub fn new(content: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            embedding: None,
            tags: Vec::new(),
            created_at: now,
            last_accessed: now,
            access_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Set embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_in_memory() -> anyhow::Result<()> {
        let mut storage = TagMemoStorage::in_memory()?;

        let mut record = MemoryRecord::new("Test memory content".to_string());
        record.add_tag("test".to_string());
        record.add_tag("example".to_string());

        storage.store_memory(record.clone())?;

        let retrieved = storage.get_memory(&record.id)?.unwrap();
        assert_eq!(retrieved.content, record.content);
        assert!(retrieved.tags.contains(&"test".to_string()));

        Ok(())
    }

    #[test]
    fn test_search_by_tag() -> anyhow::Result<()> {
        let mut storage = TagMemoStorage::in_memory()?;

        let mut record1 = MemoryRecord::new("Memory 1".to_string());
        record1.add_tag("rust".to_string());
        storage.store_memory(record1)?;

        let mut record2 = MemoryRecord::new("Memory 2".to_string());
        record2.add_tag("rust".to_string());
        record2.add_tag("ai".to_string());
        storage.store_memory(record2)?;

        let results = storage.search_by_tag("rust", 10)?;
        assert_eq!(results.len(), 2);

        Ok(())
    }
}
