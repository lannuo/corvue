//! Memory system trait and types

use crate::embedding::Embedding;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of content stored in memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    /// Plain text
    Text,
    /// Code snippet
    Code,
    /// Conversation turn
    Conversation,
    /// Internal thought
    Thought,
    /// Dream state
    Dream,
}

/// A tag for categorizing memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique ID of the tag
    pub id: String,
    /// Name of the tag
    pub name: String,
    /// Optional embedding for the tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Embedding>,
    /// Whether this is a "core" tag (higher priority)
    #[serde(default)]
    pub is_core: bool,
}

impl Tag {
    /// Create a new tag
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            embedding: None,
            is_core: false,
        }
    }

    /// Mark as a core tag
    pub fn core(mut self) -> Self {
        self.is_core = true;
        self
    }

    /// Add an embedding
    pub fn with_embedding(mut self, embedding: Embedding) -> Self {
        self.embedding = Some(embedding);
        self
    }
}

/// A scored tag (for retrieval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredTag {
    /// The tag
    pub tag: Tag,
    /// The score
    pub score: f32,
    /// Where this tag came from
    pub source: TagSource,
}

/// The source of a scored tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagSource {
    /// User-specified in the query
    UserSpecified,
    /// First-round expansion
    FirstRound,
    /// From residual pyramid at a specific level
    PyramidLevel(u32),
    /// From co-occurrence matrix
    Cooccurrence,
}

/// An item stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique ID (None when creating, Some when stored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The main content
    pub content: String,
    /// The type of content
    pub content_type: ContentType,
    /// Tags associated with this item
    pub tags: Vec<String>,
    /// Optional source (file path, URL, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// When this item was created
    pub timestamp: DateTime<Utc>,
    /// Optional embedding for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Embedding>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MemoryItem {
    /// Create a new memory item
    pub fn new(content: impl Into<String>, content_type: ContentType) -> Self {
        Self {
            id: None,
            content: content.into(),
            content_type,
            tags: Vec::new(),
            source: None,
            timestamp: chrono::Utc::now(),
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Add a source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add an embedding
    pub fn with_embedding(mut self, embedding: Embedding) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add a metadata entry
    pub fn with_metadata<K: Into<String>, V: Into<serde_json::Value>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set a custom timestamp
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// A query to the memory system
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// Text query (optional if embedding is provided)
    pub text: Option<String>,
    /// Pre-computed embedding (optional if text is provided)
    pub embedding: Option<Embedding>,
    /// Tags to filter by
    pub tags: Vec<String>,
    /// Content types to include
    pub content_types: Vec<ContentType>,
    /// Time range filter (start, end)
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Maximum number of results
    pub limit: usize,
    /// Whether to use TagMemo cognitive enhancement
    pub use_tagmemo: bool,
}

impl MemoryQuery {
    /// Create a new text-based query
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            embedding: None,
            tags: Vec::new(),
            content_types: Vec::new(),
            time_range: None,
            limit: 10,
            use_tagmemo: true,
        }
    }

    /// Create a new embedding-based query
    pub fn embedding(embedding: Embedding) -> Self {
        Self {
            text: None,
            embedding: Some(embedding),
            tags: Vec::new(),
            content_types: Vec::new(),
            time_range: None,
            limit: 10,
            use_tagmemo: true,
        }
    }

    /// Add tags to filter by
    pub fn with_tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Filter by content types
    pub fn with_content_types(mut self, types: Vec<ContentType>) -> Self {
        self.content_types = types;
        self
    }

    /// Set time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some((start, end));
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Enable/disable TagMemo
    pub fn with_tagmemo(mut self, enabled: bool) -> Self {
        self.use_tagmemo = enabled;
        self
    }
}

/// A result from a memory query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResult {
    /// The memory item
    pub item: MemoryItem,
    /// The relevance score (0.0 - 1.0)
    pub score: f64,
    /// Optional explanation of why this was retrieved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

/// An enhanced query after TagMemo processing
#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    /// The original embedding
    pub original_embedding: Embedding,
    /// The enhanced embedding after TagMemo
    pub enhanced_embedding: Embedding,
    /// Expanded tags with scores
    pub tags: Vec<ScoredTag>,
    /// EPA analysis result
    pub epa_analysis: EPAAnalysis,
    /// Residual pyramid analysis
    pub pyramid_analysis: PyramidAnalysis,
    /// Resonance detection
    pub resonance: ResonanceAnalysis,
}

/// EPA (Embedding Projection Analysis) result
#[derive(Debug, Clone)]
pub struct EPAAnalysis {
    /// Logic depth (0.0 - 1.0, higher = more focused)
    pub logic_depth: f32,
    /// Entropy (0.0 - 1.0, higher = more scattered)
    pub entropy: f32,
    /// Dominant semantic axes
    pub dominant_axes: Vec<String>,
}

/// Residual pyramid analysis
#[derive(Debug, Clone)]
pub struct PyramidAnalysis {
    /// Number of levels used
    pub levels: u32,
    /// Energy explained at each level
    pub energy_by_level: Vec<f32>,
    /// Total energy explained
    pub total_energy: f32,
    /// TagMemo activation factor
    pub tagmemo_activation: f32,
}

/// Cross-domain resonance analysis
#[derive(Debug, Clone)]
pub struct ResonanceAnalysis {
    /// Whether resonance was detected
    pub detected: bool,
    /// The two axes that resonated
    pub axes: Option<(String, String)>,
    /// Resonance strength
    pub strength: f32,
}

/// Trait for memory systems
#[async_trait::async_trait]
pub trait MemorySystem: Send + Sync {
    /// Store a memory item
    async fn store(&self, item: MemoryItem) -> Result<String>;

    /// Retrieve memory items matching a query
    async fn retrieve(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>>;

    /// Add tags to an existing memory item
    async fn tag(&self, item_id: &str, tags: Vec<String>) -> Result<()>;

    /// Search for memory items by tags
    async fn search_by_tags(&self, tags: Vec<String>) -> Result<Vec<MemoryItem>>;

    /// Get a memory item by ID
    async fn get(&self, item_id: &str) -> Result<MemoryItem>;

    /// Delete a memory item
    async fn delete(&self, item_id: &str) -> Result<()>;

    /// List recent memory items
    async fn list_recent(&self, limit: usize) -> Result<Vec<MemoryItem>>;

    /// Optional: Enhance a query with TagMemo (returns None if not supported)
    async fn enhance_query(&self, _query: MemoryQuery) -> Result<Option<EnhancedQuery>> {
        Ok(None)
    }
}
