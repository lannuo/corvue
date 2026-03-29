//! Friendly error display with suggestions

use console::style;
use corvus_core::error::{CompletionError, CorvusError, EmbeddingError, MemoryError, ToolError};
use std::fmt;
use std::time::Duration;
use tokio::time::sleep;

/// Trait for errors that can provide user-friendly suggestions
pub trait HasSuggestions {
    /// Get user-friendly suggestions for this error
    fn suggestions(&self) -> Vec<String>;
}

/// Wrapper for displaying errors with friendly formatting
pub struct FriendlyError<'a> {
    error: &'a CorvusError,
}

impl<'a> FriendlyError<'a> {
    /// Create a new friendly error wrapper
    pub fn new(error: &'a CorvusError) -> Self {
        Self { error }
    }

    /// Get the error title
    fn title(&self) -> String {
        match self.error {
            CorvusError::Completion(_) => "Completion Error".to_string(),
            CorvusError::Embedding(_) => "Embedding Error".to_string(),
            CorvusError::Memory(_) => "Memory Error".to_string(),
            CorvusError::Tool(_) => "Tool Error".to_string(),
            CorvusError::Config(_) => "Configuration Error".to_string(),
            CorvusError::Io(_) => "IO Error".to_string(),
            CorvusError::Http(_) => "Network Error".to_string(),
            CorvusError::NotFound(_) => "Not Found".to_string(),
            CorvusError::InvalidArgument(_) => "Invalid Argument".to_string(),
            CorvusError::Serde(_) => "Serialization Error".to_string(),
            CorvusError::Generic(_) => "Error".to_string(),
        }
    }
}

impl<'a> fmt::Display for FriendlyError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", style("✗").red().bold(), style(&self.title()).red().bold())?;
        writeln!(f, "  {}", style(self.error).dim())?;

        let suggestions = self.error.suggestions();
        if !suggestions.is_empty() {
            writeln!(f)?;
            writeln!(f, "  {}:", style("Suggestions").cyan().bold())?;
            for suggestion in suggestions {
                writeln!(f, "  {} {}", style("•").cyan(), suggestion)?;
            }
        }

        Ok(())
    }
}

impl HasSuggestions for CorvusError {
    fn suggestions(&self) -> Vec<String> {
        match self {
            CorvusError::Completion(e) => e.suggestions(),
            CorvusError::Embedding(e) => e.suggestions(),
            CorvusError::Memory(e) => e.suggestions(),
            CorvusError::Tool(e) => e.suggestions(),
            CorvusError::Config(msg) => config_suggestions(msg),
            CorvusError::Io(e) => io_suggestions(e),
            CorvusError::Http(msg) => http_suggestions(msg),
            CorvusError::NotFound(msg) => not_found_suggestions(msg),
            CorvusError::InvalidArgument(msg) => invalid_arg_suggestions(msg),
            CorvusError::Serde(_) => vec![
                "Check if your configuration file is valid JSON".to_string(),
                "Try running `corvus setup` to reconfigure".to_string(),
            ],
            CorvusError::Generic(_) => vec![],
        }
    }
}

impl HasSuggestions for CompletionError {
    fn suggestions(&self) -> Vec<String> {
        match self {
            CompletionError::ApiRequest(msg) => api_request_suggestions(msg),
            CompletionError::InvalidResponse(_) => vec![
                "This might be a temporary issue with the API".to_string(),
                "Try again in a few moments".to_string(),
            ],
            CompletionError::ModelNotFound(model) => vec![
                format!("Check if model '{}' is available for your account", model),
                "Try using a different model like 'gpt-4o' or 'gpt-4o-mini'".to_string(),
                "Run `corvus setup` to change your default model".to_string(),
            ],
            CompletionError::RateLimitExceeded => vec![
                "You've exceeded your API rate limit".to_string(),
                "Wait a minute before trying again".to_string(),
                "Check your OpenAI account for rate limits".to_string(),
            ],
            CompletionError::ContextWindowExceeded => vec![
                "The conversation is too long for the model".to_string(),
                "Try starting a new conversation".to_string(),
                "Use a model with a larger context window".to_string(),
            ],
            CompletionError::Streaming(_) => vec![
                "Try again without streaming".to_string(),
                "Check your network connection".to_string(),
            ],
        }
    }
}

impl HasSuggestions for EmbeddingError {
    fn suggestions(&self) -> Vec<String> {
        match self {
            EmbeddingError::ApiRequest(msg) => api_request_suggestions(msg),
            EmbeddingError::InvalidResponse(_) => vec![
                "This might be a temporary issue with the API".to_string(),
                "Try again in a few moments".to_string(),
            ],
            EmbeddingError::ModelNotFound(model) => vec![
                format!("Check if embedding model '{}' is available", model),
                "Try using 'text-embedding-3-small' instead".to_string(),
            ],
            EmbeddingError::DimensionMismatch { expected, got } => vec![
                format!("Expected embedding dimension {}, got {}", expected, got),
                "Make sure you're using the same embedding model".to_string(),
                "Try recreating your memory store".to_string(),
            ],
            EmbeddingError::TooManyDocuments { max, got } => vec![
                format!("Maximum {} documents allowed, you provided {}", max, got),
                "Try batching your documents".to_string(),
            ],
        }
    }
}

impl HasSuggestions for MemoryError {
    fn suggestions(&self) -> Vec<String> {
        match self {
            MemoryError::Storage(msg) => vec![
                format!("Storage error: {}", msg),
                "Check if you have write permissions".to_string(),
                "Make sure your disk isn't full".to_string(),
            ],
            MemoryError::ItemNotFound(_) => vec![
                "The memory item was not found".to_string(),
                "Try a different search query".to_string(),
            ],
            MemoryError::TagNotFound(_) => vec![
                "The tag was not found".to_string(),
                "Check the spelling of the tag".to_string(),
            ],
            MemoryError::VectorIndex(msg) => vec![
                format!("Vector index error: {}", msg),
                "Try rebuilding the vector index".to_string(),
            ],
            MemoryError::InvalidQuery(_) => vec![
                "Check your query syntax".to_string(),
                "Try a simpler search query".to_string(),
            ],
        }
    }
}

impl HasSuggestions for ToolError {
    fn suggestions(&self) -> Vec<String> {
        match self {
            ToolError::NotFound(_) => vec![
                "The requested tool is not available".to_string(),
                "Check that the tool is properly registered".to_string(),
            ],
            ToolError::InvalidArguments(msg) => vec![
                format!("Invalid arguments: {}", msg),
                "Check the tool documentation for correct usage".to_string(),
            ],
            ToolError::Execution(msg) => vec![
                format!("Execution failed: {}", msg),
                "Check your command syntax".to_string(),
                "Make sure you have the necessary permissions".to_string(),
            ],
            ToolError::PermissionDenied(_) => vec![
                "You don't have permission to perform this action".to_string(),
                "Check file/folder permissions".to_string(),
                "Try running with appropriate permissions".to_string(),
            ],
        }
    }
}

fn api_request_suggestions(msg: &str) -> Vec<String> {
    let mut suggestions = Vec::new();

    if msg.contains("authentication") || msg.contains("401") || msg.contains("API key") {
        suggestions.push("Check that your API key is correct".to_string());
        suggestions.push("Run `corvus setup` to reconfigure your API key".to_string());
        suggestions.push("Verify your API key hasn't expired".to_string());
    } else if msg.contains("timeout") || msg.contains("timed out") {
        suggestions.push("Check your internet connection".to_string());
        suggestions.push("Try again in a few moments".to_string());
    } else if msg.contains("429") || msg.contains("rate limit") {
        suggestions.push("You've exceeded your API rate limit".to_string());
        suggestions.push("Wait a minute before trying again".to_string());
    } else {
        suggestions.push("Check your internet connection".to_string());
        suggestions.push("Verify your API key is correct".to_string());
        suggestions.push("Check the OpenAI service status".to_string());
    }

    suggestions
}

fn config_suggestions(msg: &str) -> Vec<String> {
    let mut suggestions = vec![
        "Run `corvus setup` to reconfigure".to_string(),
    ];

    if msg.contains("API key") || msg.contains("OPENAI_API_KEY") {
        suggestions.push("Set the OPENAI_API_KEY environment variable".to_string());
        suggestions.push("Or run `corvus setup` to save your API key".to_string());
    }

    suggestions
}

fn io_suggestions(e: &std::io::Error) -> Vec<String> {
    let mut suggestions = Vec::new();

    match e.kind() {
        std::io::ErrorKind::NotFound => {
            suggestions.push("Check that the file or directory exists".to_string());
            suggestions.push("Verify the path is correct".to_string());
        }
        std::io::ErrorKind::PermissionDenied => {
            suggestions.push("Check your file/folder permissions".to_string());
            suggestions.push("Try running with appropriate permissions".to_string());
        }
        std::io::ErrorKind::ConnectionRefused => {
            suggestions.push("Check that the server is running".to_string());
            suggestions.push("Verify the port number is correct".to_string());
        }
        _ => {}
    }

    suggestions
}

fn http_suggestions(msg: &str) -> Vec<String> {
    let mut suggestions = vec![
        "Check your internet connection".to_string(),
    ];

    if msg.contains("timeout") || msg.contains("timed out") {
        suggestions.push("The request timed out. Try again".to_string());
    }

    suggestions
}

fn not_found_suggestions(msg: &str) -> Vec<String> {
    vec![
        format!("{} not found", msg),
        "Check the spelling and path".to_string(),
    ]
}

fn invalid_arg_suggestions(msg: &str) -> Vec<String> {
    vec![
        format!("Invalid argument: {}", msg),
        "Check the help documentation".to_string(),
        "Run with --help for more information".to_string(),
    ]
}

/// Print a friendly error message to stderr
pub fn print_error(error: &anyhow::Error) {
    if let Some(corvus_error) = error.downcast_ref::<CorvusError>() {
        eprintln!("\n{}\n", FriendlyError::new(corvus_error));
    } else {
        eprintln!("\n{} {}\n", style("✗").red().bold(), style(error).red());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friendly_error_display() {
        let error = CorvusError::Config("API key not set".to_string());
        let friendly = FriendlyError::new(&error);
        let display = format!("{}", friendly);
        assert!(display.contains("Configuration Error"));
        assert!(display.contains("Suggestions"));
    }

    #[test]
    fn test_config_suggestions() {
        let error = CorvusError::Config("API key not found".to_string());
        let suggestions = error.suggestions();
        assert!(!suggestions.is_empty());
    }
}

// ========== Error Recovery and Retry ==========

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries (exponential backoff)
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

/// Check if an error is retryable
pub fn is_retryable(error: &CorvusError) -> bool {
    match error {
        CorvusError::Completion(e) => match e {
            CompletionError::ApiRequest(_) => true,
            CompletionError::InvalidResponse(_) => true,
            CompletionError::RateLimitExceeded => true,
            CompletionError::Streaming(_) => true,
            CompletionError::ModelNotFound(_) => false,
            CompletionError::ContextWindowExceeded => false,
        },
        CorvusError::Http(_) => true,
        CorvusError::Io(e) => {
            e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::Interrupted
                || e.kind() == std::io::ErrorKind::ConnectionReset
        }
        _ => false,
    }
}

/// Get the recommended delay for a retry (for rate limits)
pub fn retry_delay(error: &CorvusError) -> Option<Duration> {
    match error {
        CorvusError::Completion(CompletionError::RateLimitExceeded) => {
            Some(Duration::from_secs(60))
        }
        _ => None,
    }
}

/// Retry an async operation with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;
    let mut current_delay = config.initial_delay;

    for attempt in 0..=config.max_retries {
        if attempt > 0 {
            println!(
                "{} Retrying... (attempt {}/{})",
                style("ℹ").cyan().bold(),
                attempt,
                config.max_retries
            );
            sleep(current_delay).await;
            current_delay = std::cmp::min(
                Duration::from_secs_f64(current_delay.as_secs_f64() * config.backoff_multiplier),
                config.max_delay,
            );
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                println!("{} Attempt {} failed: {}", style("⚠").yellow(), attempt + 1, e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap())
}

/// Execute an operation with fallback options
pub async fn with_fallback<F, Fut, T, E>(
    primary: F,
    fallbacks: Vec<Box<dyn Fn() -> Fut + Send + Sync>>,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    match primary().await {
        Ok(result) => return Ok(result),
        Err(_) => {
            for fallback in fallbacks {
                if let Ok(result) = fallback().await {
                    return Ok(result);
                }
            }
        }
    }

    // All fallbacks failed, try primary one more time
    primary().await
}

/// Safe wrapper for operations that can fail, with graceful degradation
pub struct SafeOperation<T> {
    result: Option<T>,
    error: Option<anyhow::Error>,
    warnings: Vec<String>,
}

impl<T> Default for SafeOperation<T> {
    fn default() -> Self {
        Self {
            result: None,
            error: None,
            warnings: Vec::new(),
        }
    }
}

impl<T> SafeOperation<T> {
    /// Create a new safe operation builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute the main operation
    pub async fn execute<F, Fut>(mut self, operation: F) -> Self
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<T>>,
    {
        match operation().await {
            Ok(result) => self.result = Some(result),
            Err(e) => self.error = Some(e),
        }
        self
    }

    /// Add a fallback operation
    pub async fn or_fallback<F, Fut>(mut self, fallback: F) -> Self
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<T>>,
    {
        if self.result.is_none() {
            if let Some(err) = &self.error {
                self.warnings.push(format!("Falling back after error: {}", err));
            }
            match fallback().await {
                Ok(result) => self.result = Some(result),
                Err(e) => self.error = Some(e),
            }
        }
        self
    }

    /// Add a default value if all else fails
    pub fn or_default(mut self, default: T) -> Self
    where
        T: Clone,
    {
        if self.result.is_none() {
            if let Some(err) = &self.error {
                self.warnings
                    .push(format!("Using default after error: {}", err));
            }
            self.result = Some(default);
        }
        self
    }

    /// Get the result, panicking if all operations failed
    pub fn unwrap(self) -> T {
        self.result.expect("All operations failed")
    }

    /// Get the result or an error
    pub fn into_result(self) -> anyhow::Result<T> {
        match self.result {
            Some(result) => Ok(result),
            None => Err(self.error.unwrap_or_else(|| anyhow::anyhow!("Operation failed"))),
        }
    }

    /// Get the warnings collected during execution
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(10));
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_safe_operation_new() {
        let op: SafeOperation<String> = SafeOperation::new();
        assert!(op.result.is_none());
        assert!(op.error.is_none());
        assert!(op.warnings.is_empty());
    }

    #[test]
    fn test_safe_operation_or_default() {
        let op: SafeOperation<String> = SafeOperation::new();
        let result = op.or_default("default value".to_string());
        // Check warnings before unwrap
        assert_eq!(result.warnings().len(), 0);
        assert_eq!(result.unwrap(), "default value");
    }
}
