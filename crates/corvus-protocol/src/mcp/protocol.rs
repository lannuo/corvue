//! MCP protocol message definitions
//!
//! Defines the JSON-RPC based protocol messages for MCP communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ProtocolError, Result};

// ============================================================================
// JSON-RPC Base Types
// ============================================================================

/// JSON-RPC request ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String ID
    String(String),
    /// Numeric ID
    Number(i64),
}

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version, must be "2.0"
    pub jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version, must be "2.0"
    pub jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Response result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Response error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC notification (no ID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version, must be "2.0"
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Notification parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

// ============================================================================
// MCP Protocol Methods
// ============================================================================

// ----------------------------------------------------------------------------
// Initialize
// ----------------------------------------------------------------------------

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// Protocol version
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Client information
    pub client_info: Implementation,
}

/// Initialize response result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    /// Protocol version
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Server information
    pub server_info: Implementation,
    /// Instructions for using this server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Client implementation info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    /// Name of the implementation
    pub name: String,
    /// Version of the implementation
    pub version: String,
}

/// Client capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    /// Sampling capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,
    /// Roots capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapabilities>,
}

/// Sampling capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapabilities {}

/// Roots capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapabilities {
    /// Whether the client supports list changed notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    /// Logging capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,
    /// Prompts capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapabilities>,
    /// Resources capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapabilities>,
    /// Tools capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapabilities>,
}

/// Logging capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapabilities {}

/// Prompts capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapabilities {
    /// Whether the server supports list changed notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapabilities {
    /// Whether the server supports subscribing to resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    /// Whether the server supports list changed notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Tools capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapabilities {
    /// Whether the server supports list changed notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

// ----------------------------------------------------------------------------
// Ping
// ----------------------------------------------------------------------------

/// Ping request (empty params)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {}

/// Ping response (empty result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {}

// ----------------------------------------------------------------------------
// Tools
// ----------------------------------------------------------------------------

/// List tools request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsRequest {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// List tools response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResponse {
    /// List of tools
    pub tools: Vec<Tool>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// The name of the tool
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON Schema for the tool arguments
    pub input_schema: Value,
}

/// Call tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequest {
    /// Name of the tool to call
    pub name: String,
    /// Arguments for the tool
    pub arguments: Value,
}

/// Call tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResponse {
    /// Content returned by the tool
    pub content: Vec<Content>,
    /// Whether the tool call was successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    /// Text content
    Text {
        /// The text content
        text: String,
    },
    /// Image content
    Image {
        /// The image data (base64)
        data: String,
        /// The image MIME type
        mime_type: String,
    },
    /// Embedded resource
    EmbeddedResource {
        /// Resource URI
        uri: String,
        /// Resource text content
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// Resource blob content
        #[serde(skip_serializing_if = "Option::is_none")]
        blob: Option<String>,
    },
}

// ----------------------------------------------------------------------------
// Resources
// ----------------------------------------------------------------------------

/// List resources request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesRequest {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// List resources response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResponse {
    /// List of resources
    pub resources: Vec<Resource>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Read resource request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    /// Resource URI
    pub uri: String,
}

/// Read resource response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResponse {
    /// Resource contents
    pub contents: Vec<ResourceContents>,
}

/// Resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    /// Resource URI
    pub uri: String,
    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Blob content (base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

// ----------------------------------------------------------------------------
// Prompts
// ----------------------------------------------------------------------------

/// List prompts request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsRequest {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// List prompts response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResponse {
    /// List of prompts
    pub prompts: Vec<Prompt>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// Prompt name
    pub name: String,
    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Get prompt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptRequest {
    /// Prompt name
    pub name: String,
    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Get prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResponse {
    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Prompt messages
    pub messages: Vec<PromptMessage>,
}

/// Prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    /// Message role
    pub role: PromptMessageRole,
    /// Message content
    pub content: Content,
}

/// Prompt message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptMessageRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

// ----------------------------------------------------------------------------
// Notifications
// ----------------------------------------------------------------------------

/// Tool list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListChangedNotification {}

/// Resource list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListChangedNotification {}

/// Prompt list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsListChangedNotification {}

/// Roots list changed notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListChangedNotification {}

/// Log message notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessageNotification {
    /// Log level
    pub level: LoggingLevel,
    /// Log message
    pub data: Value,
    /// Logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

/// Logging level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warning,
    /// Error level
    Error,
}

// ============================================================================
// Message Conversion Traits
// ============================================================================

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    pub fn new(id: RequestId, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(ProtocolError::Json)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(ProtocolError::Json)
    }
}

impl JsonRpcResponse {
    /// Create a successful response
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: RequestId, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message, data }),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(ProtocolError::Json)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(ProtocolError::Json)
    }
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC notification
    pub fn new(method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(ProtocolError::Json)
    }

    /// Deserialize from JSON string
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(ProtocolError::Json)
    }
}
