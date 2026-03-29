//! Shell execution tool for the agent

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

/// Shell execution tool
pub struct ShellTool {
    /// Working directory
    workdir: PathBuf,
    /// Timeout for commands
    timeout: Duration,
}

impl ShellTool {
    /// Create a new shell tool
    pub fn new(workdir: impl Into<PathBuf>) -> Self {
        Self {
            workdir: workdir.into(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Execute a shell command
    fn execute_command(&self, command: &str) -> Result<ShellOutput> {
        let shell = if cfg!(windows) { "cmd.exe" } else { "bash" };
        let arg = if cfg!(windows) { "/c" } else { "-c" };

        let output = std::process::Command::new(shell)
            .arg(arg)
            .arg(command)
            .current_dir(&self.workdir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        Ok(ShellOutput {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Output from a shell command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOutput {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Arguments for shell tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellArgs {
    pub command: String,
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell_exec"
    }

    fn description(&self) -> &str {
        "Execute shell commands in the working directory."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: ShellArgs = serde_json::from_value(arguments)?;
        let output = self.execute_command(&args.command)?;

        let result = json!({
            "success": output.success,
            "exit_code": output.exit_code,
            "stdout": output.stdout,
            "stderr": output.stderr
        });

        Ok(ToolResult::success("", serde_json::to_string(&result)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_shell_tool_creation() {
        let dir = tempdir().unwrap();
        let tool = ShellTool::new(dir.path());
        assert_eq!(tool.name(), "shell_exec");
    }

    #[test]
    fn test_shell_execution() -> Result<()> {
        let dir = tempdir().unwrap();
        let tool = ShellTool::new(dir.path());

        let output = if cfg!(windows) {
            tool.execute_command("echo hello")?
        } else {
            tool.execute_command("echo 'hello'")?
        };

        assert!(output.success);
        assert!(output.stdout.to_lowercase().contains("hello"));

        Ok(())
    }
}
