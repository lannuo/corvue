//! Git operations tool for the agent

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::process::Stdio;

/// Git operations tool
pub struct GitTool {
    /// Repository directory
    repo_dir: PathBuf,
}

impl GitTool {
    /// Create a new git tool for the given repository
    pub fn new(repo_dir: impl Into<PathBuf>) -> Self {
        Self {
            repo_dir: repo_dir.into(),
        }
    }

    /// Run a git command
    fn git_command(&self, args: &[&str]) -> Result<GitOutput> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.repo_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        Ok(GitOutput {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Get git status
    pub fn status(&self) -> Result<GitOutput> {
        self.git_command(&["status", "--porcelain"])
    }

    /// Get git diff
    pub fn diff(&self) -> Result<GitOutput> {
        self.git_command(&["diff"])
    }

    /// Add files
    pub fn add(&self, files: &[&str]) -> Result<GitOutput> {
        let mut args = vec!["add"];
        args.extend(files);
        self.git_command(&args)
    }

    /// Commit changes
    pub fn commit(&self, message: &str) -> Result<GitOutput> {
        self.git_command(&["commit", "-m", message])
    }

    /// Push changes
    pub fn push(&self) -> Result<GitOutput> {
        self.git_command(&["push"])
    }

    /// Pull changes
    pub fn pull(&self) -> Result<GitOutput> {
        self.git_command(&["pull"])
    }

    /// Get log
    pub fn log(&self, limit: usize) -> Result<GitOutput> {
        self.git_command(&["log", "--oneline", "-n", &limit.to_string()])
    }

    /// Checkout branch/commit
    pub fn checkout(&self, ref_name: &str) -> Result<GitOutput> {
        self.git_command(&["checkout", ref_name])
    }

    /// Create branch
    pub fn branch(&self, branch_name: &str) -> Result<GitOutput> {
        self.git_command(&["branch", branch_name])
    }

    /// List branches
    pub fn list_branches(&self) -> Result<GitOutput> {
        self.git_command(&["branch", "-a"])
    }
}

/// Output from a git command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOutput {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Arguments for git tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum GitArgs {
    Status,
    Diff,
    Add { files: Vec<String> },
    Commit { message: String },
    Push,
    Pull,
    Log { limit: Option<usize> },
    Checkout { ref_name: String },
    Branch { branch_name: String },
    ListBranches,
    Custom { args: Vec<String> },
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "git_operations"
    }

    fn description(&self) -> &str {
        "Perform git operations like status, add, commit, push, pull, etc."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "The git operation to perform",
                        "enum": ["status", "diff", "add", "commit", "push", "pull", "log", "checkout", "branch", "list_branches", "custom"]
                    },
                    "files": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Files to add (for add operation)"
                    },
                    "message": {
                        "type": "string",
                        "description": "Commit message (for commit operation)"
                    },
                    "ref_name": {
                        "type": "string",
                        "description": "Branch or commit to checkout (for checkout operation)"
                    },
                    "branch_name": {
                        "type": "string",
                        "description": "Branch name to create (for branch operation)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of log entries (for log operation)"
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Custom git arguments (for custom operation)"
                    }
                },
                "required": ["operation"]
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: GitArgs = serde_json::from_value(arguments)?;

        let output = match args {
            GitArgs::Status => self.status()?,
            GitArgs::Diff => self.diff()?,
            GitArgs::Add { files } => {
                let files_str: Vec<_> = files.iter().map(|s| s.as_str()).collect();
                self.add(&files_str)?
            }
            GitArgs::Commit { message } => self.commit(&message)?,
            GitArgs::Push => self.push()?,
            GitArgs::Pull => self.pull()?,
            GitArgs::Log { limit } => self.log(limit.unwrap_or(10))?,
            GitArgs::Checkout { ref_name } => self.checkout(&ref_name)?,
            GitArgs::Branch { branch_name } => self.branch(&branch_name)?,
            GitArgs::ListBranches => self.list_branches()?,
            GitArgs::Custom { args } => {
                let args_str: Vec<_> = args.iter().map(|s| s.as_str()).collect();
                self.git_command(&args_str)?
            }
        };

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
    fn test_git_tool_creation() {
        let dir = tempdir().unwrap();
        let tool = GitTool::new(dir.path());
        assert_eq!(tool.name(), "git_operations");
    }
}
