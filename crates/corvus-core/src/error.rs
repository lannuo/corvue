//! Error types for Corvus Core

use thiserror::Error;

/// Result type alias for Corvus operations
pub type Result<T> = std::result::Result<T, CorvusError>;

/// Main error type for Corvus
#[derive(Error, Debug)]
pub enum CorvusError {
    /// Generic error with message
    #[error("{0}")]
    Generic(String),

    /// Completion model error
    #[error("Completion error: {0}")]
    Completion(#[from] CompletionError),

    /// Embedding model error
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),

    /// Memory system error
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),

    /// Tool execution error
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid argument error
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Error for completion model operations
#[derive(Error, Debug)]
pub enum CompletionError {
    /// API request failed
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Invalid response format
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Context window exceeded
    #[error("Context window exceeded")]
    ContextWindowExceeded,

    /// Streaming error
    #[error("Streaming error: {0}")]
    Streaming(String),
}

/// Error for embedding model operations
#[derive(Error, Debug)]
pub enum EmbeddingError {
    /// API request failed
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Invalid response format
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Dimension mismatch
    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    /// Too many documents
    #[error("Too many documents: max {max}, got {got}")]
    TooManyDocuments { max: usize, got: usize },
}

/// Error for memory system operations
#[derive(Error, Debug)]
pub enum MemoryError {
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Item not found
    #[error("Memory item not found: {0}")]
    ItemNotFound(String),

    /// Tag not found
    #[error("Tag not found: {0}")]
    TagNotFound(String),

    /// Vector index error
    #[error("Vector index error: {0}")]
    VectorIndex(String),

    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

/// Error for tool operations
#[derive(Error, Debug)]
pub enum ToolError {
    /// Tool not found
    #[error("Tool not found: {0}")]
    NotFound(String),

    /// Invalid arguments
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    Execution(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

// Convenience conversions
impl From<anyhow::Error> for CorvusError {
    fn from(err: anyhow::Error) -> Self {
        CorvusError::Generic(err.to_string())
    }
}
