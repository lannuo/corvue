//! TagMemo memory store integration
//!
//! Provides persistent storage for TagMemo memories.

use corvus_memory::tagmemo::TagMemoMemory;
use corvus_core::memory::{MemoryItem, ContentType, MemorySystem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A stored memory with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMemory {
    /// The memory item
    pub item: MemoryItem,
    /// Tags associated with this memory
    pub tags: Vec<String>,
    /// When this memory was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this memory was last accessed
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// Access count
    pub access_count: u64,
}

impl StoredMemory {
    /// Create a new stored memory from a memory item
    pub fn new(item: MemoryItem) -> Self {
        let now = chrono::Utc::now();
        Self {
            item,
            tags: Vec::new(),
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Add tags to this memory
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Record an access
    pub fn record_access(&mut self) {
        self.last_accessed = chrono::Utc::now();
        self.access_count += 1;
    }
}

/// TagMemo memory store manager
pub struct TagMemoStore {
    /// The TagMemo memory system
    tagmemo: Arc<Mutex<TagMemoMemory>>,
    /// Stored memories (id -> StoredMemory)
    memories: Arc<Mutex<HashMap<String, StoredMemory>>>,
    /// Storage path (if using persistent storage)
    storage_path: Option<PathBuf>,
}

impl TagMemoStore {
    /// Create a new TagMemo store with in-memory storage
    pub fn new(embedding_dim: usize) -> anyhow::Result<Self> {
        let tagmemo = TagMemoMemory::with_in_memory_storage(embedding_dim)?;

        Ok(Self {
            tagmemo: Arc::new(Mutex::new(tagmemo)),
            memories: Arc::new(Mutex::new(HashMap::new())),
            storage_path: None,
        })
    }

    /// Open a TagMemo store with persistent storage
    pub fn open<P: Into<PathBuf>>(path: P, embedding_dim: usize) -> anyhow::Result<Self> {
        let path_buf = path.into();
        let tagmemo = TagMemoMemory::with_storage(embedding_dim, &path_buf)?;

        Ok(Self {
            tagmemo: Arc::new(Mutex::new(tagmemo)),
            memories: Arc::new(Mutex::new(HashMap::new())),
            storage_path: Some(path_buf),
        })
    }

    /// Open the default TagMemo store (persistent)
    pub fn open_default() -> anyhow::Result<Self> {
        let path = Self::default_storage_path()?;
        Self::open(path, 128)
    }

    /// Get the default storage path
    fn default_storage_path() -> anyhow::Result<PathBuf> {
        let mut path = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        path.push(".corvus");
        path.push("memory.db");
        Ok(path)
    }

    /// Load memories from storage
    pub async fn load_memories(&self) -> anyhow::Result<()> {
        let tagmemo = self.tagmemo.lock().await;
        tagmemo.load_memories_from_storage().await?;
        Ok(())
    }

    /// Get the storage path (if using persistent storage)
    pub fn storage_path(&self) -> Option<&PathBuf> {
        self.storage_path.as_ref()
    }

    /// Add a memory to the store
    pub async fn add_memory(
        &self,
        content: String,
        content_type: ContentType,
        tags: Vec<String>,
    ) -> anyhow::Result<String> {
        let mut item = MemoryItem::new(content, content_type)
            .with_tags(tags.clone());

        // Store in TagMemo
        let id = {
            let tagmemo = self.tagmemo.lock().await;
            tagmemo.store(item.clone()).await?
        };

        // Update item with ID
        item.id = Some(id.clone());

        // Store in local map
        let stored = StoredMemory::new(item).with_tags(tags);
        self.memories.lock().await.insert(id.clone(), stored);

        Ok(id)
    }

    /// Get a memory by ID
    pub async fn get_memory(&self, memory_id: &str) -> Option<StoredMemory> {
        let mut memories = self.memories.lock().await;
        if let Some(stored) = memories.get_mut(memory_id) {
            stored.record_access();
            Some(stored.clone())
        } else {
            None
        }
    }

    /// List recent memories
    pub async fn list_memories(&self, limit: usize) -> Vec<StoredMemory> {
        let memories = self.memories.lock().await;
        let mut items: Vec<_> = memories.values().cloned().collect();

        // Sort by last accessed (newest first)
        items.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        items.into_iter().take(limit).collect()
    }

    /// Search memories by content
    pub async fn search_memories(&self, query: &str, limit: usize) -> Vec<StoredMemory> {
        let tagmemo = self.tagmemo.lock().await;

        // Add query tags to activate
        let query_tags: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        for tag in &query_tags {
            tagmemo.add_tag(tag.clone(), false, None);
        }

        // Propagate wave
        let wave_result = tagmemo.propagate_wave(&query_tags, 0.8);
        let activated_tags = tagmemo.get_activated_tags(&wave_result);

        // Get memories with matching tags
        let memories = self.memories.lock().await;
        let mut results: Vec<_> = memories
            .values()
            .filter(|m| {
                // Check if any activated tag matches
                for (tag, _score) in &activated_tags {
                    if m.tags.contains(tag) || m.item.content.to_lowercase().contains(&tag.to_lowercase()) {
                        return true;
                    }
                }
                // Also check direct content match
                m.item.content.to_lowercase().contains(&query.to_lowercase())
            })
            .cloned()
            .collect();

        // Sort by relevance (simplified)
        results.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        results.into_iter().take(limit).collect()
    }

    /// Delete a memory
    pub async fn delete_memory(&self, memory_id: &str) -> bool {
        let mut memories = self.memories.lock().await;
        memories.remove(memory_id).is_some()
    }

    /// Get all tags
    pub async fn all_tags(&self) -> Vec<String> {
        let tagmemo = self.tagmemo.lock().await;
        tagmemo.all_tags().into_iter().map(|t| t.tag).collect()
    }

    /// Add a tag
    pub async fn add_tag(&self, tag: String, is_core: bool) {
        let tagmemo = self.tagmemo.lock().await;
        tagmemo.add_tag(tag, is_core, None);
    }

    /// Associate tags
    pub async fn associate_tags(&self, tag1: &str, tag2: &str, weight: f32) {
        let tagmemo = self.tagmemo.lock().await;
        tagmemo.associate_tags(tag1, tag2, weight);
    }
}
