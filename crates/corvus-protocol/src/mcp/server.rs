//! MCP server implementation
//!
//! Provides a framework for implementing MCP servers.

use crate::error::{ProtocolError, Result};
use crate::mcp::protocol::*;
use async_trait::async_trait;
use serde_json::Value;

/// Handler for MCP server requests
#[async_trait]
pub trait McpServerHandler: Send + Sync {
    /// Get server information
    fn server_info(&self) -> Implementation;

    /// Get server capabilities
    fn capabilities(&self) -> ServerCapabilities;

    /// Get server instructions
    fn instructions(&self) -> Option<String> {
        None
    }

    /// Handle initialize request
    async fn on_initialize(&self, _request: InitializeRequest) -> Result<InitializeResponse> {
        Ok(InitializeResponse {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities(),
            server_info: self.server_info(),
            instructions: self.instructions(),
        })
    }

    /// Handle ping request
    async fn on_ping(&self) -> Result<PingResponse> {
        Ok(PingResponse {})
    }

    /// Handle list tools request
    async fn on_list_tools(&self, _request: ListToolsRequest) -> Result<ListToolsResponse> {
        Ok(ListToolsResponse {
            tools: vec![],
            next_cursor: None,
        })
    }

    /// Handle call tool request
    async fn on_call_tool(&self, _request: CallToolRequest) -> Result<CallToolResponse> {
        Err(ProtocolError::MethodNotFound("call_tool".to_string()))
    }

    /// Handle list resources request
    async fn on_list_resources(
        &self,
        _request: ListResourcesRequest,
    ) -> Result<ListResourcesResponse> {
        Ok(ListResourcesResponse {
            resources: vec![],
            next_cursor: None,
        })
    }

    /// Handle read resource request
    async fn on_read_resource(&self, _request: ReadResourceRequest) -> Result<ReadResourceResponse> {
        Err(ProtocolError::MethodNotFound("read_resource".to_string()))
    }

    /// Handle list prompts request
    async fn on_list_prompts(&self, _request: ListPromptsRequest) -> Result<ListPromptsResponse> {
        Ok(ListPromptsResponse {
            prompts: vec![],
            next_cursor: None,
        })
    }

    /// Handle get prompt request
    async fn on_get_prompt(&self, _request: GetPromptRequest) -> Result<GetPromptResponse> {
        Err(ProtocolError::MethodNotFound("get_prompt".to_string()))
    }
}

/// MCP server implementation
pub struct McpServer {
    handler: Box<dyn McpServerHandler>,
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server with the given handler
    pub fn new(handler: Box<dyn McpServerHandler>) -> Self {
        Self {
            handler,
            initialized: false,
        }
    }

    /// Handle a JSON-RPC request
    pub async fn handle_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let id = request.id.clone();

        // Check if we need to initialize first
        if request.method != "initialize" && !self.initialized {
            return Ok(JsonRpcResponse::error(
                id,
                -32000,
                "Not initialized".to_string(),
                None,
            ));
        }

        let result = match request.method.as_str() {
            "initialize" => {
                let params = request
                    .params
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing params".to_string()))?;
                let init_request: InitializeRequest = serde_json::from_value(params)?;
                let response = self.handler.on_initialize(init_request).await?;
                self.initialized = true;
                serde_json::to_value(response)?
            }
            "ping" => {
                let response = self.handler.on_ping().await?;
                serde_json::to_value(response)?
            }
            "tools/list" => {
                let params = request.params.unwrap_or(Value::Null);
                let list_request: ListToolsRequest = serde_json::from_value(params)?;
                let response = self.handler.on_list_tools(list_request).await?;
                serde_json::to_value(response)?
            }
            "tools/call" => {
                let params = request
                    .params
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing params".to_string()))?;
                let call_request: CallToolRequest = serde_json::from_value(params)?;
                let response = self.handler.on_call_tool(call_request).await?;
                serde_json::to_value(response)?
            }
            "resources/list" => {
                let params = request.params.unwrap_or(Value::Null);
                let list_request: ListResourcesRequest = serde_json::from_value(params)?;
                let response = self.handler.on_list_resources(list_request).await?;
                serde_json::to_value(response)?
            }
            "resources/read" => {
                let params = request
                    .params
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing params".to_string()))?;
                let read_request: ReadResourceRequest = serde_json::from_value(params)?;
                let response = self.handler.on_read_resource(read_request).await?;
                serde_json::to_value(response)?
            }
            "prompts/list" => {
                let params = request.params.unwrap_or(Value::Null);
                let list_request: ListPromptsRequest = serde_json::from_value(params)?;
                let response = self.handler.on_list_prompts(list_request).await?;
                serde_json::to_value(response)?
            }
            "prompts/get" => {
                let params = request
                    .params
                    .ok_or_else(|| ProtocolError::InvalidParams("Missing params".to_string()))?;
                let get_request: GetPromptRequest = serde_json::from_value(params)?;
                let response = self.handler.on_get_prompt(get_request).await?;
                serde_json::to_value(response)?
            }
            _ => {
                return Ok(JsonRpcResponse::error(
                    id,
                    -32601,
                    format!("Method not found: {}", request.method),
                    None,
                ));
            }
        };

        Ok(JsonRpcResponse::success(id, result))
    }

    /// Check if the server is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
