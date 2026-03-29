//! Agent and core integration tests

#[cfg(test)]
mod agent_tests {
    use crate::agent::{AgentBuilder, AgentConfig};
    use crate::completion::*;
    use crate::tool::*;
    use crate::types::Message;
    use async_trait::async_trait;

    // Mock completion model for testing
    struct MockCompletionModel {
        model_name: String,
        responses: Vec<String>,
        current_response: std::sync::Mutex<usize>,
    }

    impl MockCompletionModel {
        fn new(model_name: &str, responses: Vec<String>) -> Self {
            Self {
                model_name: model_name.to_string(),
                responses,
                current_response: std::sync::Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl CompletionModel for MockCompletionModel {
        fn model_name(&self) -> &str {
            &self.model_name
        }

        async fn complete(&self, _request: CompletionRequest) -> crate::error::Result<CompletionResponse> {
            let mut current = self.current_response.lock().unwrap();
            let response_text = if *current < self.responses.len() {
                let text = self.responses[*current].clone();
                *current += 1;
                text
            } else {
                "Final response".to_string()
            };

            Ok(CompletionResponse {
                id: "test-id".to_string(),
                model: self.model_name.clone(),
                choices: vec![Choice {
                    index: 0,
                    message: Message::assistant(&response_text),
                    finish_reason: Some("stop".to_string()),
                }],
                usage: Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                    cached_input_tokens: None,
                },
                raw: None,
            })
        }

        async fn complete_stream(&self, _request: CompletionRequest) -> crate::error::Result<StreamingCompletionResponse> {
            unimplemented!("Streaming not implemented for mock")
        }
    }

    // Mock tool for testing
    struct MockTool {
        name: String,
        description: String,
    }

    impl MockTool {
        fn new(name: &str, description: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn definition(&self) -> ToolDefinition {
            ToolDefinition::simple(self.name(), self.description())
        }

        async fn call(&self, _arguments: serde_json::Value) -> crate::error::Result<ToolResult> {
            Ok(ToolResult::success("test-call-id", format!("Executed {} successfully", self.name)))
        }
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.stream, false);
        assert_eq!(config.context_window_size, 128000);
    }

    #[test]
    fn test_agent_config_custom() {
        let config = AgentConfig {
            temperature: 0.5,
            max_tokens: Some(1000),
            max_iterations: 10,
            preamble: Some("You are a helpful assistant.".to_string()),
            stream: true,
            context_window_size: 8000,
        };
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.max_iterations, 10);
    }

    #[test]
    fn test_agent_builder_creation() {
        let _builder = AgentBuilder::default();
        // Just verify it can be created
    }

    #[tokio::test]
    async fn test_agent_builder_with_tools() {
        let model = MockCompletionModel::new("test-model", vec!["Hello!".to_string()]);
        let tool = MockTool::new("test-tool", "A test tool");

        let agent = AgentBuilder::default()
            .completion_model(model)
            .tool(tool)
            .build()
            .unwrap();

        assert_eq!(agent.tools().len(), 1);
        assert!(agent.tools().contains("test-tool"));
    }

    #[tokio::test]
    async fn test_agent_simple_run() {
        let responses = vec!["Hello, world!".to_string()];
        let model = MockCompletionModel::new("test-model", responses);

        let agent = AgentBuilder::default()
            .completion_model(model)
            .build()
            .unwrap();

        let result = agent.run("Hi there").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_agent_with_preamble() {
        let responses = vec!["Preamble response".to_string()];
        let model = MockCompletionModel::new("test-model", responses);

        let agent = AgentBuilder::default()
            .completion_model(model)
            .preamble("You are a test assistant.")
            .build()
            .unwrap();

        let result = agent.run("Test").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_definition_new() {
        let def = ToolDefinition::new(
            "test-tool",
            "A test tool",
            serde_json::json!({"type": "object"}),
        );
        assert_eq!(def.name, "test-tool");
        assert_eq!(def.description, "A test tool");
    }

    #[test]
    fn test_tool_definition_simple() {
        let def = ToolDefinition::simple("test-tool", "A test tool");
        assert_eq!(def.name, "test-tool");
        assert_eq!(def.description, "A test tool");
    }

    #[test]
    fn test_tool_call_new() {
        let call = ToolCall::new("call-1", "test-tool", serde_json::json!({"arg": "value"}));
        assert_eq!(call.id, "call-1");
        assert_eq!(call.name, "test-tool");
    }

    #[test]
    fn test_tool_call_parse_args() {
        #[derive(serde::Deserialize)]
        struct TestArgs {
            arg: String,
        }

        let call = ToolCall::new("call-1", "test-tool", serde_json::json!({"arg": "value"}));
        let args: TestArgs = call.parse_args().unwrap();
        assert_eq!(args.arg, "value");
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("call-1", "Success!");
        assert_eq!(result.tool_call_id, "call-1");
        assert_eq!(result.content, "Success!");
        assert_eq!(result.is_error, false);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("call-1", "Error!");
        assert_eq!(result.tool_call_id, "call-1");
        assert_eq!(result.content, "Error!");
        assert_eq!(result.is_error, true);
    }

    #[test]
    fn test_tool_result_with_metadata() {
        let result = ToolResult::success("call-1", "Success!")
            .with_metadata(serde_json::json!({"key": "value"}));
        assert!(result.metadata.is_some());
    }

    #[test]
    fn test_tool_set_new() {
        let tools = ToolSet::new();
        assert!(tools.is_empty());
        assert_eq!(tools.len(), 0);
    }

    #[test]
    fn test_tool_set_add() {
        let mut tools = ToolSet::new();
        let tool = MockTool::new("test-tool", "A test tool");
        tools.add(tool);
        assert_eq!(tools.len(), 1);
        assert!(tools.contains("test-tool"));
    }

    #[test]
    fn test_tool_set_get() {
        let mut tools = ToolSet::new();
        let tool = MockTool::new("test-tool", "A test tool");
        tools.add(tool);
        let found = tools.get("test-tool");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test-tool");
    }

    #[test]
    fn test_tool_set_definitions() {
        let mut tools = ToolSet::new();
        tools.add(MockTool::new("tool-1", "Tool 1"));
        tools.add(MockTool::new("tool-2", "Tool 2"));
        let defs = tools.definitions();
        assert_eq!(defs.len(), 2);
    }

    #[tokio::test]
    async fn test_tool_set_call() {
        let mut tools = ToolSet::new();
        tools.add(MockTool::new("test-tool", "A test tool"));

        let call = ToolCall::new("call-1", "test-tool", serde_json::json!({}));
        let result = tools.call(&call).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.tool_call_id, "call-1");
        assert!(result.content.contains("successfully"));
    }

    #[test]
    fn test_tool_set_iter() {
        let mut tools = ToolSet::new();
        tools.add(MockTool::new("tool-1", "Tool 1"));
        tools.add(MockTool::new("tool-2", "Tool 2"));

        let mut names: Vec<_> = tools.iter().map(|(name, _)| name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["tool-1", "tool-2"]);
    }

    #[tokio::test]
    async fn test_agent_accessors() {
        let model = MockCompletionModel::new("test-model", vec!["Hello".to_string()]);
        let tool = MockTool::new("test-tool", "A test tool");

        let agent = AgentBuilder::default()
            .completion_model(model)
            .tool(tool)
            .temperature(0.5)
            .build()
            .unwrap();

        assert_eq!(agent.completion_model().model_name(), "test-model");
        assert_eq!(agent.tools().len(), 1);
        assert_eq!(agent.config().temperature, 0.5);
        assert!(agent.memory().is_none());
    }
}
