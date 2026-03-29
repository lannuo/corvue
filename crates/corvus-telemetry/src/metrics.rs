//! Basic metrics collection

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether metrics are enabled
    pub enabled: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Basic metrics collector
pub struct Metrics {
    tokens_used: AtomicU64,
    requests_total: AtomicU64,
    tool_calls_total: AtomicU64,
    errors_total: AtomicU64,
}

impl Metrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            tokens_used: AtomicU64::new(0),
            requests_total: AtomicU64::new(0),
            tool_calls_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
        }
    }

    /// Create a new metrics collector with config
    pub async fn new_with_config(
        _service_name: String,
        _service_version: String,
        _config: MetricsConfig,
    ) -> Self {
        Self::new()
    }

    /// Record token usage
    pub fn record_tokens(&self, tokens: u64, _model: &str) {
        self.tokens_used.fetch_add(tokens, Ordering::Relaxed);
    }

    /// Increment request count
    pub fn increment_requests(&self, _model: &str) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment tool calls
    pub fn increment_tool_calls(&self, _tool_name: &str) {
        self.tool_calls_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment errors
    pub fn increment_errors(&self, _error_type: &str) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Get token count
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used.load(Ordering::Relaxed)
    }

    /// Get request count
    pub fn requests_total(&self) -> u64 {
        self.requests_total.load(Ordering::Relaxed)
    }

    /// Get tool call count
    pub fn tool_calls_total(&self) -> u64 {
        self.tool_calls_total.load(Ordering::Relaxed)
    }

    /// Get error count
    pub fn errors_total(&self) -> u64 {
        self.errors_total.load(Ordering::Relaxed)
    }

    /// Shutdown metrics
    pub async fn shutdown(&self) {
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
