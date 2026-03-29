//! OpenAI completion model implementation

use crate::openai::client::OpenAIClient;
use crate::openai::types::{
    ChatMessage, FunctionCall,
    FunctionDefinition, ToolCall, ToolDefinition,
};
use corvus_core::completion::{
    Choice, CompletionDelta, CompletionModel, CompletionRequest, CompletionResponse,
    MessageDelta as CorvusMessageDelta, StreamingCompletionResponse,
    ToolCallDelta as CorvusToolCallDelta, Usage,
};
use corvus_core::error::{CompletionError, Result};
use corvus_core::tool::ToolDefinition as CorvusToolDefinition;
use corvus_core::types::{Message, Role};
use futures::StreamExt;
use std::sync::Arc;

/// OpenAI completion model
pub struct OpenAICompletionModel {
    client: Arc<OpenAIClient>,
    model: String,
}

impl OpenAICompletionModel {
    /// Create a new OpenAI completion model
    pub fn new(client: Arc<OpenAIClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }

    /// Create a new OpenAI completion model with GPT-4o
    pub fn gpt_4o(client: Arc<OpenAIClient>) -> Self {
        Self::new(client, crate::openai::models::GPT_4O)
    }

    /// Create a new OpenAI completion model with GPT-4o Mini
    pub fn gpt_4o_mini(client: Arc<OpenAIClient>) -> Self {
        Self::new(client, crate::openai::models::GPT_4O_MINI)
    }
}

// Convert Corvus message to OpenAI message
fn to_openai_message(msg: &Message) -> ChatMessage {
    let role = match msg.role {
        Role::System => "system".to_string(),
        Role::User => "user".to_string(),
        Role::Assistant => "assistant".to_string(),
        Role::Tool => "tool".to_string(),
    };

    let tool_calls = msg.tool_calls.as_ref().map(|calls| {
        calls
            .iter()
            .map(|call| ToolCall {
                id: call.id.clone(),
                type_: "function".to_string(),
                function: FunctionCall {
                    name: call.name.clone(),
                    arguments: serde_json::to_string(&call.arguments).unwrap_or_default(),
                },
            })
            .collect()
    });

    ChatMessage {
        role,
        content: Some(msg.content.clone()),
        tool_calls,
        tool_call_id: msg.tool_call_id.clone(),
    }
}

// Convert OpenAI message to Corvus message
fn from_openai_message(msg: &ChatMessage) -> Message {
    let role = match msg.role.as_str() {
        "system" => Role::System,
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        _ => Role::User,
    };

    let tool_calls = msg.tool_calls.as_ref().map(|calls| {
        calls
            .iter()
            .filter_map(|call| {
                let args = serde_json::from_str(&call.function.arguments).ok()?;
                Some(corvus_core::tool::ToolCall::new(
                    call.id.clone(),
                    call.function.name.clone(),
                    args,
                ))
            })
            .collect()
    });

    Message {
        role,
        content: msg.content.clone().unwrap_or_default(),
        tool_calls,
        tool_call_id: msg.tool_call_id.clone(),
    }
}

// Convert Corvus tool definition to OpenAI format
fn to_openai_tool(tool: &CorvusToolDefinition) -> ToolDefinition {
    ToolDefinition {
        type_: "function".to_string(),
        function: FunctionDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.parameters.clone(),
        },
    }
}

#[async_trait::async_trait]
impl CompletionModel for OpenAICompletionModel {
    fn model_name(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let openai_request = crate::openai::types::ChatCompletionRequest {
            model: self.model.clone(),
            messages: request.messages.iter().map(to_openai_message).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            tools: request.tools.map(|tools| tools.iter().map(to_openai_tool).collect()),
            stream: None,
        };

        let response = self.client.chat_completion(openai_request).await
            .map_err(|e| CompletionError::ApiRequest(e.to_string()))?;

        let choices = response
            .choices
            .iter()
            .map(|c| Choice {
                index: c.index,
                message: from_openai_message(&c.message),
                finish_reason: c.finish_reason.clone(),
            })
            .collect();

        Ok(CompletionResponse {
            id: response.id.clone(),
            model: response.model.clone(),
            choices,
            usage: Usage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
                cached_input_tokens: None,
            },
            raw: None,
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse> {
        let openai_request = crate::openai::types::ChatCompletionRequest {
            model: self.model.clone(),
            messages: request.messages.iter().map(to_openai_message).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            tools: request.tools.map(|tools| tools.iter().map(to_openai_tool).collect()),
            stream: Some(true),
        };

        let stream = self.client.chat_completion_stream(openai_request).await
            .map_err(|e| CompletionError::ApiRequest(e.to_string()))?;

        // Convert OpenAI stream to Corvus stream
        let converted_stream = stream.map(|result| match result {
            Ok(response) => {
                let delta = response.choices.first().map(|c| {
                    let content = c.delta.content.clone();
                    let role = c.delta.role.as_ref().and_then(|r| match r.as_str() {
                        "system" => Some(Role::System),
                        "user" => Some(Role::User),
                        "assistant" => Some(Role::Assistant),
                        "tool" => Some(Role::Tool),
                        _ => None,
                    });

                    let tool_calls = c.delta.tool_calls.as_ref().map(|calls| {
                        calls
                            .iter()
                            .map(|call| CorvusToolCallDelta {
                                index: call.index,
                                id: call.id.clone(),
                                name: call.function.as_ref().and_then(|f| f.name.clone()),
                                arguments: call.function.as_ref().and_then(|f| f.arguments.clone()),
                            })
                            .collect()
                    });

                    CorvusMessageDelta {
                        role,
                        content,
                        tool_calls,
                    }
                });

                Ok(CompletionDelta {
                    id: response.id.clone(),
                    delta: delta.unwrap_or(CorvusMessageDelta {
                        role: None,
                        content: None,
                        tool_calls: None,
                    }),
                })
            }
            Err(e) => Err(CompletionError::ApiRequest(e.to_string()).into()),
        });

        Ok(Box::pin(converted_stream))
    }
}
