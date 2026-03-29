//! MCP Server Framework
//!
//! Provides a convenient Rust framework for building MCP servers with
//! automatic tool registration, resource change notifications, and authentication.

use crate::error::{ProtocolError, Result};
use crate::mcp::server::McpServerHandler;
use crate::mcp::protocol::*;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A registered tool
#[derive(Clone)]
pub struct RegisteredTool {
    /// Tool definition
    pub tool: Tool,
    /// Handler function
    pub handler: Arc<dyn Fn(Value) -> Result<CallToolResponse> + Send + Sync>,
}

/// A registered resource
#[derive(Clone)]
pub struct RegisteredResource {
    /// Resource definition
    pub resource: Resource,
    /// Read handler
    pub read_handler: Option<Arc<dyn Fn() -> Result<ResourceContents> + Send + Sync>>,
}

/// A registered prompt
#[derive(Clone)]
pub struct RegisteredPrompt {
    /// Prompt definition
    pub prompt: Prompt,
    /// Get handler
    pub get_handler: Option<Arc<dyn Fn(Value) -> Result<GetPromptResponse> + Send + Sync>>,
}

/// Simple MCP server builder
pub struct McpServerBuilder {
    /// Server name
    name: String,
    /// Server version
    version: String,
    /// Server instructions
    instructions: Option<String>,
    /// Registered tools
    tools: HashMap<String, RegisteredTool>,
    /// Registered resources
    resources: HashMap<String, RegisteredResource>,
    /// Registered prompts
    prompts: HashMap<String, RegisteredPrompt>,
    /// Server capabilities
    capabilities: ServerCapabilities,
}

impl McpServerBuilder {
    /// Create a new server builder
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            instructions: None,
            tools: HashMap::new(),
            resources: HashMap::new(),
            prompts: HashMap::new(),
            capabilities: ServerCapabilities::default(),
        }
    }

    /// Set server instructions
    pub fn with_instructions(mut self, instructions: String) -> Self {
        self.instructions = Some(instructions);
        self
    }

    /// Register a tool
    pub fn register_tool<F>(mut self, tool: Tool, handler: F) -> Self
    where
        F: Fn(Value) -> Result<CallToolResponse> + Send + Sync + 'static,
    {
        let name = tool.name.clone();
        self.tools.insert(
            name.clone(),
            RegisteredTool {
                tool,
                handler: Arc::new(handler),
            },
        );

        // Update capabilities
        if self.capabilities.tools.is_none() {
            self.capabilities.tools = Some(ToolsCapabilities { list_changed: None });
        }

        self
    }

    /// Register a resource
    pub fn register_resource<F>(mut self, resource: Resource, read_handler: F) -> Self
    where
        F: Fn() -> Result<ResourceContents> + Send + Sync + 'static,
    {
        let uri = resource.uri.clone();
        self.resources.insert(
            uri.clone(),
            RegisteredResource {
                resource,
                read_handler: Some(Arc::new(read_handler)),
            },
        );

        // Update capabilities
        if self.capabilities.resources.is_none() {
            self.capabilities.resources = Some(ResourcesCapabilities {
                subscribe: None,
                list_changed: None,
            });
        }

        self
    }

    /// Register a prompt
    pub fn register_prompt<F>(mut self, prompt: Prompt, get_handler: F) -> Self
    where
        F: Fn(Value) -> Result<GetPromptResponse> + Send + Sync + 'static,
    {
        let name = prompt.name.clone();
        self.prompts.insert(
            name.clone(),
            RegisteredPrompt {
                prompt,
                get_handler: Some(Arc::new(get_handler)),
            },
        );

        // Update capabilities
        if self.capabilities.prompts.is_none() {
            self.capabilities.prompts = Some(PromptsCapabilities { list_changed: None });
        }

        self
    }

    /// Enable logging capabilities
    pub fn with_logging(mut self) -> Self {
        self.capabilities.logging = Some(LoggingCapabilities {});
        self
    }

    /// Build the server handler
    pub fn build(self) -> SimpleMcpServer {
        SimpleMcpServer {
            name: self.name,
            version: self.version,
            instructions: self.instructions,
            tools: Arc::new(Mutex::new(self.tools)),
            resources: Arc::new(Mutex::new(self.resources)),
            prompts: Arc::new(Mutex::new(self.prompts)),
            capabilities: self.capabilities,
            change_listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Change listener trait
pub trait ChangeListener: Send + Sync {
    /// Called when tools change
    fn on_tools_changed(&self) {}
    /// Called when resources change
    fn on_resources_changed(&self) {}
    /// Called when prompts change
    fn on_prompts_changed(&self) {}
}

/// Simple MCP server implementation
pub struct SimpleMcpServer {
    name: String,
    version: String,
    instructions: Option<String>,
    tools: Arc<Mutex<HashMap<String, RegisteredTool>>>,
    resources: Arc<Mutex<HashMap<String, RegisteredResource>>>,
    prompts: Arc<Mutex<HashMap<String, RegisteredPrompt>>>,
    capabilities: ServerCapabilities,
    change_listeners: Arc<Mutex<Vec<Box<dyn ChangeListener>>>>,
}

impl SimpleMcpServer {
    /// Add a change listener
    pub fn add_change_listener(&self, listener: Box<dyn ChangeListener>) {
        self.change_listeners.lock().unwrap().push(listener);
    }

    /// Notify tools changed
    fn notify_tools_changed(&self) {
        for listener in self.change_listeners.lock().unwrap().iter() {
            listener.on_tools_changed();
        }
    }

    /// Notify resources changed
    fn notify_resources_changed(&self) {
        for listener in self.change_listeners.lock().unwrap().iter() {
            listener.on_resources_changed();
        }
    }

    /// Notify prompts changed
    fn notify_prompts_changed(&self) {
        for listener in self.change_listeners.lock().unwrap().iter() {
            listener.on_prompts_changed();
        }
    }

    /// Dynamically add a tool
    pub fn add_tool<F>(&self, tool: Tool, handler: F)
    where
        F: Fn(Value) -> Result<CallToolResponse> + Send + Sync + 'static,
    {
        let name = tool.name.clone();
        self.tools.lock().unwrap().insert(
            name,
            RegisteredTool {
                tool,
                handler: Arc::new(handler),
            },
        );
        self.notify_tools_changed();
    }

    /// Dynamically remove a tool
    pub fn remove_tool(&self, name: &str) {
        self.tools.lock().unwrap().remove(name);
        self.notify_tools_changed();
    }

    /// Dynamically add a resource
    pub fn add_resource<F>(&self, resource: Resource, read_handler: F)
    where
        F: Fn() -> Result<ResourceContents> + Send + Sync + 'static,
    {
        let uri = resource.uri.clone();
        self.resources.lock().unwrap().insert(
            uri,
            RegisteredResource {
                resource,
                read_handler: Some(Arc::new(read_handler)),
            },
        );
        self.notify_resources_changed();
    }

    /// Dynamically remove a resource
    pub fn remove_resource(&self, uri: &str) {
        self.resources.lock().unwrap().remove(uri);
        self.notify_resources_changed();
    }

    /// Dynamically add a prompt
    pub fn add_prompt<F>(&self, prompt: Prompt, get_handler: F)
    where
        F: Fn(Value) -> Result<GetPromptResponse> + Send + Sync + 'static,
    {
        let name = prompt.name.clone();
        self.prompts.lock().unwrap().insert(
            name,
            RegisteredPrompt {
                prompt,
                get_handler: Some(Arc::new(get_handler)),
            },
        );
        self.notify_prompts_changed();
    }

    /// Dynamically remove a prompt
    pub fn remove_prompt(&self, name: &str) {
        self.prompts.lock().unwrap().remove(name);
        self.notify_prompts_changed();
    }

    /// Get all registered tools
    pub fn tools(&self) -> Vec<Tool> {
        self.tools
            .lock()
            .unwrap()
            .values()
            .map(|t| t.tool.clone())
            .collect()
    }

    /// Get all registered resources
    pub fn resources(&self) -> Vec<Resource> {
        self.resources
            .lock()
            .unwrap()
            .values()
            .map(|r| r.resource.clone())
            .collect()
    }

    /// Get all registered prompts
    pub fn prompts(&self) -> Vec<Prompt> {
        self.prompts
            .lock()
            .unwrap()
            .values()
            .map(|p| p.prompt.clone())
            .collect()
    }
}

#[async_trait]
impl McpServerHandler for SimpleMcpServer {
    fn server_info(&self) -> Implementation {
        Implementation {
            name: self.name.clone(),
            version: self.version.clone(),
        }
    }

    fn capabilities(&self) -> ServerCapabilities {
        self.capabilities.clone()
    }

    fn instructions(&self) -> Option<String> {
        self.instructions.clone()
    }

    async fn on_list_tools(&self, _request: ListToolsRequest) -> Result<ListToolsResponse> {
        Ok(ListToolsResponse {
            tools: self.tools(),
            next_cursor: None,
        })
    }

    async fn on_call_tool(&self, request: CallToolRequest) -> Result<CallToolResponse> {
        let tools = self.tools.lock().unwrap();
        let registered = tools
            .get(&request.name)
            .ok_or_else(|| ProtocolError::Protocol(format!("Tool not found: {}", request.name)))?;

        (registered.handler)(request.arguments)
    }

    async fn on_list_resources(&self, _request: ListResourcesRequest) -> Result<ListResourcesResponse> {
        Ok(ListResourcesResponse {
            resources: self.resources(),
            next_cursor: None,
        })
    }

    async fn on_read_resource(&self, request: ReadResourceRequest) -> Result<ReadResourceResponse> {
        let resources = self.resources.lock().unwrap();
        let registered = resources
            .get(&request.uri)
            .ok_or_else(|| ProtocolError::Protocol(format!("Resource not found: {}", request.uri)))?;

        let contents = if let Some(handler) = &registered.read_handler {
            vec![handler()?]
        } else {
            vec![]
        };

        Ok(ReadResourceResponse { contents })
    }

    async fn on_list_prompts(&self, _request: ListPromptsRequest) -> Result<ListPromptsResponse> {
        Ok(ListPromptsResponse {
            prompts: self.prompts(),
            next_cursor: None,
        })
    }

    async fn on_get_prompt(&self, request: GetPromptRequest) -> Result<GetPromptResponse> {
        let prompts = self.prompts.lock().unwrap();
        let registered = prompts
            .get(&request.name)
            .ok_or_else(|| ProtocolError::Protocol(format!("Prompt not found: {}", request.name)))?;

        if let Some(handler) = &registered.get_handler {
            handler(request.arguments.unwrap_or(Value::Null))
        } else {
            Err(ProtocolError::Protocol(format!(
                "Prompt has no handler: {}",
                request.name
            )))
        }
    }
}

/// Create a text content item
pub fn text_content(text: String) -> Content {
    Content::Text { text }
}

/// Create a successful call tool response with text
pub fn tool_response_text(text: String) -> CallToolResponse {
    CallToolResponse {
        content: vec![text_content(text)],
        is_error: None,
    }
}

/// Create an error call tool response
pub fn tool_response_error(error: String) -> CallToolResponse {
    CallToolResponse {
        content: vec![text_content(error)],
        is_error: Some(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_builder_creation() {
        let builder = McpServerBuilder::new("test-server".to_string(), "1.0.0".to_string());
        assert_eq!(builder.name, "test-server");
        assert_eq!(builder.version, "1.0.0");
    }

    #[test]
    fn test_builder_with_instructions() {
        let builder = McpServerBuilder::new("test".to_string(), "1.0".to_string())
            .with_instructions("Use these tools".to_string());
        assert_eq!(builder.instructions, Some("Use these tools".to_string()));
    }

    #[tokio::test]
    async fn test_register_tool() {
        let builder = McpServerBuilder::new("test".to_string(), "1.0".to_string());

        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({}),
        };

        let builder = builder.register_tool(tool, |_args| {
            Ok(tool_response_text("Hello!".to_string()))
        });

        assert!(builder.capabilities.tools.is_some());

        let server = builder.build();
        assert_eq!(server.tools().len(), 1);
    }

    #[tokio::test]
    async fn test_call_tool() {
        let builder = McpServerBuilder::new("test".to_string(), "1.0".to_string());

        let tool = Tool {
            name: "greet".to_string(),
            description: "Greet someone".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }),
        };

        let builder = builder.register_tool(tool, |args| {
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            Ok(tool_response_text(format!("Hello, {}!", name)))
        });

        let server = builder.build();

        let request = CallToolRequest {
            name: "greet".to_string(),
            arguments: json!({ "name": "Test" }),
        };

        let response = server.on_call_tool(request).await.unwrap();
        assert!(!response.content.is_empty());
    }
}
