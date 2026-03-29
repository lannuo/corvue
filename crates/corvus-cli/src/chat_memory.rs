//! Chat memory integration
//!
//! Provides integration between chat mode and TagMemo memory system.

use crate::memory_store::TagMemoStore;
use crate::memory_store::StoredMemory;
use corvus_core::memory::ContentType;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Chat memory integration manager
#[derive(Clone)]
pub struct ChatMemory {
    /// The TagMemo store
    store: Arc<Mutex<TagMemoStore>>,
    /// Memory relevance threshold
    threshold: f32,
    /// Maximum memories to retrieve
    max_memories: usize,
    /// Total memories saved
    total_saved: u64,
    /// Total memories retrieved
    total_retrieved: u64,
}

impl ChatMemory {
    /// Create a new chat memory manager
    pub fn new(store: TagMemoStore, threshold: f32, max_memories: usize) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            threshold: threshold.clamp(0.0, 1.0),
            max_memories: max_memories.max(1),
            total_saved: 0,
            total_retrieved: 0,
        }
    }

    /// Create a chat memory manager with default store
    pub fn open_default(threshold: f32, max_memories: usize) -> anyhow::Result<Self> {
        let store = TagMemoStore::open_default()?;
        Ok(Self::new(store, threshold, max_memories))
    }

    /// Save a user message to memory
    pub async fn save_user_message(&mut self, content: &str) -> anyhow::Result<String> {
        let tags = Self::extract_tags(content);
        let store = self.store.lock().await;
        let id = store.add_memory(
            content.to_string(),
            ContentType::Text,
            tags,
        ).await?;
        self.total_saved += 1;
        Ok(id)
    }

    /// Save an assistant message to memory
    pub async fn save_assistant_message(&mut self, content: &str) -> anyhow::Result<String> {
        let tags = Self::extract_tags(content);
        let store = self.store.lock().await;
        let id = store.add_memory(
            content.to_string(),
            ContentType::Text,
            tags,
        ).await?;
        self.total_saved += 1;
        Ok(id)
    }

    /// Retrieve relevant memories for a query
    pub async fn retrieve_memories(&mut self, query: &str) -> anyhow::Result<Vec<StoredMemory>> {
        let store = self.store.lock().await;
        let memories = store.search_memories(query, self.max_memories).await;

        // Filter by threshold if we had scores (simplified for now)
        // In a real implementation, we'd check the relevance score
        self.total_retrieved += memories.len() as u64;

        Ok(memories)
    }

    /// Build a context string from relevant memories
    pub async fn build_memory_context(&mut self, query: &str) -> anyhow::Result<String> {
        let memories = self.retrieve_memories(query).await?;

        if memories.is_empty() {
            return Ok(String::new());
        }

        let mut context = String::from("\n=== Relevant Memories ===\n");
        for (i, memory) in memories.iter().enumerate() {
            context.push_str(&format!("\n[Memory {}]\n", i + 1));
            context.push_str(&memory.item.content);
            context.push('\n');
        }
        context.push_str("\n=== End Memories ===\n");

        Ok(context)
    }

    /// Inject memories into a prompt
    pub async fn inject_memories(&mut self, prompt: &str) -> anyhow::Result<String> {
        let memory_context = self.build_memory_context(prompt).await?;

        if memory_context.is_empty() {
            Ok(prompt.to_string())
        } else {
            Ok(format!("{}\n\n{}", memory_context, prompt))
        }
    }

    /// Get memory usage statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_saved: self.total_saved,
            total_retrieved: self.total_retrieved,
            threshold: self.threshold,
            max_memories: self.max_memories,
        }
    }

    /// Load memories from storage
    pub async fn load_memories(&mut self) -> anyhow::Result<()> {
        let store = self.store.lock().await;
        store.load_memories().await?;
        Ok(())
    }

    /// Extract simple tags from content (keyword extraction)
    fn extract_tags(content: &str) -> Vec<String> {
        let stop_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to",
            "for", "of", "with", "by", "from", "as", "is", "was", "are",
            "were", "be", "been", "being", "have", "has", "had", "do",
            "does", "did", "will", "would", "could", "should", "may",
            "might", "must", "shall", "can", "need", "dare", "ought",
            "used", "it", "its", "it's", "this", "that", "these", "those",
            "i", "me", "my", "mine", "you", "your", "yours", "he", "him",
            "his", "she", "her", "hers", "we", "us", "our", "ours", "they",
            "them", "their", "theirs", "what", "which", "who", "whom", "whose",
            "where", "when", "why", "how", "all", "each", "every", "both",
            "few", "more", "most", "other", "some", "such", "no", "nor",
            "not", "only", "own", "same", "so", "than", "too", "very", "just",
            "also", "now", "here", "there", "then", "once", "if", "about",
            "into", "through", "during", "before", "after", "above", "below",
            "between", "under", "again", "further", "then", "once", "help",
            "hello", "hi", "hey",
        ];

        content
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .filter(|s| !s.is_empty() && s.len() > 2 && !stop_words.contains(s))
            .map(|s| s.to_string())
            .take(10)
            .collect()
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memories saved
    pub total_saved: u64,
    /// Total memories retrieved
    pub total_retrieved: u64,
    /// Relevance threshold
    pub threshold: f32,
    /// Maximum memories per retrieval
    pub max_memories: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags() {
        let content = "Hello, how can I help you with Rust programming today?";
        let tags = ChatMemory::extract_tags(content);

        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
        assert!(!tags.contains(&"hello".to_string()));
    }

    #[test]
    fn test_extract_tags_empty() {
        let content = "the a an and or but";
        let tags = ChatMemory::extract_tags(content);

        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_chat_memory_creation() -> anyhow::Result<()> {
        let store = TagMemoStore::new(128)?;
        let memory = ChatMemory::new(store, 0.5, 10);

        assert_eq!(memory.threshold, 0.5);
        assert_eq!(memory.max_memories, 10);
        assert_eq!(memory.total_saved, 0);
        assert_eq!(memory.total_retrieved, 0);

        Ok(())
    }
}
