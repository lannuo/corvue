//! Corvus Telemetry - Observability and monitoring
//!
//! This crate provides telemetry capabilities for Corvus:
//! - Tracing (tracing crate)
//! - Metrics (basic counters)
//! - Structured logging

#![warn(missing_docs)]

pub mod error;
pub mod metrics;
pub mod logging;

// Public re-exports
pub use error::{TelemetryError, Result};
pub use metrics::Metrics;
pub use logging::{Logger, LoggingConfig, LogFormat};

use std::sync::Arc;

/// Telemetry configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled
    pub enabled: bool,
    /// Service name for telemetry
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: "corvus".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Telemetry manager that coordinates logging and metrics
pub struct Telemetry {
    config: TelemetryConfig,
    metrics: Option<Arc<Metrics>>,
    logger: Option<Arc<Logger>>,
}

impl Telemetry {
    /// Create a new telemetry manager with the given configuration
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            metrics: None,
            logger: None,
        }
    }

    /// Initialize telemetry with the configuration
    pub async fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Initialize logging
        if self.config.logging.enabled {
            let logger = Logger::new(self.config.logging.clone())?;
            logger.init()?;
            self.logger = Some(Arc::new(logger));
        }

        // Initialize metrics
        self.metrics = Some(Arc::new(Metrics::new()));

        Ok(())
    }

    /// Get the metrics if enabled
    pub fn metrics(&self) -> Option<&Arc<Metrics>> {
        self.metrics.as_ref()
    }

    /// Get the logger if enabled
    pub fn logger(&self) -> Option<&Arc<Logger>> {
        self.logger.as_ref()
    }

    /// Shutdown telemetry
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new(TelemetryConfig::default())
    }
}
