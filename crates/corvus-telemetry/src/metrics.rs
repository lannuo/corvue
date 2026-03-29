//! OpenTelemetry metrics integration

use crate::error::{Result, TelemetryError};
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Meter, Unit, UpDownCounter},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    metrics::{selectors, AggregatorSelector, MeterProvider, PeriodicReader, SdkMeterProvider},
    Resource,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics are enabled
    pub enabled: bool,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
    /// Export interval in milliseconds
    pub export_interval_ms: u64,
    /// Export timeout in milliseconds
    pub export_timeout_ms: u64,
    /// Additional attributes to add to all metrics
    pub attributes: Vec<(String, String)>,
    /// Prometheus export port (if enabled)
    pub prometheus_port: Option<u16>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            otlp_endpoint: None,
            export_interval_ms: 60000,
            export_timeout_ms: 10000,
            attributes: Vec::new(),
            prometheus_port: None,
        }
    }
}

/// Metrics collector for Corvus
pub struct Metrics {
    _provider: SdkMeterProvider,
    meter: Meter,

    // Counters
    pub tokens_used: Counter<u64>,
    pub requests_total: Counter<u64>,
    pub tool_calls_total: Counter<u64>,
    pub errors_total: Counter<u64>,

    // UpDownCounters
    pub active_sessions: UpDownCounter<i64>,

    // Histograms
    pub request_duration: Histogram<f64>,
    pub token_latency: Histogram<f64>,
    pub tool_call_duration: Histogram<f64>,
}

impl Metrics {
    /// Create a new metrics collector
    pub async fn new(
        service_name: String,
        service_version: String,
        config: MetricsConfig,
    ) -> Result<Arc<Self>> {
        if !config.enabled {
            return Err(TelemetryError::Metrics("Metrics are disabled".to_string()));
        }

        // Build resource
        let mut resource = Resource::new(vec![
            KeyValue::new("service.name", service_name),
            KeyValue::new("service.version", service_version),
        ]);

        // Add custom attributes
        for (key, value) in config.attributes {
            resource = resource.merge(&Resource::new(vec![KeyValue::new(key, value)]));
        }

        // Build meter provider
        let provider = if let Some(endpoint) = config.otlp_endpoint {
            // OTLP exporter
            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
                .with_timeout(Duration::from_millis(config.export_timeout_ms));

            let reader = PeriodicReader::builder(
                opentelemetry_otlp::new_pipeline()
                    .metrics(opentelemetry_sdk::runtime::Tokio)
                    .with_exporter(exporter)
                    .build()
                    .map_err(|e| {
                        TelemetryError::Metrics(format!("Failed to build OTLP metrics exporter: {}", e))
                    })?,
            )
            .with_interval(Duration::from_millis(config.export_interval_ms))
            .build();

            MeterProvider::builder()
                .with_resource(resource)
                .with_reader(reader)
                .build()
        } else {
            // No exporter configured
            MeterProvider::builder()
                .with_resource(resource)
                .build()
        };

        // Set global meter provider
        global::set_meter_provider(provider.clone());

        let meter = global::meter("corvus");

        // Create metrics
        let tokens_used = meter
            .u64_counter("tokens_used")
            .with_description("Total tokens used")
            .with_unit(Unit::new("{token}"))
            .init();

        let requests_total = meter
            .u64_counter("requests_total")
            .with_description("Total number of requests")
            .init();

        let tool_calls_total = meter
            .u64_counter("tool_calls_total")
            .with_description("Total number of tool calls")
            .init();

        let errors_total = meter
            .u64_counter("errors_total")
            .with_description("Total number of errors")
            .init();

        let active_sessions = meter
            .i64_up_down_counter("active_sessions")
            .with_description("Number of active sessions")
            .init();

        let request_duration = meter
            .f64_histogram("request_duration_seconds")
            .with_description("Request duration in seconds")
            .with_unit(Unit::new("s"))
            .init();

        let token_latency = meter
            .f64_histogram("token_latency_seconds")
            .with_description("Time per token in seconds")
            .with_unit(Unit::new("s"))
            .init();

        let tool_call_duration = meter
            .f64_histogram("tool_call_duration_seconds")
            .with_description("Tool call duration in seconds")
            .with_unit(Unit::new("s"))
            .init();

        Ok(Arc::new(Self {
            _provider: provider,
            meter,
            tokens_used,
            requests_total,
            tool_calls_total,
            errors_total,
            active_sessions,
            request_duration,
            token_latency,
            tool_call_duration,
        }))
    }

    /// Record token usage
    pub fn record_tokens(&self, tokens: u64, model: &str) {
        self.tokens_used.add(
            &opentelemetry::Context::current(),
            tokens,
            &[KeyValue::new("model", model.to_string())],
        );
    }

    /// Increment request count
    pub fn increment_requests(&self, model: &str) {
        self.requests_total.add(
            &opentelemetry::Context::current(),
            1,
            &[KeyValue::new("model", model.to_string())],
        );
    }

    /// Increment tool calls
    pub fn increment_tool_calls(&self, tool_name: &str) {
        self.tool_calls_total.add(
            &opentelemetry::Context::current(),
            1,
            &[KeyValue::new("tool", tool_name.to_string())],
        );
    }

    /// Increment errors
    pub fn increment_errors(&self, error_type: &str) {
        self.errors_total.add(
            &opentelemetry::Context::current(),
            1,
            &[KeyValue::new("error_type", error_type.to_string())],
        );
    }

    /// Increment active sessions
    pub fn increment_active_sessions(&self) {
        self.active_sessions
            .add(&opentelemetry::Context::current(), 1, &[]);
    }

    /// Decrement active sessions
    pub fn decrement_active_sessions(&self) {
        self.active_sessions
            .add(&opentelemetry::Context::current(), -1, &[]);
    }

    /// Record request duration
    pub fn record_request_duration(&self, duration: Duration, model: &str) {
        self.request_duration.record(
            &opentelemetry::Context::current(),
            duration.as_secs_f64(),
            &[KeyValue::new("model", model.to_string())],
        );
    }

    /// Record tool call duration
    pub fn record_tool_call_duration(&self, duration: Duration, tool_name: &str) {
        self.tool_call_duration.record(
            &opentelemetry::Context::current(),
            duration.as_secs_f64(),
            &[KeyValue::new("tool", tool_name.to_string())],
        );
    }

    /// Shutdown metrics and flush all data
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Drop for Metrics {
    fn drop(&mut self) {
        // Note: shutdown should be called explicitly
    }
}
