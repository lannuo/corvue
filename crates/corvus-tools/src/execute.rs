//! Code execution tool for the agent

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use corvus_execution::{SandboxConfig, SandboxExecutor};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Code execution tool
pub struct ExecuteTool {
    executor: SandboxExecutor,
    config: SandboxConfig,
}

impl ExecuteTool {
    /// Create a new execute tool
    pub fn new() -> Result<Self> {
        Ok(Self {
            executor: SandboxExecutor::new()?,
            config: SandboxConfig::default(),
        })
    }

    /// Create with custom config
    pub fn with_config(config: SandboxConfig) -> Result<Self> {
        Ok(Self {
            executor: SandboxExecutor::new()?,
            config,
        })
    }
}

/// Arguments for execute tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteArgs {
    /// Code to execute
    pub code: String,
    /// Language (optional, auto-detected if not provided)
    pub language: Option<String>,
}

#[async_trait]
impl Tool for ExecuteTool {
    fn name(&self) -> &str {
        "execute_code"
    }

    fn description(&self) -> &str {
        "Execute code in a sandboxed environment. Supports Python, JavaScript, Rust, Shell, Go, and C."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "The code to execute"
                    },
                    "language": {
                        "type": "string",
                        "description": "The programming language (python, javascript, rust, shell, go, c). Auto-detected if not provided.",
                        "enum": ["python", "javascript", "rust", "shell", "go", "c"]
                    }
                },
                "required": ["code"]
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: ExecuteArgs = serde_json::from_value(arguments)?;

        let result = self.executor.execute_script(&args.code, &self.config);

        match result {
            Ok(exec_result) => {
                let output = format!(
                    "Exit code: {}\n\nStdout:\n{}\n\nStderr:\n{}",
                    exec_result.exit_code, exec_result.stdout, exec_result.stderr
                );

                Ok(ToolResult::success("", output))
            }
            Err(e) => Ok(ToolResult::error(
                "",
                format!("Execution failed: {}", e),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_tool_creation() {
        let tool = ExecuteTool::new();
        assert!(tool.is_ok());
    }

    #[test]
    fn test_tool_metadata() {
        let tool = ExecuteTool::new().unwrap();
        assert_eq!(tool.name(), "execute_code");
        assert!(!tool.description().is_empty());
    }
}
