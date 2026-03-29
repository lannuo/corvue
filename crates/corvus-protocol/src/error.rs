//! Error types for MCP/ACP protocol

use thiserror::Error;

/// Result type for protocol operations
pub type Result<T> = std::result::Result<T, ProtocolError>;

/// Protocol errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Invalid message
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Timeout
    #[error("Operation timed out")]
    Timeout,

    /// Not initialized
    #[error("Not initialized")]
    NotInitialized,

    /// Method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid params
    #[error("Invalid params: {0}")]
    InvalidParams(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
