//! Tool trait and types

use crate::error::{Result, ToolError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tool definition for LLM consumption
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolDefinition {
    /// The name of the tool
    pub name: String,
    /// A description of what the tool does
    pub description: String,
    /// The parameters schema (JSON Schema)
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    /// Create a simple tool definition with no parameters
    pub fn simple(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(
            name,
            description,
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        )
    }
}

/// A tool call from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The unique ID of this tool call
    pub id: String,
    /// The name of the tool to call
    pub name: String,
    /// The arguments (JSON)
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }

    /// Parse arguments into a specific type
    pub fn parse_args<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_value(self.arguments.clone())
            .map_err(|e| ToolError::InvalidArguments(format!("Failed to parse arguments: {}", e)).into())
    }
}

/// The result of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool call ID this result corresponds to
    pub tool_call_id: String,
    /// The result content
    pub content: String,
    /// Whether this is an error result
    #[serde(default)]
    pub is_error: bool,
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            is_error: false,
            metadata: None,
        }
    }

    /// Create an error tool result
    pub fn error(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            is_error: true,
            metadata: None,
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Trait for implementing tools
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the tool definition
    fn definition(&self) -> ToolDefinition;

    /// Call the tool with arguments
    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult>;
}

// Implement Tool for Arc<dyn Tool> so we can share tools
#[async_trait::async_trait]
impl<T: Tool + ?Sized> Tool for std::sync::Arc<T> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn description(&self) -> &str {
        (**self).description()
    }

    fn definition(&self) -> ToolDefinition {
        (**self).definition()
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        (**self).call(arguments).await
    }
}

// Implement Tool for Box<dyn Tool>
#[async_trait::async_trait]
impl<T: Tool + ?Sized> Tool for Box<T> {
    fn name(&self) -> &str {
        (**self).name()
    }

    fn description(&self) -> &str {
        (**self).description()
    }

    fn definition(&self) -> ToolDefinition {
        (**self).definition()
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        (**self).call(arguments).await
    }
}

/// A collection of tools
#[derive(Default)]
pub struct ToolSet {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolSet {
    /// Create a new empty tool set
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Add a tool to the set
    pub fn add(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    /// Add a boxed tool to the set
    pub fn add_boxed(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Get all tool definitions
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Call a tool by name
    pub async fn call(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        let tool = self
            .get(&tool_call.name)
            .ok_or_else(|| ToolError::NotFound(tool_call.name.clone()))?;

        let mut result = tool.call(tool_call.arguments.clone()).await?;
        result.tool_call_id = tool_call.id.clone();
        Ok(result)
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the tool set is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Iterate over tools
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Box<dyn Tool>)> {
        self.tools.iter()
    }
}
