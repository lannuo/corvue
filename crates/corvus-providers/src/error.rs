//! Provider error types

use thiserror::Error;

/// Error type for provider operations
#[derive(Error, Debug)]
pub enum ProviderError {
    /// API request failed
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Invalid response format
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Context window exceeded
    #[error("Context window exceeded")]
    ContextWindowExceeded,

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
