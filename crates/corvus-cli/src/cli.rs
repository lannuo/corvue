//! CLI command definitions

use clap::Parser;
use std::path::PathBuf;

/// Corvus AI Agent CLI
#[derive(Parser, Debug)]
#[command(name = "corvus", about = "Corvus AI Agent - Intelligent coding assistant", version = "0.1.0")]
pub struct Cli {
    /// Path to config file
    #[arg(long, short = 'c', global = true)]
    pub config: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    /// The command to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Parser, Debug)]
pub enum Commands {
    /// Start interactive chat mode
    Chat(ChatArgs),

    /// Run a single task
    Run(RunArgs),

    /// Configuration management commands
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Interactive setup wizard
    Setup,

    /// Session management commands
    #[command(subcommand)]
    Session(SessionCommands),

    /// Model management commands
    #[command(subcommand)]
    Model(ModelCommands),

    /// MCP server management commands
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Plugin management commands
    #[command(subcommand)]
    Plugin(PluginCommands),

    /// Memory management commands
    #[command(subcommand)]
    Memory(MemoryCommands),
}

/// Configuration management subcommands
#[derive(Parser, Debug)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Export configuration to a file
    Export(ConfigExportArgs),
    /// Import configuration from a file
    Import(ConfigImportArgs),
    /// Set a configuration value
    Set(ConfigSetArgs),
    /// Reset configuration to defaults
    Reset,
}

/// Arguments for exporting config
#[derive(Parser, Debug)]
pub struct ConfigExportArgs {
    /// Output file path
    pub output: std::path::PathBuf,
}

/// Arguments for importing config
#[derive(Parser, Debug)]
pub struct ConfigImportArgs {
    /// Input file path
    pub input: std::path::PathBuf,
    /// Merge with existing config instead of replacing
    #[arg(long, short = 'm')]
    pub merge: bool,
}

/// Arguments for setting a config value
#[derive(Parser, Debug)]
pub struct ConfigSetArgs {
    /// Configuration key (e.g., default_model, temperature)
    pub key: String,
    /// Configuration value
    pub value: String,
}

/// MCP server management subcommands
#[derive(Parser, Debug)]
pub enum McpCommands {
    /// List configured MCP servers
    List,
    /// Add an MCP server
    Add(McpAddArgs),
    /// Remove an MCP server
    Remove(McpRemoveArgs),
    /// Test connection to an MCP server
    Test(McpTestArgs),
}

/// Arguments for adding an MCP server
#[derive(Parser, Debug)]
pub struct McpAddArgs {
    /// Server name
    pub name: String,
    /// Command to execute
    pub command: String,
    /// Arguments for the command
    #[arg(last = true)]
    pub args: Vec<String>,
}

/// Arguments for removing an MCP server
#[derive(Parser, Debug)]
pub struct McpRemoveArgs {
    /// Server name
    pub name: String,
}

/// Arguments for testing an MCP server
#[derive(Parser, Debug)]
pub struct McpTestArgs {
    /// Server name
    pub name: String,
}

/// Plugin management subcommands
#[derive(Parser, Debug)]
pub enum PluginCommands {
    /// List installed plugins
    List,
    /// Install a plugin
    Install(PluginInstallArgs),
    /// Uninstall a plugin
    Uninstall(PluginUninstallArgs),
    /// Enable a plugin
    Enable(PluginEnableArgs),
    /// Disable a plugin
    Disable(PluginDisableArgs),
}

/// Arguments for installing a plugin
#[derive(Parser, Debug)]
pub struct PluginInstallArgs {
    /// Path or URL to plugin
    pub path: String,
}

/// Arguments for uninstalling a plugin
#[derive(Parser, Debug)]
pub struct PluginUninstallArgs {
    /// Plugin name
    pub name: String,
}

/// Arguments for enabling a plugin
#[derive(Parser, Debug)]
pub struct PluginEnableArgs {
    /// Plugin name
    pub name: String,
}

/// Arguments for disabling a plugin
#[derive(Parser, Debug)]
pub struct PluginDisableArgs {
    /// Plugin name
    pub name: String,
}

/// Memory management subcommands
#[derive(Parser, Debug)]
pub enum MemoryCommands {
    /// List memories
    List(MemoryListArgs),
    /// Search memories
    Search(MemorySearchArgs),
    /// Export memories
    Export(MemoryExportArgs),
    /// Import memories
    Import(MemoryImportArgs),
    /// Delete a memory
    Delete(MemoryDeleteArgs),
}

/// Arguments for listing memories
#[derive(Parser, Debug)]
pub struct MemoryListArgs {
    /// Maximum number of memories to show
    #[arg(long, short = 'n', default_value = "20")]
    pub limit: usize,
}

/// Arguments for searching memories
#[derive(Parser, Debug)]
pub struct MemorySearchArgs {
    /// Search query
    pub query: String,
    /// Maximum number of results
    #[arg(long, short = 'n', default_value = "10")]
    pub limit: usize,
}

/// Arguments for exporting memories
#[derive(Parser, Debug)]
pub struct MemoryExportArgs {
    /// Output file path
    pub output: std::path::PathBuf,
}

/// Arguments for importing memories
#[derive(Parser, Debug)]
pub struct MemoryImportArgs {
    /// Input file path
    pub input: std::path::PathBuf,
}

/// Arguments for deleting a memory
#[derive(Parser, Debug)]
pub struct MemoryDeleteArgs {
    /// Memory ID to delete
    pub memory_id: String,
}

/// Session management subcommands
#[derive(Parser, Debug)]
pub enum SessionCommands {
    /// List all sessions
    List(SessionListArgs),

    /// Continue a previous session
    Continue(SessionContinueArgs),

    /// Show session details
    Show(SessionShowArgs),

    /// Rename a session
    Rename(SessionRenameArgs),

    /// Delete a session
    Delete(SessionDeleteArgs),

    /// Search sessions
    Search(SessionSearchArgs),

    /// Export a session to JSON
    Export(SessionExportArgs),

    /// Import a session from JSON
    Import(SessionImportArgs),
}

/// Arguments for exporting a session
#[derive(Parser, Debug)]
pub struct SessionExportArgs {
    /// Session ID to export
    pub session_id: String,
    /// Output file path (optional)
    #[arg(long, short = 'o')]
    pub output: Option<std::path::PathBuf>,
}

/// Arguments for importing a session
#[derive(Parser, Debug)]
pub struct SessionImportArgs {
    /// Input file path
    pub input: std::path::PathBuf,
}

/// Model management subcommands
#[derive(Parser, Debug)]
pub enum ModelCommands {
    /// List available models
    List,
    /// Show current model
    Current,
    /// Set default model
    Use(ModelUseArgs),
}

/// Arguments for setting the default model
#[derive(Parser, Debug)]
pub struct ModelUseArgs {
    /// Model name to use
    pub model: String,
}

/// Arguments for listing sessions
#[derive(Parser, Debug)]
pub struct SessionListArgs {
    /// Maximum number of sessions to show
    #[arg(long, short = 'n', default_value = "10")]
    pub limit: usize,
}

/// Arguments for continuing a session
#[derive(Parser, Debug)]
pub struct SessionContinueArgs {
    /// Session ID to continue (defaults to last session)
    pub session_id: Option<String>,
}

/// Arguments for showing a session
#[derive(Parser, Debug)]
pub struct SessionShowArgs {
    /// Session ID to show
    pub session_id: String,
}

/// Arguments for renaming a session
#[derive(Parser, Debug)]
pub struct SessionRenameArgs {
    /// Session ID to rename
    pub session_id: String,
    /// New name for the session
    pub new_name: String,
}

/// Arguments for deleting a session
#[derive(Parser, Debug)]
pub struct SessionDeleteArgs {
    /// Session ID to delete
    pub session_id: String,
}

/// Arguments for searching sessions
#[derive(Parser, Debug)]
pub struct SessionSearchArgs {
    /// Search query
    pub query: String,
    /// Maximum number of results
    #[arg(long, short = 'n', default_value = "10")]
    pub limit: usize,
}

/// Arguments for chat mode
#[derive(Parser, Debug)]
pub struct ChatArgs {
    /// Model to use
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Continue a specific session
    #[arg(long, short = 's')]
    pub session: Option<String>,

    /// Initial prompt
    pub prompt: Vec<String>,
}

/// Arguments for run mode
#[derive(Parser, Debug)]
pub struct RunArgs {
    /// Model to use
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// The task to execute
    pub task: Vec<String>,
}
