//! Simple code execution (for development)

use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Simple execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether successful
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Stdout
    pub stdout: String,
    /// Stderr
    pub stderr: String,
    /// Duration
    pub duration: Duration,
}

/// Simple code executor (no sandboxing - for development only)
pub struct SimpleExecutor {
    workdir: TempDir,
}

impl SimpleExecutor {
    /// Create a new simple executor
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            workdir: TempDir::new()?,
        })
    }

    /// Get work directory
    pub fn workdir(&self) -> &std::path::Path {
        self.workdir.path()
    }

    /// Execute a Python script
    pub fn execute_python(&self, code: &str) -> std::io::Result<ExecutionResult> {
        let script_path = self.workdir.path().join("script.py");
        std::fs::write(&script_path, code)?;

        self.execute_command(&["python3", "script.py"])
    }

    /// Execute a shell command
    pub fn execute_command(&self, command: &[&str]) -> std::io::Result<ExecutionResult> {
        if command.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Empty command",
            ));
        }

        let start = Instant::now();
        let output = Command::new(command[0])
            .args(&command[1..])
            .current_dir(self.workdir.path())
            .output()?;

        let duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok(ExecutionResult {
            success: output.status.success(),
            exit_code,
            stdout,
            stderr,
            duration,
        })
    }
}

impl Default for SimpleExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create executor")
    }
}
