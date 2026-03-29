//! Simple in-memory memory implementation

use corvus_core::error::{MemoryError, Result};
use corvus_core::memory::*;
use crate::vector::KnnIndex;
use std::collections::HashMap;
use std::sync::Mutex;

/// A simple in-memory memory system with vector search
pub struct InMemoryMemory {
    items: Mutex<HashMap<String, MemoryItem>>,
    vector_index: Mutex<KnnIndex>,
}

impl InMemoryMemory {
    /// Create a new empty in-memory memory
    pub fn new() -> Self {
        Self {
            items: Mutex::new(HashMap::new()),
            vector_index: Mutex::new(KnnIndex::new()),
        }
    }

    /// Generate a unique ID
    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("mem_{:x}", nanos)
    }
}

impl Default for InMemoryMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MemorySystem for InMemoryMemory {
    async fn store(&self, mut item: MemoryItem) -> Result<String> {
        let id = item.id.clone().unwrap_or_else(Self::generate_id);
        item.id = Some(id.clone());

        // Add to vector index if embedding is available
        if let Some(embedding) = &item.embedding {
            let mut vector_index = self.vector_index.lock().unwrap();
            let mut metadata = HashMap::new();
            metadata.insert("id".to_string(), id.clone());
            vector_index.add(id.clone(), embedding.clone(), metadata);
        }

        let mut items = self.items.lock().unwrap();
        items.insert(id.clone(), item);

        Ok(id)
    }

    async fn retrieve(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>> {
        let items = self.items.lock().unwrap();

        // If we have a query embedding, use vector search for better results
        if let Some(query_embedding) = &query.embedding {
            let mut vector_index = self.vector_index.lock().unwrap();

            // Get similar IDs from vector index
            let similar_ids = vector_index.search(query_embedding, query.limit * 2);

            let mut results = Vec::new();
            for (id, similarity) in similar_ids {
                if let Some(item) = items.get(&id) {
                    // Apply additional filters
                    let passes_filters =
                        // Content type filter
                        (query.content_types.is_empty() || query.content_types.contains(&item.content_type)) &&
                        // Tags filter
                        (query.tags.is_empty() || query.tags.iter().any(|t| item.tags.contains(t))) &&
                        // Time range filter
                        query.time_range.as_ref().is_none_or(|(start, end)| {
                            item.timestamp >= *start && item.timestamp <= *end
                        });

                    if passes_filters {
                        results.push(MemoryResult {
                            item: item.clone(),
                            score: (similarity as f64 + 1.0) / 2.0, // Normalize to 0-1
                            explanation: Some("Vector similarity search".to_string()),
                        });
                    }
                }
            }

            results.truncate(query.limit);
            return Ok(results);
        }

        // Fallback to traditional search
        let mut results: Vec<MemoryResult> = items
            .values()
            .filter(|item| {
                // Filter by content types
                if !query.content_types.is_empty() && !query.content_types.contains(&item.content_type) {
                    return false;
                }

                // Filter by tags
                if !query.tags.is_empty() {
                    let has_tag = query.tags.iter().any(|t| item.tags.contains(t));
                    if !has_tag {
                        return false;
                    }
                }

                // Filter by time range
                if let Some((start, end)) = &query.time_range {
                    if item.timestamp < *start || item.timestamp > *end {
                        return false;
                    }
                }

                true
            })
            .map(|item| {
                // Calculate score based on query type
                let score = if let Some(text) = &query.text {
                    // Simple text matching
                    let text_lower = text.to_lowercase();
                    let item_lower = item.content.to_lowercase();
                    if item_lower.contains(&text_lower) {
                        0.8
                    } else {
                        0.1
                    }
                } else {
                    0.5
                };

                MemoryResult {
                    item: item.clone(),
                    score,
                    explanation: None,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        results.truncate(query.limit);

        Ok(results)
    }

    async fn tag(&self, item_id: &str, tags: Vec<String>) -> Result<()> {
        let mut items = self.items.lock().unwrap();

        let item = items
            .get_mut(item_id)
            .ok_or_else(|| MemoryError::ItemNotFound(item_id.to_string()))?;

        for tag in tags {
            if !item.tags.contains(&tag) {
                item.tags.push(tag);
            }
        }

        Ok(())
    }

    async fn search_by_tags(&self, tags: Vec<String>) -> Result<Vec<MemoryItem>> {
        let items = self.items.lock().unwrap();

        Ok(items
            .values()
            .filter(|item| tags.iter().any(|t| item.tags.contains(t)))
            .cloned()
            .collect())
    }

    async fn get(&self, item_id: &str) -> Result<MemoryItem> {
        let items = self.items.lock().unwrap();

        items
            .get(item_id)
            .cloned()
            .ok_or_else(|| MemoryError::ItemNotFound(item_id.to_string()).into())
    }

    async fn delete(&self, item_id: &str) -> Result<()> {
        // Remove from vector index
        let mut vector_index = self.vector_index.lock().unwrap();
        vector_index.remove(item_id);
        drop(vector_index);

        let mut items = self.items.lock().unwrap();

        items
            .remove(item_id)
            .ok_or_else(|| MemoryError::ItemNotFound(item_id.to_string()))?;

        Ok(())
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryItem>> {
        let items = self.items.lock().unwrap();

        let mut items: Vec<_> = items.values().cloned().collect();
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        items.truncate(limit);

        Ok(items)
    }
}
