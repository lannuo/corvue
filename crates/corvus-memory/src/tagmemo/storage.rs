//! TagMemo storage backend
//!
//! SQLite-based persistent storage for TagMemo memories, tags,
//! and co-occurrence data.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::wave::{TagNode, TagEdge, EdgeType};

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
                activation REAL NOT NULL DEFAULT 0,
                membrane_potential REAL NOT NULL DEFAULT -70,
                last_spike_ms INTEGER,
                embedding BLOB,
                metadata BLOB,
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
            "CREATE TABLE IF NOT EXISTS tag_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1,
                cooccurrence_count INTEGER NOT NULL DEFAULT 1,
                edge_type TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES tags(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES tags(id) ON DELETE CASCADE,
                UNIQUE(source_id, target_id)
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Initialize schema version if not exists
        conn.execute(
            "INSERT OR IGNORE INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![1, Self::now()],
        )?;

        // Indexes
        conn.execute("CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_memory_tags_memory ON memory_tags(memory_id)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_memory_tags_tag ON memory_tags(tag_id)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tag_edges_source ON tag_edges(source_id)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tag_edges_target ON tag_edges(target_id)", [])?;

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

    // ===== TagNode and TagEdge persistence =====

    /// Save a tag node
    pub fn save_tag_node(&mut self, node: &TagNode) -> anyhow::Result<()> {
        let embedding_bytes = node.embedding.as_ref().and_then(|e| serde_json::to_vec(e).ok());
        let metadata_bytes = serde_json::to_vec(&node.metadata)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO tags
             (id, name, is_core, activation, membrane_potential, last_spike_ms, embedding, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                node.id,
                node.tag,
                node.is_core,
                node.activation,
                node.membrane_potential,
                node.last_spike_ms,
                embedding_bytes,
                metadata_bytes,
                Self::now(),
            ],
        )?;

        Ok(())
    }

    /// Load all tag nodes
    pub fn load_tag_nodes(&mut self) -> anyhow::Result<Vec<TagNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, is_core, activation, membrane_potential, last_spike_ms, embedding, metadata
             FROM tags",
        )?;

        let nodes = stmt.query_map([], |row| {
            let embedding_bytes: Option<Vec<u8>> = row.get(6)?;
            let metadata_bytes: Vec<u8> = row.get(7)?;

            let embedding = embedding_bytes.and_then(|b| serde_json::from_slice(&b).ok());
            let metadata: HashMap<String, String> = serde_json::from_slice(&metadata_bytes).unwrap_or_default();

            Ok(TagNode {
                id: row.get(0)?,
                tag: row.get(1)?,
                is_core: row.get(2)?,
                activation: row.get(3)?,
                membrane_potential: row.get(4)?,
                last_spike_ms: row.get(5)?,
                embedding,
                metadata,
            })
        })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(nodes)
    }

    /// Save a tag edge
    pub fn save_tag_edge(&mut self, edge: &TagEdge) -> anyhow::Result<()> {
        let edge_type_str = match edge.edge_type {
            EdgeType::Semantic => "semantic",
            EdgeType::Temporal => "temporal",
            EdgeType::Hierarchical => "hierarchical",
            EdgeType::Associative => "associative",
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO tag_edges
             (source_id, target_id, weight, cooccurrence_count, edge_type)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                edge.source,
                edge.target,
                edge.weight,
                edge.cooccurrence_count,
                edge_type_str,
            ],
        )?;

        Ok(())
    }

    /// Load all tag edges
    pub fn load_tag_edges(&mut self) -> anyhow::Result<Vec<TagEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id, target_id, weight, cooccurrence_count, edge_type
             FROM tag_edges",
        )?;

        let edges = stmt.query_map([], |row| {
            let edge_type_str: String = row.get(4)?;
            let edge_type = match edge_type_str.as_str() {
                "semantic" => EdgeType::Semantic,
                "temporal" => EdgeType::Temporal,
                "hierarchical" => EdgeType::Hierarchical,
                _ => EdgeType::Associative,
            };

            Ok(TagEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                weight: row.get(2)?,
                cooccurrence_count: row.get(3)?,
                edge_type,
            })
        })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(edges)
    }

    /// Save co-occurrence matrix entry
    pub fn save_cooccurrence(&mut self, tag1_id: &str, tag2_id: &str, count: u64) -> anyhow::Result<()> {
        let (t1, t2) = if tag1_id < tag2_id {
            (tag1_id, tag2_id)
        } else {
            (tag2_id, tag1_id)
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO tag_cooccurrences (tag1_id, tag2_id, count, last_occurred)
             VALUES (?1, ?2, ?3, ?4)",
            params![t1, t2, count, Self::now()],
        )?;

        Ok(())
    }

    /// Load all co-occurrences
    pub fn load_cooccurrences(&mut self) -> anyhow::Result<HashMap<(String, String), u64>> {
        let mut stmt = self.conn.prepare(
            "SELECT tag1_id, tag2_id, count FROM tag_cooccurrences",
        )?;

        let mut cooccurrences = HashMap::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let t1: String = row.get(0)?;
            let t2: String = row.get(1)?;
            let count: u64 = row.get(2)?;
            cooccurrences.insert((t1, t2), count);
        }

        Ok(cooccurrences)
    }

    // ===== Database migration =====

    /// Get current schema version
    pub fn get_schema_version(&mut self) -> anyhow::Result<i32> {
        let version = self.conn.query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        ).optional()?;

        Ok(version.unwrap_or(0))
    }

    /// Migrate database to latest schema
    pub fn migrate(&mut self) -> anyhow::Result<()> {
        let current_version = self.get_schema_version()?;
        let target_version = 1;

        if current_version < target_version {
            // Add future migrations here
            self.conn.execute(
                "UPDATE schema_version SET version = ?1, applied_at = ?2 WHERE version = ?3",
                params![target_version, Self::now(), current_version],
            )?;
        }

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
