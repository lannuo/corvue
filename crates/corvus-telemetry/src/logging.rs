//! Structured logging configuration and initialization

use crate::error::{Result, TelemetryError};
use serde::{Deserialize, Serialize};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable text format
    Text,
    /// JSON format for structured logging
    Json,
    /// Compact format
    Compact,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Text
    }
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
    /// Whether to include timestamps
    pub with_timestamp: bool,
    /// Whether to include thread IDs
    pub with_thread_id: bool,
    /// Whether to include span events
    pub with_span_events: bool,
    /// Target filter (e.g., "corvus=debug,tower_http=info")
    pub target_filter: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            format: LogFormat::default(),
            with_timestamp: true,
            with_thread_id: false,
            with_span_events: false,
            target_filter: None,
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

        // Parse log level
        let level_filter = self.config.level.parse::<LevelFilter>().map_err(|_| {
            TelemetryError::Config(format!("Invalid log level: {}", self.config.level))
        })?;

        // Build env filter
        let mut env_filter = EnvFilter::from_default_env()
            .add_directive(level_filter.into());

        if let Some(filter) = &self.config.target_filter {
            for directive in filter.split(',') {
                if let Ok(d) = directive.trim().parse() {
                    env_filter = env_filter.add_directive(d);
                }
            }
        }

        // Create formatter layer
        let fmt_layer = match self.config.format {
            LogFormat::Text => {
                let mut layer = fmt::layer()
                    .with_target(true)
                    .with_level(true);

                if self.config.with_timestamp {
                    layer = layer.with_timer(fmt::time::UtcTime::rfc_3339());
                } else {
                    layer = layer.with_timer(fmt::time::None);
                }

                if self.config.with_thread_id {
                    layer = layer.with_thread_ids(true);
                }

                if self.config.with_span_events {
                    layer = layer.with_span_events(FmtSpan::FULL);
                }

                layer.boxed()
            }
            LogFormat::Json => {
                let mut layer = fmt::layer()
                    .json()
                    .with_target(true)
                    .with_level(true);

                if self.config.with_timestamp {
                    layer = layer.with_timer(fmt::time::UtcTime::rfc_3339());
                }

                if self.config.with_span_events {
                    layer = layer.with_span_events(FmtSpan::FULL);
                }

                layer.boxed()
            }
            LogFormat::Compact => {
                let mut layer = fmt::layer()
                    .compact()
                    .with_target(true)
                    .with_level(true);

                if self.config.with_timestamp {
                    layer = layer.with_timer(fmt::time::UtcTime::rfc_3339());
                }

                layer.boxed()
            }
        };

        // Initialize subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| TelemetryError::Logging(format!("Failed to initialize logger: {}", e)))?;

        Ok(())
    }

    /// Get the logging configuration
    pub fn config(&self) -> &LoggingConfig {
        &self.config
    }
}
