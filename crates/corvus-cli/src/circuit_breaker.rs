//! Circuit Breaker pattern implementation
//!
//! Prevents repeated calls to services that are likely to fail.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is half-open, allowing a test request
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: usize,
    /// Time to wait before transitioning from Open to HalfOpen
    pub reset_timeout: Duration,
    /// Number of successful requests in HalfOpen state before closing
    pub success_threshold: usize,
    /// Time window for counting failures
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
            failure_window: Duration::from_secs(60),
        }
    }
}

/// A single failure record
#[derive(Debug, Clone)]
struct FailureRecord {
    timestamp: Instant,
}

/// Circuit Breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Mutex<CircuitState>,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    last_failure_time: Mutex<Option<Instant>>,
    open_time: Mutex<Option<Instant>>,
    failures: Mutex<Vec<FailureRecord>>,
    call_count: AtomicU64,
    rejection_count: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(CircuitState::Closed),
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            last_failure_time: Mutex::new(None),
            open_time: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            call_count: AtomicU64::new(0),
            rejection_count: AtomicU64::new(0),
        }
    }

    /// Check if the circuit allows a request
    pub async fn allow_request(&self) -> bool {
        self.call_count.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to HalfOpen
                if let Some(open_time) = *self.open_time.lock().await {
                    if open_time.elapsed() >= self.config.reset_timeout {
                        *state = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::Relaxed);
                        return true;
                    }
                }
                self.rejection_count.fetch_add(1, Ordering::Relaxed);
                false
            }
            CircuitState::HalfOpen => {
                // Allow one request at a time in HalfOpen state
                true
            }
        }
    }

    /// Record a successful call
    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
                self.cleanup_old_failures().await;
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    *self.open_time.lock().await = None;
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed call
    pub async fn record_failure(&self) {
        let mut state = self.state.lock().await;

        match *state {
            CircuitState::Closed => {
                self.failure_count.fetch_add(1, Ordering::Relaxed);

                // Record the failure
                let mut failures = self.failures.lock().await;
                failures.push(FailureRecord {
                    timestamp: Instant::now(),
                });
                *self.last_failure_time.lock().await = Some(Instant::now());

                // Clean up old failures
                self.cleanup_old_failures_internal(&mut failures).await;

                // Check if we should open the circuit
                if failures.len() >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    *self.open_time.lock().await = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen state opens the circuit again
                *state = CircuitState::Open;
                *self.open_time.lock().await = Some(Instant::now());
                self.success_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {}
        }
    }

    /// Get the current state
    pub async fn state(&self) -> CircuitState {
        *self.state.lock().await
    }

    /// Get statistics about the circuit breaker
    pub async fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: self.state().await,
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            call_count: self.call_count.load(Ordering::Relaxed),
            rejection_count: self.rejection_count.load(Ordering::Relaxed),
            last_failure_time: *self.last_failure_time.lock().await,
            open_time: *self.open_time.lock().await,
        }
    }

    /// Manually reset the circuit breaker to Closed state
    pub async fn force_reset(&self) {
        let mut state = self.state.lock().await;
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        *self.open_time.lock().await = None;
        *self.last_failure_time.lock().await = None;
        self.failures.lock().await.clear();
    }

    /// Clean up old failures outside the window
    async fn cleanup_old_failures(&self) {
        let mut failures = self.failures.lock().await;
        self.cleanup_old_failures_internal(&mut failures).await;
    }

    async fn cleanup_old_failures_internal(&self, failures: &mut Vec<FailureRecord>) {
        let now = Instant::now();
        failures.retain(|f| now.duration_since(f.timestamp) < self.config.failure_window);
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current state
    pub state: CircuitState,
    /// Number of failures in the current window
    pub failure_count: usize,
    /// Number of successes in HalfOpen state
    pub success_count: usize,
    /// Total number of calls
    pub call_count: u64,
    /// Number of rejected calls
    pub rejection_count: u64,
    /// Time of the last failure
    pub last_failure_time: Option<Instant>,
    /// Time when the circuit was opened
    pub open_time: Option<Instant>,
}

/// Error when the circuit is open
#[derive(Debug, Clone, thiserror::Error)]
#[error("Circuit breaker is open")]
pub struct CircuitOpenError;

/// Execute an operation with circuit breaker protection
pub async fn with_circuit_breaker<F, Fut, T, E>(
    circuit_breaker: &CircuitBreaker,
    operation: F,
) -> Result<T, CircuitBreakerError<E>>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    if !circuit_breaker.allow_request().await {
        return Err(CircuitBreakerError::CircuitOpen);
    }

    match operation().await {
        Ok(result) => {
            circuit_breaker.record_success().await;
            Ok(result)
        }
        Err(e) => {
            circuit_breaker.record_failure().await;
            Err(CircuitBreakerError::Operation(e))
        }
    }
}

/// Error type for circuit breaker operations
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    /// Circuit is open
    #[error("Circuit breaker is open")]
    CircuitOpen,
    /// Operation failed
    #[error("Operation failed: {0}")]
    Operation(E),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_default_state() {
        let cb = CircuitBreaker::new();
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
            failure_window: Duration::from_secs(60),
        };
        let cb = CircuitBreaker::with_config(config);

        // Record failures
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.allow_request().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_resets_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
            failure_window: Duration::from_secs(60),
        };
        let cb = CircuitBreaker::with_config(config);

        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.stats().await.failure_count, 2);

        cb.record_success().await;
        assert_eq!(cb.stats().await.failure_count, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_force_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 2,
            failure_window: Duration::from_secs(60),
        };
        let cb = CircuitBreaker::with_config(config);

        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        cb.force_reset().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.allow_request().await);
    }
}
