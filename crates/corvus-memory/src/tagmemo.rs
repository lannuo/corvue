//! TagMemo V7 cognitive memory system
//!
//! This module provides the complete TagMemo V7 "Wave" cognitive
//! memory implementation, integrating EPA analysis, Residual Pyramid
//! decomposition, TagMemo Wave propagation, and SQLite storage.

pub mod epa;
pub mod pyramid;
pub mod storage;
pub mod wave;

pub use epa::{EpaAnalysis, EpaModule};
pub use pyramid::{PyramidLevel, ResidualPyramid};
pub use storage::{MemoryRecord, TagMemoStorage};
pub use wave::{EdgeType, LifParams, SpikePropagation, TagEdge, TagMemoWave, TagNode, WaveParams, WaveQueryResult};

use anyhow::Result;
use corvus_core::error::{MemoryError, Result as CorvusResult};
use corvus_core::memory::*;
use ndarray::Array1;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Complete TagMemo V7 cognitive memory system
pub struct TagMemoMemory {
    /// EPA module for embedding analysis
    epa: Arc<Mutex<EpaModule>>,
    /// Residual Pyramid for multi-scale analysis
    pyramid: Arc<Mutex<ResidualPyramid>>,
    /// TagMemo Wave network
    wave: Arc<Mutex<TagMemoWave>>,
    /// In-memory storage for memory items
    items: Arc<Mutex<HashMap<String, MemoryItem>>>,
    /// Persistent storage (optional)
    storage: Arc<Mutex<Option<TagMemoStorage>>>,
}

impl TagMemoMemory {
    /// Create a new in-memory TagMemo system
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            epa: Arc::new(Mutex::new(EpaModule::new(embedding_dim))),
            pyramid: Arc::new(Mutex::new(ResidualPyramid::new(5, embedding_dim))),
            wave: Arc::new(Mutex::new(TagMemoWave::new())),
            items: Arc::new(Mutex::new(HashMap::new())),
            storage: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a TagMemo system with persistent storage
    pub fn with_storage<P: AsRef<Path>>(embedding_dim: usize, path: P) -> Result<Self> {
        let mut storage = TagMemoStorage::open(path)?;
        storage.migrate()?;

        // Load existing data
        let nodes = storage.load_tag_nodes()?;
        let edges = storage.load_tag_edges()?;
        let cooccurrences = storage.load_cooccurrences()?;

        let mut wave = TagMemoWave::new();

        // Restore nodes
        for node in nodes {
            let node_id = node.id.clone();
            let tag = node.tag.clone();
            wave.nodes.insert(node_id.clone(), node);
            wave.tag_to_id.insert(tag, node_id);
        }

        // Restore edges
        for edge in edges {
            wave.outgoing_edges.entry(edge.source.clone()).or_default().push(edge.clone());
            wave.incoming_edges.entry(edge.target.clone()).or_default().push(edge);
        }

        // Restore co-occurrences
        wave.cooccurrence = cooccurrences;

        // Update next_node_id
        if let Some(max_id) = wave.nodes.keys().filter_map(|k| -> Option<u64> {
            k.strip_prefix("tag_").and_then(|s| s.parse::<u64>().ok())
        }).max() {
            wave.next_node_id = max_id + 1;
        }

        Ok(Self {
            epa: Arc::new(Mutex::new(EpaModule::new(embedding_dim))),
            pyramid: Arc::new(Mutex::new(ResidualPyramid::new(5, embedding_dim))),
            wave: Arc::new(Mutex::new(wave)),
            items: Arc::new(Mutex::new(HashMap::new())),
            storage: Arc::new(Mutex::new(Some(storage))),
        })
    }

    /// Create a TagMemo system with in-memory storage
    pub fn with_in_memory_storage(embedding_dim: usize) -> Result<Self> {
        Ok(Self::new(embedding_dim))
    }

    /// Save current state to persistent storage
    pub fn save_to_storage(&self) -> Result<()> {
        let mut storage_opt = self.storage.lock().unwrap();
        let Some(storage) = storage_opt.as_mut() else {
            return Ok(());
        };

        // Save tag nodes
        let wave = self.wave.lock().unwrap();
        for node in wave.nodes.values() {
            storage.save_tag_node(node)?;
        }

        // Save edges
        for edges in wave.outgoing_edges.values() {
            for edge in edges {
                storage.save_tag_edge(edge)?;
            }
        }

        // Save co-occurrences
        for ((t1, t2), count) in &wave.cooccurrence {
            storage.save_cooccurrence(t1, t2, *count)?;
        }

        Ok(())
    }

    /// Load memories from persistent storage (if available)
    pub async fn load_memories_from_storage(&self) -> CorvusResult<()> {
        let mut storage_opt = self.storage.lock().unwrap();
        let Some(storage) = storage_opt.as_mut() else {
            return Ok(());
        };

        let records = storage.get_recent_memories(10000)?;
        let mut items = self.items.lock().unwrap();

        for record in records {
            use chrono::TimeZone;
            let timestamp = chrono::Utc.timestamp_opt(record.created_at, 0).single().unwrap_or(chrono::Utc::now());
            let mut memory_item = MemoryItem::new(record.content, ContentType::Text)
                .with_tags(record.tags);
            memory_item.id = Some(record.id.clone());
            memory_item.timestamp = timestamp;
            items.insert(record.id, memory_item);
        }

        Ok(())
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

    /// Analyze an embedding with EPA
    pub fn analyze_embedding(&self, embedding: &[f32]) -> EpaAnalysis {
        let array = Array1::from(embedding.to_vec());
        self.epa.lock().unwrap().analyze(&array)
    }

    /// Decompose an embedding with Residual Pyramid
    pub fn decompose_embedding(&self, embedding: &[f32]) -> Result<Vec<PyramidLevel>> {
        let array = Array1::from(embedding.to_vec());
        let mut pyramid = self.pyramid.lock().unwrap();
        let levels = pyramid.decompose(&array)?;
        Ok(levels.to_vec())
    }

    /// Propagate a wave from tags
    pub fn propagate_wave(&self, tags: &[String], activation: f32) -> WaveQueryResult {
        let mut wave = self.wave.lock().unwrap();
        wave.propagate_wave(tags, activation)
    }

    /// Add a reference embedding to EPA
    pub fn add_reference_embedding(&self, embedding: &[f32]) {
        let array = Array1::from(embedding.to_vec());
        let mut epa = self.epa.lock().unwrap();
        epa.add_reference(array);
    }

    /// Add a tag with optional embedding
    pub fn add_tag(&self, tag: String, is_core: bool, embedding: Option<&[f32]>) {
        let mut wave = self.wave.lock().unwrap();
        let _id = wave.add_tag(tag.clone(), is_core);
        if let Some(emb) = embedding {
            wave.set_embedding(&tag, emb.to_vec());
        }
        drop(wave);
        let _ = self.save_to_storage();
    }

    /// Add an association between tags
    pub fn associate_tags(&self, tag1: &str, tag2: &str, weight: f32) {
        let mut wave = self.wave.lock().unwrap();
        wave.add_edge(tag1, tag2, EdgeType::Associative, weight);
        drop(wave);
        let _ = self.save_to_storage();
    }

    /// Record co-occurrence of tags
    pub fn record_cooccurrence(&self, tags: &[String]) {
        let mut wave = self.wave.lock().unwrap();
        wave.record_cooccurrence(tags, EdgeType::Temporal);
        drop(wave);
        let _ = self.save_to_storage();
    }

    /// Find similar tags using wave propagation
    pub fn find_similar_tags(&self, tag: &str, top_k: usize) -> Vec<(String, f32)> {
        let mut wave = self.wave.lock().unwrap();
        wave.find_similar_tags(tag, top_k)
    }

    /// Get core tags
    pub fn core_tags(&self) -> Vec<TagNode> {
        let wave = self.wave.lock().unwrap();
        wave.core_tags().iter().map(|&t| t.clone()).collect()
    }

    /// Get all tags
    pub fn all_tags(&self) -> Vec<TagNode> {
        let wave = self.wave.lock().unwrap();
        wave.all_tags().iter().map(|&t| t.clone()).collect()
    }

    /// Reset wave activations
    pub fn reset_activations(&self) {
        let mut wave = self.wave.lock().unwrap();
        wave.reset_activations();
    }

    /// Get activated tags with scores from a wave result
    pub fn get_activated_tags(&self, result: &WaveQueryResult) -> Vec<(String, f32)> {
        let wave = self.wave.lock().unwrap();
        wave.get_activated_tags(result)
    }
}

#[async_trait::async_trait]
impl MemorySystem for TagMemoMemory {
    async fn store(&self, mut item: MemoryItem) -> CorvusResult<String> {
        let id = item.id.clone().unwrap_or_else(Self::generate_id);
        item.id = Some(id.clone());

        // Record tags in TagMemo
        if !item.tags.is_empty() {
            let mut wave = self.wave.lock().unwrap();
            for tag in &item.tags {
                let _ = wave.add_tag(tag.clone(), false);
            }
            wave.record_cooccurrence(&item.tags, EdgeType::Temporal);
        }

        // Store in memory
        let mut items = self.items.lock().unwrap();
        items.insert(id.clone(), item.clone());
        drop(items);

        // Also save to persistent storage if available
        let mut storage_opt = self.storage.lock().unwrap();
        if let Some(storage) = storage_opt.as_mut() {
            let created_at = item.timestamp.timestamp();
            let record = MemoryRecord {
                id: id.clone(),
                content: item.content.clone(),
                embedding: None,
                tags: item.tags.clone(),
                created_at,
                last_accessed: created_at,
                access_count: 0,
                metadata: HashMap::new(),
            };
            storage.store_memory(record)?;
        }
        drop(storage_opt);

        let _ = self.save_to_storage();

        Ok(id)
    }

    async fn retrieve(&self, query: MemoryQuery) -> CorvusResult<Vec<MemoryResult>> {
        let items = self.items.lock().unwrap();

        // Use TagMemo to expand tags if enabled
        let expanded_tags = if query.use_tagmemo && !query.tags.is_empty() {
            let mut wave = self.wave.lock().unwrap();
            let result = wave.propagate_wave(&query.tags, 1.0);
            wave.get_activated_tags(&result)
                .into_iter()
                .map(|(tag, _)| tag)
                .collect::<Vec<_>>()
        } else {
            query.tags.clone()
        };

        let mut results: Vec<MemoryResult> = items
            .values()
            .filter(|item| {
                // Filter by content types
                if !query.content_types.is_empty() && !query.content_types.contains(&item.content_type) {
                    return false;
                }

                // Filter by tags (using expanded tags if available)
                let tags_to_check = if expanded_tags.is_empty() {
                    &query.tags
                } else {
                    &expanded_tags
                };

                if !tags_to_check.is_empty() {
                    let has_tag = tags_to_check.iter().any(|t| item.tags.contains(t));
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
                // Calculate score
                let base_score = if !query.tags.is_empty() {
                    // Tag overlap score
                    let overlap: usize = query
                        .tags
                        .iter()
                        .filter(|t| item.tags.contains(t))
                        .count();
                    overlap as f64 / query.tags.len() as f64
                } else if let Some(text) = &query.text {
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

                // Boost score with TagMemo if enabled
                let final_score = if query.use_tagmemo && !item.tags.is_empty() {
                    base_score * 1.2
                } else {
                    base_score
                };

                MemoryResult {
                    item: item.clone(),
                    score: final_score.min(1.0),
                    explanation: if query.use_tagmemo {
                        Some("TagMemo enhanced retrieval".to_string())
                    } else {
                        None
                    },
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        results.truncate(query.limit);

        Ok(results)
    }

    async fn tag(&self, item_id: &str, tags: Vec<String>) -> CorvusResult<()> {
        let mut items = self.items.lock().unwrap();

        let item = items
            .get_mut(item_id)
            .ok_or_else(|| MemoryError::ItemNotFound(item_id.to_string()))?;

        // Add tags to item
        for tag in &tags {
            if !item.tags.contains(tag) {
                item.tags.push(tag.clone());
            }
        }

        // Also update TagMemo
        let mut wave = self.wave.lock().unwrap();
        for tag in &tags {
            let _ = wave.add_tag(tag.clone(), false);
        }
        if !item.tags.is_empty() {
            wave.record_cooccurrence(&item.tags, EdgeType::Temporal);
        }

        Ok(())
    }

    async fn search_by_tags(&self, tags: Vec<String>) -> CorvusResult<Vec<MemoryItem>> {
        let items = self.items.lock().unwrap();

        Ok(items
            .values()
            .filter(|item| tags.iter().any(|t| item.tags.contains(t)))
            .cloned()
            .collect())
    }

    async fn get(&self, item_id: &str) -> CorvusResult<MemoryItem> {
        let items = self.items.lock().unwrap();

        items
            .get(item_id)
            .cloned()
            .ok_or_else(|| MemoryError::ItemNotFound(item_id.to_string()).into())
    }

    async fn delete(&self, item_id: &str) -> CorvusResult<()> {
        let mut items = self.items.lock().unwrap();

        items
            .remove(item_id)
            .ok_or_else(|| MemoryError::ItemNotFound(item_id.to_string()))?;
        drop(items);

        // Also delete from persistent storage
        let mut storage_opt = self.storage.lock().unwrap();
        if let Some(storage) = storage_opt.as_mut() {
            storage.delete_memory(item_id)?;
        }

        Ok(())
    }

    async fn list_recent(&self, limit: usize) -> CorvusResult<Vec<MemoryItem>> {
        let items = self.items.lock().unwrap();

        let mut items: Vec<_> = items.values().cloned().collect();
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        items.truncate(limit);

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tagmemo_creation() {
        let _memory = TagMemoMemory::new(128);
    }

    #[test]
    fn test_tagmemo_with_in_memory_storage() -> Result<()> {
        let _memory = TagMemoMemory::with_in_memory_storage(128)?;
        Ok(())
    }

    #[test]
    fn test_epa_analysis() {
        let memory = TagMemoMemory::new(128);
        let embedding = vec![0.1; 128];

        let analysis = memory.analyze_embedding(&embedding);
        assert!(analysis.logic_depth >= 0.0);
        assert!(analysis.logic_depth <= 1.0);
        assert!(analysis.resonance >= 0.0);
    }

    #[test]
    fn test_pyramid_decomposition() -> Result<()> {
        let memory = TagMemoMemory::new(128);
        let embedding = vec![0.5; 128];

        let levels = memory.decompose_embedding(&embedding)?;
        assert_eq!(levels.len(), 5); // 5 levels by default

        Ok(())
    }

    #[test]
    fn test_tagmemo_persistence() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test_memory.db");

        // Create and populate memory
        {
            let memory = TagMemoMemory::with_storage(128, &db_path)?;
            memory.add_tag("test".to_string(), true, None);
            memory.add_tag("example".to_string(), false, None);
            memory.associate_tags("test", "example", 0.8);
            memory.save_to_storage()?;
        }

        // Reopen and verify
        {
            let memory = TagMemoMemory::with_storage(128, &db_path)?;
            let tags = memory.all_tags();
            assert_eq!(tags.len(), 2);
        }

        Ok(())
    }
}
