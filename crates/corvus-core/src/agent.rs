//! Agent trait and implementation

use crate::completion::{CompletionModel, CompletionRequest, StreamingCompletionResponse};
use crate::memory::MemorySystem;
use crate::tool::{Tool, ToolCall, ToolResult, ToolSet};
use crate::types::Message;
use crate::error::{CorvusError, Result};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Configuration for the agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Temperature for LLM calls
    pub temperature: f32,
    /// Maximum tokens per completion
    pub max_tokens: Option<u32>,
    /// Maximum number of tool call iterations
    pub max_iterations: u32,
    /// System prompt / preamble
    pub preamble: Option<String>,
    /// Whether to stream responses
    pub stream: bool,
    /// Context window size in tokens
    pub context_window_size: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: None,
            max_iterations: 20,
            preamble: None,
            stream: false,
            context_window_size: 128000,
        }
    }
}

/// The main Corvus agent
pub struct Agent {
    /// Completion model
    completion_model: Arc<dyn CompletionModel>,
    /// Optional memory system
    memory: Option<Arc<dyn MemorySystem>>,
    /// Tools available to the agent
    tools: ToolSet,
    /// Configuration
    config: AgentConfig,
}

impl Agent {
    /// Create a new agent builder
    pub fn builder() -> AgentBuilder {
        AgentBuilder::default()
    }

    /// Run the agent with a user prompt
    pub async fn run(&self, prompt: &str) -> Result<String> {
        let mut messages = Vec::new();

        // Add system preamble if provided
        if let Some(preamble) = &self.config.preamble {
            messages.push(Message::system(preamble));
        }

        // Retrieve relevant memories if available
        if let Some(memory) = &self.memory {
            let memory_query = crate::memory::MemoryQuery::text(prompt)
                .with_limit(5);

            if let Ok(memories) = memory.retrieve(memory_query).await {
                if !memories.is_empty() {
                    let mut context = "Relevant context from memory:\n\n".to_string();
                    for (i, mem) in memories.iter().enumerate() {
                        context.push_str(&format!("{}. {}\n\n", i + 1, mem.item.content));
                    }
                    messages.push(Message::system(context));
                }
            }
        }

        // Add user message
        messages.push(Message::user(prompt));

        // Main agentic loop
        let mut iteration = 0;
        while iteration < self.config.max_iterations {
            debug!("Agent iteration {}", iteration + 1);

            // Trim messages to fit context window
            let trimmed_messages = self.trim_messages(messages.clone());

            // Prepare request with tools
            let mut request = CompletionRequest::new(
                self.completion_model.model_name().to_string(),
                trimmed_messages,
            )
            .with_temperature(self.config.temperature);

            if let Some(max) = self.config.max_tokens {
                request = request.with_max_tokens(max);
            }

            if !self.tools.is_empty() {
                request = request.with_tools(self.tools.definitions());
            }

            // Get completion
            let response = self.completion_model.complete(request).await?;

            let choice = response
                .choices
                .first()
                .ok_or_else(|| CorvusError::InvalidArgument("No choices in response".to_string()))?;

            // Add assistant message to conversation
            messages.push(choice.message.clone());

            // Check for tool calls
            if let Some(tool_calls) = &choice.message.tool_calls {
                if !tool_calls.is_empty() {
                    debug!("Got {} tool calls", tool_calls.len());

                    // Execute all tool calls
                    for tool_call in tool_calls {
                        info!("Executing tool: {}", tool_call.name);

                        let result = self.execute_tool(tool_call).await?;

                        // Add tool result to conversation
                        messages.push(Message::tool(&result.tool_call_id, &result.content));
                    }

                    iteration += 1;
                    continue;
                }
            }

            // No tool calls - we're done
            let content = &choice.message.content;
            if !content.is_empty() {
                // Store the conversation in memory if available
                if let Some(memory) = &self.memory {
                    let _ = memory
                        .store(crate::memory::MemoryItem::new(
                            format!("User: {}\n\nAssistant: {}", prompt, content),
                            crate::memory::ContentType::Conversation,
                        ))
                        .await;
                }

                return Ok(content.clone());
            }
        }

        warn!("Max iterations reached");
        Ok("Max iterations reached without completion.".to_string())
    }

    /// Run the agent with streaming response
    pub async fn run_stream(&self, prompt: &str) -> Result<StreamingCompletionResponse> {
        let mut messages = Vec::new();

        // Add system preamble if provided
        if let Some(preamble) = &self.config.preamble {
            messages.push(Message::system(preamble));
        }

        // Retrieve relevant memories if available
        if let Some(memory) = &self.memory {
            let memory_query = crate::memory::MemoryQuery::text(prompt)
                .with_limit(5);

            if let Ok(memories) = memory.retrieve(memory_query).await {
                if !memories.is_empty() {
                    let mut context = "Relevant context from memory:\n\n".to_string();
                    for (i, mem) in memories.iter().enumerate() {
                        context.push_str(&format!("{}. {}\n\n", i + 1, mem.item.content));
                    }
                    messages.push(Message::system(context));
                }
            }
        }

        // Add user message
        messages.push(Message::user(prompt));

        // Trim messages to fit context window
        let trimmed_messages = self.trim_messages(messages.clone());

        // Prepare request with tools
        let mut request = CompletionRequest::new(
            self.completion_model.model_name().to_string(),
            trimmed_messages,
        )
        .with_temperature(self.config.temperature)
        .with_streaming();

        if let Some(max) = self.config.max_tokens {
            request = request.with_max_tokens(max);
        }

        if !self.tools.is_empty() {
            request = request.with_tools(self.tools.definitions());
        }

        // Get streaming completion
        let stream = self.completion_model.complete_stream(request).await?;

        // For simplicity, this initial streaming implementation just streams the
        // first response without tool call support
        // TODO: Add full tool call support for streaming
        Ok(stream)
    }

    /// Execute a single tool call
    async fn execute_tool(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        match self.tools.call(tool_call).await {
            Ok(result) => Ok(result),
            Err(e) => Ok(ToolResult::error(
                &tool_call.id,
                format!("Tool execution failed: {}", e),
            )),
        }
    }

    /// Get a reference to the completion model
    pub fn completion_model(&self) -> &Arc<dyn CompletionModel> {
        &self.completion_model
    }

    /// Get a reference to the memory system, if available
    pub fn memory(&self) -> Option<&Arc<dyn MemorySystem>> {
        self.memory.as_ref()
    }

    /// Get a reference to the tool set
    pub fn tools(&self) -> &ToolSet {
        &self.tools
    }

    /// Get the agent configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Count tokens in a message (simple estimate: ~4 chars per token)
    fn count_tokens(message: &Message) -> usize {
        let content_len = message.content.len();
        let tool_calls_len = message.tool_calls.as_ref()
            .map(|tc| tc.iter().map(|t| t.name.len() + t.arguments.to_string().len()).sum::<usize>())
            .unwrap_or(0);
        (content_len + tool_calls_len).div_ceil(4) // +3 for rounding
    }

    /// Trim messages to fit within context window
    fn trim_messages(&self, messages: Vec<Message>) -> Vec<Message> {
        if messages.is_empty() {
            return messages;
        }

        // Always keep the first system message(s)
        let system_messages: Vec<Message> = messages
            .iter()
            .take_while(|m| m.role == crate::types::Role::System)
            .cloned()
            .collect();

        let remaining_messages: Vec<Message> = messages
            .into_iter()
            .skip(system_messages.len())
            .collect();

        // Count tokens in system messages
        let system_tokens: usize = system_messages
            .iter()
            .map(Self::count_tokens)
            .sum();

        let available_tokens = self.config.context_window_size.saturating_sub(system_tokens);

        if available_tokens == 0 {
            return system_messages;
        }

        // Keep as many recent messages as possible
        let mut kept_messages = Vec::new();
        let mut total_tokens = 0;

        // Iterate from the end to keep most recent messages
        for msg in remaining_messages.into_iter().rev() {
            let msg_tokens = Self::count_tokens(&msg);
            if total_tokens + msg_tokens <= available_tokens {
                kept_messages.push(msg);
                total_tokens += msg_tokens;
            } else {
                // Skip older messages
                break;
            }
        }

        // Reverse to get back to chronological order
        kept_messages.reverse();

        // Combine system messages with kept messages
        let mut result = system_messages;
        result.extend(kept_messages);
        result
    }
}

/// Builder for constructing an Agent
#[derive(Default)]
pub struct AgentBuilder {
    completion_model: Option<Arc<dyn CompletionModel>>,
    memory: Option<Arc<dyn MemorySystem>>,
    tools: ToolSet,
    config: AgentConfig,
}

impl AgentBuilder {
    /// Set the completion model
    pub fn completion_model(mut self, model: impl CompletionModel + 'static) -> Self {
        self.completion_model = Some(Arc::new(model));
        self
    }

    /// Set the completion model as an Arc
    pub fn completion_model_arc(mut self, model: Arc<dyn CompletionModel>) -> Self {
        self.completion_model = Some(model);
        self
    }

    /// Add a memory system
    pub fn memory(mut self, memory: impl MemorySystem + 'static) -> Self {
        self.memory = Some(Arc::new(memory));
        self
    }

    /// Add a memory system as an Arc
    pub fn memory_arc(mut self, memory: Arc<dyn MemorySystem>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools.add(tool);
        self
    }

    /// Set the temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.config.temperature = temp.clamp(0.0, 2.0);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.config.max_tokens = Some(max);
        self
    }

    /// Set max iterations
    pub fn max_iterations(mut self, max: u32) -> Self {
        self.config.max_iterations = max;
        self
    }

    /// Set the system preamble
    pub fn preamble(mut self, preamble: impl Into<String>) -> Self {
        self.config.preamble = Some(preamble.into());
        self
    }

    /// Enable streaming
    pub fn stream(mut self, enabled: bool) -> Self {
        self.config.stream = enabled;
        self
    }

    /// Set context window size
    pub fn context_window_size(mut self, size: usize) -> Self {
        self.config.context_window_size = size;
        self
    }

    /// Build the agent
    pub fn build(self) -> Result<Agent> {
        Ok(Agent {
            completion_model: self
                .completion_model
                .ok_or_else(|| CorvusError::InvalidArgument("completion model is required".to_string()))?,
            memory: self.memory,
            tools: self.tools,
            config: self.config,
        })
    }
}
