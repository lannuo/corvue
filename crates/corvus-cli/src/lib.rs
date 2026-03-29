//! Corvus CLI library

#![warn(missing_docs)]
#![allow(missing_docs)] // Temporarily allow missing docs

pub mod cache;
pub mod circuit_breaker;
pub mod cli;
pub mod completer;
pub mod config;
pub mod config_loader;
pub mod errors;
pub mod format;
pub mod memory_store;
pub mod chat_memory;
pub mod mcp_bridge;
pub mod session;

pub use cache::{CachedCompletionModel, ResponseCache};
pub use chat_memory::{ChatMemory, MemoryStats};
pub use cli::{Cli, Commands, ConfigCommands, McpCommands, ModelCommands, SessionCommands};
pub use config::Config;
pub use errors::{print_error, FriendlyError, HasSuggestions};
pub use format::*;
pub use memory_store::{TagMemoStore, StoredMemory};
pub use session::{ChatMessage, ChatSession, SessionExport, SessionStorage};
