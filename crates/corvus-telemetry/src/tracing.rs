//! OpenTelemetry tracing integration

use crate::error::{Result, TelemetryError};
use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer as OtelTracer},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{BatchConfig, Config, TracerProvider},
    Resource,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Whether tracing is enabled
    pub enabled: bool,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
    /// Export timeout
    pub export_timeout_ms: u64,
    /// Batch size
    pub batch_size: usize,
    /// Sampling ratio (0.0 - 1.0)
    pub sampling_ratio: f64,
    /// Additional attributes to add to all spans
    pub attributes: Vec<(String, String)>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            otlp_endpoint: None,
            export_timeout_ms: 10000,
            batch_size: 512,
            sampling_ratio: 1.0,
            attributes: Vec::new(),
        }
    }
}

/// Tracer for creating and managing spans
pub struct Tracer {
    _provider: TracerProvider,
    tracer: opentelemetry_sdk::trace::Tracer,
}

impl Tracer {
    /// Create a new tracer with the given configuration
    pub async fn new(
        service_name: String,
        service_version: String,
        config: TracingConfig,
    ) -> Result<Self> {
        if !config.enabled {
            return Err(TelemetryError::Tracing("Tracing is disabled".to_string()));
        }

        // Set up propagator
        global::set_text_map_propagator(TraceContextPropagator::new());

        // Build resource
        let mut resource = Resource::new(vec![
            KeyValue::new("service.name", service_name),
            KeyValue::new("service.version", service_version),
        ]);

        // Add custom attributes
        for (key, value) in config.attributes {
            resource = resource.merge(&Resource::new(vec![KeyValue::new(key, value)]));
        }

        // Build tracer provider
        let provider = if let Some(endpoint) = config.otlp_endpoint {
            // OTLP exporter
            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_timeout(Duration::from_millis(config.export_timeout_ms));

            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(exporter)
                .with_trace_config(Config::default().with_resource(resource))
                .with_batch_config(
                    BatchConfig::default()
                        .with_max_export_batch_size(config.batch_size)
                        .with_max_queue_size(config.batch_size * 4),
                )
                .install_batch(opentelemetry_sdk::runtime::Tokio)
                .map_err(|e| TelemetryError::Tracing(format!("Failed to install OTLP tracer: {}", e)))?
        } else {
            // No exporter configured, use a no-op tracer provider
            TracerProvider::builder()
                .with_config(Config::default().with_resource(resource))
                .build()
        };

        // Set global tracer provider
        global::set_tracer_provider(provider.clone());

        let tracer = provider.tracer("corvus");

        Ok(Self {
            _provider: provider,
            tracer,
        })
    }

    /// Get the current trace ID as a string, if available
    pub fn current_trace_id(&self) -> Option<String> {
        let cx = opentelemetry::Context::current();
        let span = cx.span();
        let span_context = span.span_context();
        if span_context.is_valid() {
            Some(span_context.trace_id().to_string())
        } else {
            None
        }
    }

    /// Get the current span ID as a string, if available
    pub fn current_span_id(&self) -> Option<String> {
        let cx = opentelemetry::Context::current();
        let span = cx.span();
        let span_context = span.span_context();
        if span_context.is_valid() {
            Some(span_context.span_id().to_string())
        } else {
            None
        }
    }

    /// Shutdown the tracer and flush all pending spans
    pub async fn shutdown(&self) -> Result<()> {
        opentelemetry::global::shutdown_tracer_provider();
        Ok(())
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        // Note: shutdown should be called explicitly with .await
        // This is a fallback for cleanup
    }
}
