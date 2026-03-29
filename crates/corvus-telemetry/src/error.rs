//! Telemetry errors

use thiserror::Error;

/// Telemetry error type
#[derive(Error, Debug)]
pub enum TelemetryError {
    /// Tracing initialization error
    #[error("Tracing error: {0}")]
    Tracing(String),

    /// Metrics initialization error
    #[error("Metrics error: {0}")]
    Metrics(String),

    /// Logging initialization error
    #[error("Logging error: {0}")]
    Logging(String),

    /// Export error
    #[error("Export error: {0}")]
    Export(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

/// Result type for telemetry operations
pub type Result<T> = std::result::Result<T, TelemetryError>;
