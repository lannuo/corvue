//! Corvus Core - Trait definitions and core abstractions
//!
//! This crate provides the foundational traits and types for the Corvus AI Agent system.

#![warn(missing_docs)]
#![allow(missing_docs)] // Temporarily allow missing docs for release

pub mod agent;
pub mod completion;
pub mod embedding;
pub mod error;
pub mod memory;
pub mod plugin;
pub mod tool;
pub mod types;
#[cfg(test)]
mod tests;

// Public re-exports
pub use agent::{Agent, AgentBuilder};
pub use completion::{
    CompletionDelta, CompletionModel, CompletionRequest, CompletionResponse, Choice,
    MessageDelta, ToolCallDelta, Usage,
};
pub use embedding::{Embedding, EmbeddingModel, EmbeddingSimilarity};
pub use error::{CorvusError, Result};
pub use memory::{
    ContentType, EnhancedQuery, MemoryItem, MemoryQuery, MemoryResult, MemorySystem,
    ScoredTag, Tag, TagSource,
};
pub use tool::{Tool, ToolCall, ToolDefinition, ToolResult};
pub use types::{Message, Role};
pub use plugin::{Plugin, PluginManager, PluginMetadata};
