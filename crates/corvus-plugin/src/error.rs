//! Plugin errors

use thiserror::Error;

/// Plugin error type
#[derive(Error, Debug)]
pub enum PluginError {
    /// Initialization error
    #[error("Plugin initialization failed: {0}")]
    Initialization(String),

    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Invalid plugin
    #[error("Invalid plugin: {0}")]
    Invalid(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

/// Plugin result type
pub type PluginResult<T> = std::result::Result<T, PluginError>;

impl From<String> for PluginError {
    fn from(s: String) -> Self {
        PluginError::Other(s)
    }
}

impl From<&str> for PluginError {
    fn from(s: &str) -> Self {
        PluginError::Other(s.to_string())
    }
}
