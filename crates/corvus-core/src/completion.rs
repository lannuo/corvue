//! Completion model trait and types

use crate::error::Result;
use crate::tool::ToolDefinition;
use crate::types::{Message, Role};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// A streaming delta of a completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionDelta {
    /// The ID of the completion
    pub id: String,
    /// The delta content
    pub delta: MessageDelta,
}

/// A delta of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    /// The role (if provided in this delta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// The content delta (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool call deltas (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// A delta of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// The index of the tool call
    pub index: u32,
    /// The tool call ID (if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The tool name (if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments delta (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
    /// Number of cached input tokens (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u32>,
}

/// A choice in the completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// The index of this choice
    pub index: u32,
    /// The message
    pub message: Message,
    /// Why the completion finished
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// A request to a completion model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The model to use
    pub model: String,
    /// The conversation messages
    pub messages: Vec<Message>,
    /// Temperature for sampling (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
}

impl CompletionRequest {
    /// Create a new completion request
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
            tools: None,
            stream: false,
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 2.0));
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Set top-p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Add tools
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }
}

/// A response from a completion model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// The ID of the completion
    pub id: String,
    /// The model used
    pub model: String,
    /// The choices
    pub choices: Vec<Choice>,
    /// Token usage
    pub usage: Usage,
    /// Raw provider-specific response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

impl CompletionResponse {
    /// Get the first message content, if available
    pub fn content(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }

    /// Get the first tool calls, if available
    pub fn tool_calls(&self) -> Option<&[crate::tool::ToolCall]> {
        self.choices
            .first()
            .and_then(|c| c.message.tool_calls.as_deref())
    }

    /// Get the finish reason of the first choice
    pub fn finish_reason(&self) -> Option<&str> {
        self.choices.first().and_then(|c| c.finish_reason.as_deref())
    }
}

/// Type alias for a streaming completion response
pub type StreamingCompletionResponse = Pin<Box<dyn Stream<Item = Result<CompletionDelta>> + Send>>;

/// Trait for completion models
#[async_trait::async_trait]
pub trait CompletionModel: Send + Sync {
    /// Get the model name
    fn model_name(&self) -> &str;

    /// Perform a completion
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Perform a streaming completion
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse>;
}
