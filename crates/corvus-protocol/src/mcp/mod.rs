//! MCP (Model Context Protocol) implementation
//!
//! This module provides implementation of the Model Context Protocol
//! for communicating with external tools and services.

pub mod protocol;
pub mod client;
pub mod server;
pub mod framework;
pub mod servers;

#[cfg(test)]
mod protocol_tests;

pub use client::McpClient;
pub use server::{McpServer, McpServerHandler};
pub use framework::{
    McpServerBuilder, SimpleMcpServer, RegisteredTool, RegisteredResource, RegisteredPrompt,
    ChangeListener, text_content, tool_response_text, tool_response_error,
};

use serde::{Deserialize, Serialize};

/// An MCP tool provided by a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// The name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON Schema for the tool arguments
    pub input_schema: serde_json::Value,
}
