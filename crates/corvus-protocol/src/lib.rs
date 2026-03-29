//! Corvus Protocol - MCP/ACP protocol implementation
//!
//! Implementation of the Model Context Protocol (MCP) and Agent Client Protocol (ACP)
//! for connecting with external tools and services.

#![warn(missing_docs)]

pub mod mcp;
pub mod transport;
pub mod error;

pub use mcp::{
    client::McpClient,
    server::{McpServer, McpServerHandler},
    framework::{
        McpServerBuilder, SimpleMcpServer, RegisteredTool, RegisteredResource, RegisteredPrompt,
        ChangeListener, text_content, tool_response_text, tool_response_error,
    },
    servers::{
        FilesystemServer,
    },
    protocol::{
        CallToolRequest, CallToolResponse,
        InitializeRequest, InitializeResponse,
        ListToolsRequest, ListToolsResponse,
        PingRequest, PingResponse,
        Tool, Content,
        Implementation, ServerCapabilities, ClientCapabilities,
    },
};
pub use transport::{Transport, TransportMessage, StdioTransport};
pub use error::{ProtocolError, Result};
