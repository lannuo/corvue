//! Corvus Telemetry - Observability and monitoring
//!
//! This crate provides telemetry capabilities for Corvus:
//! - Tracing (OpenTelemetry)
//! - Metrics (OpenTelemetry)
//! - Structured logging

#![warn(missing_docs)]

pub mod error;
pub mod metrics;
pub mod tracing;
pub mod logging;

// Public re-exports
pub use error::{TelemetryError, Result};
pub use metrics::{Metrics, MetricsConfig};
pub use tracing::{Tracer, TracingConfig};
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
    /// Tracing configuration
    pub tracing: TracingConfig,
    /// Metrics configuration
    pub metrics: MetricsConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_name: "corvus".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            tracing: TracingConfig::default(),
            metrics: MetricsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Telemetry manager that coordinates tracing, metrics, and logging
pub struct Telemetry {
    config: TelemetryConfig,
    tracer: Option<Arc<Tracer>>,
    metrics: Option<Arc<Metrics>>,
    logger: Option<Arc<Logger>>,
}

impl Telemetry {
    /// Create a new telemetry manager with the given configuration
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            tracer: None,
            metrics: None,
            logger: None,
        }
    }

    /// Initialize telemetry with the configuration
    pub async fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Initialize logging first so we can log other init steps
        if self.config.logging.enabled {
            let logger = Logger::new(self.config.logging.clone())?;
            logger.init()?;
            self.logger = Some(Arc::new(logger));
        }

        // Initialize tracing
        if self.config.tracing.enabled {
            let tracer = Tracer::new(
                self.config.service_name.clone(),
                self.config.service_version.clone(),
                self.config.tracing.clone(),
            )
            .await?;
            self.tracer = Some(Arc::new(tracer));
        }

        // Initialize metrics
        if self.config.metrics.enabled {
            let metrics = Metrics::new(
                self.config.service_name.clone(),
                self.config.service_version.clone(),
                self.config.metrics.clone(),
            )
            .await?;
            self.metrics = Some(Arc::new(metrics));
        }

        Ok(())
    }

    /// Get the tracer if enabled
    pub fn tracer(&self) -> Option<&Arc<Tracer>> {
        self.tracer.as_ref()
    }

    /// Get the metrics if enabled
    pub fn metrics(&self) -> Option<&Arc<Metrics>> {
        self.metrics.as_ref()
    }

    /// Get the logger if enabled
    pub fn logger(&self) -> Option<&Arc<Logger>> {
        self.logger.as_ref()
    }

    /// Shutdown telemetry and flush all data
    pub async fn shutdown(&self) -> Result<()> {
        if let Some(tracer) = &self.tracer {
            tracer.shutdown().await?;
        }
        if let Some(metrics) = &self.metrics {
            metrics.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new(TelemetryConfig::default())
    }
}
