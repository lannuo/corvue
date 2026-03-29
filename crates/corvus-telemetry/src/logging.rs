//! Structured logging configuration and initialization

use crate::error::{Result, TelemetryError};
use serde::{Deserialize, Serialize};

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogFormat {
    /// Human-readable text format
    #[default]
    Text,
    /// JSON format for structured logging
    Json,
    /// Compact format
    Compact,
}


/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Whether logging is enabled
    pub enabled: bool,
    /// Log level (off, error, warn, info, debug, trace)
    pub level: String,
    /// Log format
    pub format: LogFormat,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            format: LogFormat::default(),
        }
    }
}

/// Logger instance
pub struct Logger {
    config: LoggingConfig,
}

impl Logger {
    /// Create a new logger with the given configuration
    pub fn new(config: LoggingConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Initialize the global logger
    pub fn init(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Simple logger initialization
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .try_init()
            .map_err(|e| TelemetryError::Logging(format!("Failed to initialize logger: {}", e)))?;

        Ok(())
    }

    /// Get the logging configuration
    pub fn config(&self) -> &LoggingConfig {
        &self.config
    }
}
