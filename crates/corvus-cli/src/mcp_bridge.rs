//! MCP (Model Context Protocol) bridge
//!
//! Bridges MCP servers and tools to Corvus's tool system.

use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use corvus_protocol::{
    mcp::McpClient,
    Content, InitializeResponse, Tool as McpTool,
};
use corvus_protocol::transport::StdioTransport;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// An MCP tool wrapper that implements the Corvus Tool trait
pub struct McpToolBridge {
    /// The MCP tool definition
    tool: McpTool,
    /// Client for the MCP server
    client: Arc<Mutex<McpClient>>,
}

impl McpToolBridge {
    /// Create a new MCP tool bridge
    pub fn new(tool: McpTool, client: Arc<Mutex<McpClient>>) -> Self {
        Self { tool, client }
    }

    /// Convert MCP Content to a string
    fn content_to_string(content: &[Content]) -> String {
        let mut result = String::new();
        for item in content {
            match item {
                Content::Text { text } => {
                    result.push_str(text);
                    result.push('\n');
                }
                Content::Image { .. } => {
                    result.push_str("[Image content]\n");
                }
                Content::EmbeddedResource { uri, text, .. } => {
                    result.push_str(&format!("Resource: {}\n", uri));
                    if let Some(t) = text {
                        result.push_str(t);
                        result.push('\n');
                    }
                }
            }
        }
        result.trim_end().to_string()
    }
}

#[async_trait::async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        &self.tool.name
    }

    fn description(&self) -> &str {
        &self.tool.description
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.tool.name.clone(),
            self.tool.description.clone(),
            self.tool.input_schema.clone(),
        )
    }

    async fn call(&self, arguments: Value) -> corvus_core::error::Result<ToolResult> {
        let client = self.client.lock().await;
        let response = client
            .call_tool(self.tool.name.clone(), arguments)
            .await;

        match response {
            Ok(call_response) => {
                let content = Self::content_to_string(&call_response.content);
                if call_response.is_error.unwrap_or(false) {
                    Ok(ToolResult::error("", content))
                } else {
                    Ok(ToolResult::success("", content))
                }
            }
            Err(e) => Ok(ToolResult::error("", format!("MCP tool error: {}", e))),
        }
    }
}

/// Manager for MCP servers
pub struct McpServerManager {
    /// Connected servers
    servers: HashMap<String, Arc<Mutex<McpClient>>>,
}

impl McpServerManager {
    /// Create a new MCP server manager
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    /// Connect to an MCP server via stdio
    pub async fn connect_stdio(
        &mut self,
        name: String,
        command: String,
        args: Vec<String>,
    ) -> anyhow::Result<InitializeResponse> {
        let transport = StdioTransport::spawn(&command, &args).await?;
        let mut client = McpClient::new(transport);

        let init_response = client
            .initialize("corvus".to_string(), env!("CARGO_PKG_VERSION").to_string())
            .await?;

        self.servers.insert(name, Arc::new(Mutex::new(client)));

        Ok(init_response)
    }

    /// Get all tools from all connected servers
    pub async fn all_tools(&mut self) -> anyhow::Result<Vec<McpToolBridge>> {
        let mut tools = Vec::new();

        for client in self.servers.values() {
            let client_clone = Arc::clone(client);
            let mut client_guard = client.lock().await;
            let server_tools = client_guard.list_tools().await?;
            for tool in server_tools {
                tools.push(McpToolBridge::new(tool.clone(), Arc::clone(&client_clone)));
            }
        }

        Ok(tools)
    }

    /// Get a client by name
    pub fn get_client(&self, name: &str) -> Option<&Arc<Mutex<McpClient>>> {
        self.servers.get(name)
    }

    /// Get a mutable client by name
    pub fn get_client_mut(&mut self, name: &str) -> Option<&mut Arc<Mutex<McpClient>>> {
        self.servers.get_mut(name)
    }

    /// List all connected servers
    pub fn list_servers(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }
}

impl Default for McpServerManager {
    fn default() -> Self {
        Self::new()
    }
}
