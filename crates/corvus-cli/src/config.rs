//! Configuration management

use console::style;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration wizard for interactive setup
pub struct ConfigWizard;

impl ConfigWizard {
    /// Run the interactive setup wizard
    pub async fn run() -> anyhow::Result<Config> {
        use console::style;

        println!("{}", style("╔════════════════════════════════════════╗").green());
        println!("{}", style("║     Corvus Setup Wizard                 ║").green());
        println!("{}", style("╚════════════════════════════════════════╝").green());

        let mut config = Config::load();

        println!("\nLet's configure your Corvus setup!\n");

        // Select Provider
        println!("{} Select your AI provider:", style("?").cyan().bold());
        println!("  1) OpenAI (Recommended)");
        println!("  2) Ollama (Local models)");
        print!("  Your choice [1]: ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        config.provider = match input.trim() {
            "2" => ProviderType::Ollama,
            _ => ProviderType::OpenAI,
        };

        // Provider-specific configuration
        match config.provider {
            ProviderType::OpenAI => {
                // API Key
                let has_api_key = config.openai_api_key.is_some();
                if has_api_key {
                    print!("{} OpenAI API key is already configured. Update it? [y/N]: ", style("?").cyan().bold());
                    std::io::Write::flush(&mut std::io::stdout())?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("y") {
                        config.openai_api_key = Some(Self::prompt_api_key()?);
                    }
                } else {
                    config.openai_api_key = Some(Self::prompt_api_key()?);
                }

                // Default Model
                println!("\n{} Select your default model:", style("?").cyan().bold());
                println!("  1) gpt-4o (Recommended)");
                println!("  2) gpt-4o-mini (Fast, affordable)");
                println!("  3) gpt-4-turbo");
                println!("  4) gpt-3.5-turbo");
                print!("  Your choice [1]: ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                config.default_model = match input.trim() {
                    "2" => "gpt-4o-mini".to_string(),
                    "3" => "gpt-4-turbo".to_string(),
                    "4" => "gpt-3.5-turbo".to_string(),
                    _ => "gpt-4o".to_string(),
                };
            }
            ProviderType::Ollama => {
                // Ollama base URL
                let has_ollama_url = config.ollama_base_url.is_some();
                if has_ollama_url {
                    print!("{} Ollama base URL is already configured. Update it? [y/N]: ", style("?").cyan().bold());
                    std::io::Write::flush(&mut std::io::stdout())?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("y") {
                        config.ollama_base_url = Some(Self::prompt_ollama_url()?);
                    }
                } else {
                    config.ollama_base_url = Some(Self::prompt_ollama_url()?);
                }

                // Default Model for Ollama
                println!("\n{} Select your default model:", style("?").cyan().bold());
                println!("  1) llama3.1:8b (Recommended)");
                println!("  2) llama3:8b");
                println!("  3) mistral:7b");
                println!("  4) gemma:2b");
                println!("  5) codellama:7b");
                println!("  6) Custom (enter your own)");
                print!("  Your choice [1]: ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                config.default_model = match input.trim() {
                    "2" => "llama3:8b".to_string(),
                    "3" => "mistral:7b".to_string(),
                    "4" => "gemma:2b".to_string(),
                    "5" => "codellama:7b".to_string(),
                    "6" => {
                        print!("  Enter model name: ");
                        std::io::Write::flush(&mut std::io::stdout())?;
                        let mut custom = String::new();
                        std::io::stdin().read_line(&mut custom)?;
                        custom.trim().to_string()
                    }
                    _ => "llama3.1:8b".to_string(),
                };
            }
        }

        // Temperature
        print!("\n{} Set temperature (0.0-2.0, lower = more deterministic) [{}]: ", style("?").cyan().bold(), config.temperature);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if let Ok(t) = input.trim().parse::<f32>() {
            if (0.0..=2.0).contains(&t) {
                config.temperature = t;
            }
        }

        // Max iterations
        print!("\n{} Set max tool call iterations [{}]: ", style("?").cyan().bold(), config.max_iterations);
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if let Ok(n) = input.trim().parse::<u32>() {
            if n >= 1 {
                config.max_iterations = n;
            }
        }

        // Use memory
        print!("\n{} Enable memory system? [Y/n]: ", style("?").cyan().bold());
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        config.use_memory = !input.trim().eq_ignore_ascii_case("n");

        // Use cache
        print!("\n{} Enable response caching? [Y/n]: ", style("?").cyan().bold());
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        config.use_cache = !input.trim().eq_ignore_ascii_case("n");

        // Save config
        println!();
        config.save()?;

        println!("{}", style("✓ Configuration saved successfully!").green());
        println!("\nConfig file: {}", Config::config_file().display());
        println!("\nYou're all set! Run `corvus chat` to start.");

        Ok(config)
    }

    /// Quick setup with minimal prompts
    pub async fn quick_setup() -> anyhow::Result<Config> {
        let mut config = Config::load();

        if config.openai_api_key.is_none() {
            println!("{}", style("╔════════════════════════════════════════╗").green());
            println!("{}", style("║     Corvus Quick Setup                  ║").green());
            println!("{}", style("╚════════════════════════════════════════╝").green());
            config.openai_api_key = Some(Self::prompt_api_key()?);
            config.save()?;
            println!("{}", style("✓ Configuration saved!").green());
        }

        Ok(config)
    }

    fn prompt_api_key() -> anyhow::Result<String> {
        use console::style;
        loop {
            print!("{} Enter your OpenAI API key: ", style("?").cyan().bold());
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let key = input.trim().to_string();
            if !key.is_empty() {
                return Ok(key);
            }
            println!("{} API key cannot be empty. Please try again.", style("✗").red());
        }
    }

    fn prompt_ollama_url() -> anyhow::Result<String> {
        use console::style;
        print!("{} Enter Ollama base URL [http://localhost:11434/v1]: ", style("?").cyan().bold());
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let url = input.trim().to_string();
        if url.is_empty() {
            Ok("http://localhost:11434/v1".to_string())
        } else {
            Ok(url)
        }
    }
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server name
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum ProviderType {
    /// OpenAI API
    #[default]
    OpenAI,
    /// Ollama (local)
    Ollama,
}


/// Configuration for Corvus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default provider to use
    #[serde(default)]
    pub provider: ProviderType,
    /// Default model to use
    pub default_model: String,
    /// OpenAI API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Ollama base URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ollama_base_url: Option<String>,
    /// Temperature
    pub temperature: f32,
    /// Max iterations
    pub max_iterations: u32,
    /// Whether to use memory
    pub use_memory: bool,
    /// Whether to enable response caching
    #[serde(default = "default_true")]
    pub use_cache: bool,
    /// Context window size (in tokens)
    #[serde(default = "default_context_window_size")]
    pub context_window_size: usize,
    /// MCP servers to connect to
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

fn default_context_window_size() -> usize {
    context_window::DEFAULT_WINDOW_SIZE
}

fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderType::OpenAI,
            default_model: "gpt-4o".to_string(),
            openai_api_key: None,
            ollama_base_url: None,
            temperature: 0.7,
            max_iterations: 20,
            use_memory: true,
            use_cache: true,
            context_window_size: context_window::DEFAULT_WINDOW_SIZE,
            mcp_servers: Vec::new(),
        }
    }
}

impl Config {
    /// Get the config directory path
    pub fn config_dir() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("corvus")
    }

    /// Get the config file path
    pub fn config_file() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    /// Load config from file
    pub fn load() -> Self {
        let path = Self::config_file();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save config to file
    pub fn save(&self) -> std::io::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::config_file(), content)?;

        Ok(())
    }

    // ========== Configuration Validation ==========

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        match self.provider {
            ProviderType::OpenAI => {
                if self.openai_api_key.is_none() && std::env::var("OPENAI_API_KEY").is_err() {
                    return Err(ConfigValidationError::MissingApiKey);
                }
            }
            ProviderType::Ollama => {
                if self.ollama_base_url.is_none() {
                    return Err(ConfigValidationError::MissingOllamaUrl);
                }
            }
        }

        Ok(())
    }

    /// Export configuration to a file
    pub fn export_to_file(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Import configuration from a file
    pub fn import_from_file(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Merge another configuration into this one
    pub fn merge(&mut self, other: Self) {
        self.provider = other.provider;
        self.default_model = other.default_model;
        if other.openai_api_key.is_some() {
            self.openai_api_key = other.openai_api_key;
        }
        if other.ollama_base_url.is_some() {
            self.ollama_base_url = other.ollama_base_url;
        }
        self.temperature = other.temperature;
        self.max_iterations = other.max_iterations;
        self.use_memory = other.use_memory;
        self.use_cache = other.use_cache;
        self.context_window_size = other.context_window_size;
        if !other.mcp_servers.is_empty() {
            self.mcp_servers = other.mcp_servers;
        }
    }
}

/// Configuration validation error
#[derive(Debug)]
pub enum ConfigValidationError {
    /// Missing OpenAI API key
    MissingApiKey,
    /// Missing Ollama base URL
    MissingOllamaUrl,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValidationError::MissingApiKey => write!(f, "Missing OpenAI API key"),
            ConfigValidationError::MissingOllamaUrl => write!(f, "Missing Ollama base URL"),
        }
    }
}

impl std::error::Error for ConfigValidationError {}

// ========== Prompt Templates ==========

/// A predefined prompt template
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// The system prompt
    pub system_prompt: String,
}

/// Built-in prompt templates
pub fn builtin_templates() -> Vec<PromptTemplate> {
    vec![
        PromptTemplate {
            name: "default".to_string(),
            description: "Default coding assistant".to_string(),
            system_prompt: "You are Corvus, an intelligent AI assistant. You help with coding tasks and general questions. You have access to these tools: execute_code (run code), file_operations (read/write files), shell_exec (run shell commands), and git_operations (git commands).".to_string(),
        },
        PromptTemplate {
            name: "code-reviewer".to_string(),
            description: "Code review specialist".to_string(),
            system_prompt: "You are Corvus, a code review specialist. You carefully review code for bugs, security issues, style issues, and performance improvements. Be constructive and detailed in your feedback.".to_string(),
        },
        PromptTemplate {
            name: "explainer".to_string(),
            description: "Concept explainer".to_string(),
            system_prompt: "You are Corvus, a helpful explainer. You break down complex concepts into simple, easy-to-understand explanations. Use examples and analogies when helpful.".to_string(),
        },
        PromptTemplate {
            name: "debugger".to_string(),
            description: "Debugging assistant".to_string(),
            system_prompt: "You are Corvus, a debugging expert. You methodically analyze problems, ask clarifying questions, and help identify root causes. Be systematic and thorough.".to_string(),
        },
    ]
}

/// Get a prompt template by name
pub fn get_template(name: &str) -> Option<PromptTemplate> {
    builtin_templates().into_iter().find(|t| t.name == name)
}

// ========== Debug/Diagnostic Mode ==========

/// Debug mode configuration
#[derive(Debug, Clone, Default)]
pub struct DebugConfig {
    /// Enable debug output
    pub enabled: bool,
    /// Show token counts
    pub show_tokens: bool,
    /// Show API calls
    pub show_api_calls: bool,
    /// Show timing information
    pub show_timing: bool,
}

impl DebugConfig {
    /// Create a new debug config with all options enabled
    pub fn full() -> Self {
        Self {
            enabled: true,
            show_tokens: true,
            show_api_calls: true,
            show_timing: true,
        }
    }
}

/// Simple token counter (approximate)
pub fn count_tokens(text: &str) -> usize {
    // Very rough approximation: 1 token ~= 4 chars
    text.chars().count() / 4
}

/// Context window management
pub mod context_window {
    use super::*;

    /// Default context window size (tokens)
    pub const DEFAULT_WINDOW_SIZE: usize = 8192;

    /// A simple message trait for context window management
    pub trait HasContent {
        /// Get the role of the message
        fn role(&self) -> &str;
        /// Get the content of the message
        fn content(&self) -> &str;
    }

    /// Truncate conversation to fit context window (generic version)
    pub fn truncate_conversation_generic<M: HasContent + Clone>(
        messages: &[M],
        max_tokens: usize,
    ) -> Vec<M> {
        let mut result = Vec::new();
        let mut total_tokens = 0;

        // Always keep system messages if present
        let (system_messages, other_messages): (Vec<_>, Vec<_>) = messages
            .iter()
            .cloned()
            .partition(|m| m.role() == "system");

        result.extend(system_messages);

        // Add other messages from most recent to oldest
        for msg in other_messages.into_iter().rev() {
            let msg_tokens = count_tokens(msg.content());
            if total_tokens + msg_tokens <= max_tokens {
                result.push(msg);
                total_tokens += msg_tokens;
            } else {
                break;
            }
        }

        // Reverse to get back to correct order (system first, then chronological)
        let (systems, others): (Vec<_>, Vec<_>) = result
            .into_iter()
            .partition(|m| m.role() == "system");

        let mut final_result = systems;
        final_result.extend(others.into_iter().rev());
        final_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.provider, ProviderType::OpenAI);
        assert_eq!(config.default_model, "gpt-4o");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_iterations, 20);
        assert!(config.use_memory);
    }

    #[test]
    fn test_config_merge() {
        let mut config = Config::default();
        let mut other = Config::default();

        other.default_model = "gpt-4o-mini".to_string();
        other.temperature = 0.5;

        config.merge(other);

        assert_eq!(config.default_model, "gpt-4o-mini");
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_provider_type_default() {
        let provider = ProviderType::default();
        assert_eq!(provider, ProviderType::OpenAI);
    }

    #[test]
    fn test_builtin_templates() {
        let templates = builtin_templates();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|t| t.name == "default"));
        assert!(templates.iter().any(|t| t.name == "code-reviewer"));
    }

    #[test]
    fn test_get_template() {
        let template = get_template("default");
        assert!(template.is_some());
        assert_eq!(template.unwrap().name, "default");

        let template = get_template("nonexistent");
        assert!(template.is_none());
    }

    #[test]
    fn test_count_tokens() {
        let text = "Hello, world!";
        let tokens = count_tokens(text);
        assert!(tokens > 0);
    }

    #[test]
    fn test_context_window_default_size() {
        use super::context_window::*;
        // Just test that the module compiles and DEFAULT_WINDOW_SIZE exists
        assert_eq!(DEFAULT_WINDOW_SIZE, 8192);
    }

    #[test]
    fn test_debug_config_full() {
        let debug = DebugConfig::full();
        assert!(debug.enabled);
        assert!(debug.show_tokens);
        assert!(debug.show_api_calls);
        assert!(debug.show_timing);
    }

    #[test]
    fn test_debug_config_default() {
        let debug = DebugConfig::default();
        assert!(!debug.enabled);
        assert!(!debug.show_tokens);
        assert!(!debug.show_api_calls);
        assert!(!debug.show_timing);
    }

    #[test]
    fn test_mcp_server_config() {
        let server = McpServerConfig {
            name: "test-server".to_string(),
            command: "test-command".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string()],
        };

        assert_eq!(server.name, "test-server");
        assert_eq!(server.command, "test-command");
        assert_eq!(server.args.len(), 2);
    }

    #[test]
    fn test_prompt_template() {
        let template = PromptTemplate {
            name: "test".to_string(),
            description: "Test template".to_string(),
            system_prompt: "You are a test assistant.".to_string(),
        };

        assert_eq!(template.name, "test");
        assert_eq!(template.description, "Test template");
        assert_eq!(template.system_prompt, "You are a test assistant.");
    }
}
