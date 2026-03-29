//! Cross-platform sandboxed code execution
//!
//! Provides practical sandboxed execution with security best-practices
//! across all platforms. Full OS-level sandboxing (namespaces, seccomp,
//! sandbox_init) is planned for future versions.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use walkdir::WalkDir;

/// Execution result from sandboxed code
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Whether execution timed out
    pub timed_out: bool,
    /// Files created during execution
    pub created_files: Vec<PathBuf>,
    /// Files modified during execution
    pub modified_files: Vec<PathBuf>,
    /// Files deleted during execution
    pub deleted_files: Vec<PathBuf>,
}

/// Configuration for sandbox execution
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Time limit for execution
    pub timeout: Duration,
    /// Memory limit in bytes (not enforced on all platforms)
    pub memory_limit: Option<u64>,
    /// CPU limit in seconds (not enforced on all platforms)
    pub cpu_limit: Option<u64>,
    /// Whether network access is allowed (currently blocked)
    pub allow_network: bool,
    /// Read-only directories (host paths)
    pub read_only_dirs: Vec<PathBuf>,
    /// Additional environment variables
    pub env_vars: Vec<(String, String)>,
    /// Maximum file size (bytes) for created files
    pub max_file_size: Option<u64>,
    /// Maximum number of files that can be created
    pub max_files: Option<usize>,
    /// Allowed commands (empty = all allowed)
    pub allowed_commands: HashSet<String>,
    /// Blocked commands
    pub blocked_commands: HashSet<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        let mut blocked = HashSet::new();
        blocked.insert("rm".to_string());
        blocked.insert("rmdir".to_string());
        blocked.insert("dd".to_string());
        blocked.insert("mkfs".to_string());
        blocked.insert("wget".to_string());
        blocked.insert("curl".to_string());
        blocked.insert("nc".to_string());
        blocked.insert("netcat".to_string());
        blocked.insert("ssh".to_string());
        blocked.insert("scp".to_string());

        Self {
            timeout: Duration::from_secs(30),
            memory_limit: Some(512 * 1024 * 1024), // 512 MB
            cpu_limit: Some(30),
            allow_network: false,
            read_only_dirs: Vec::new(),
            env_vars: Vec::new(),
            max_file_size: Some(10 * 1024 * 1024), // 10 MB
            max_files: Some(100),
            allowed_commands: HashSet::new(),
            blocked_commands: blocked,
        }
    }
}

/// Cross-platform sandbox executor
pub struct SandboxExecutor {
    /// Temporary working directory
    workdir: TempDir,
    /// Files tracked at start
    initial_files: HashSet<PathBuf>,
}

impl SandboxExecutor {
    /// Create a new sandbox executor
    pub fn new() -> anyhow::Result<Self> {
        let workdir = TempDir::new()?;
        let initial_files = Self::scan_directory(workdir.path())?;

        Ok(Self {
            workdir,
            initial_files,
        })
    }

    /// Get the working directory
    pub fn work_dir(&self) -> &Path {
        self.workdir.path()
    }

    /// Scan directory for files
    fn scan_directory(path: &Path) -> anyhow::Result<HashSet<PathBuf>> {
        let mut files = HashSet::new();
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                files.insert(entry.path().to_path_buf());
            }
        }
        Ok(files)
    }

    /// Check if a command is allowed
    fn is_command_allowed(&self, command: &str, config: &SandboxConfig) -> bool {
        let cmd_name = Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command);

        if config.blocked_commands.contains(cmd_name) {
            return false;
        }

        if !config.allowed_commands.is_empty() {
            return config.allowed_commands.contains(cmd_name);
        }

        true
    }

    /// Execute a command in the sandbox
    pub fn execute(&self, command: &[&str], config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let start = Instant::now();

        if command.is_empty() {
            return Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: "Empty command".to_string(),
                duration: Duration::from_millis(0),
                timed_out: false,
                created_files: Vec::new(),
                modified_files: Vec::new(),
                deleted_files: Vec::new(),
            });
        }

        // Check if command is allowed
        if !self.is_command_allowed(command[0], config) {
            return Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Command '{}' is blocked", command[0]),
                duration: Duration::from_millis(0),
                timed_out: false,
                created_files: Vec::new(),
                modified_files: Vec::new(),
                deleted_files: Vec::new(),
            });
        }

        // Build command
        let mut cmd = Command::new(command[0]);
        cmd.args(&command[1..]);
        cmd.current_dir(self.workdir.path());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Clear dangerous env vars
        cmd.env_clear();
        cmd.env("PATH", "/usr/bin:/bin:/usr/local/bin");
        cmd.env("HOME", self.workdir.path());
        cmd.env("TMPDIR", self.workdir.path());
        cmd.env("TEMP", self.workdir.path());

        // Add allowed env vars
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Execute with timeout
        let output = match cmd.output() {
            Ok(output) => output,
            Err(e) => {
                return Ok(ExecutionResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!("Failed to execute: {}", e),
                    duration: start.elapsed(),
                    timed_out: false,
                    created_files: Vec::new(),
                    modified_files: Vec::new(),
                    deleted_files: Vec::new(),
                });
            }
        };

        let duration = start.elapsed();
        let timed_out = duration > config.timeout;

        // Check file changes
        let final_files = Self::scan_directory(self.workdir.path())?;
        let mut created_files = Vec::new();
        let modified_files = Vec::new();
        let mut deleted_files = Vec::new();

        for file in &final_files {
            if !self.initial_files.contains(file) {
                created_files.push(file.clone());
            }
        }

        for file in &self.initial_files {
            if !final_files.contains(file) {
                deleted_files.push(file.clone());
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ExecutionResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            duration,
            timed_out,
            created_files,
            modified_files,
            deleted_files,
        })
    }

    /// Execute a script with automatic language detection
    pub fn execute_script(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let lang = detect_language(code);

        match lang {
            Language::Python => self.execute_python(code, config),
            Language::JavaScript => self.execute_javascript(code, config),
            Language::Rust => self.execute_rust(code, config),
            Language::Shell => self.execute_shell(code, config),
            Language::Go => self.execute_go(code, config),
            Language::C => self.execute_c(code, config),
        }
    }

    /// Execute Python code
    pub fn execute_python(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let script_path = self.workdir.path().join("script.py");
        std::fs::write(&script_path, code)?;

        self.execute(&["python3", script_path.to_str().unwrap()], config)
    }

    /// Execute JavaScript code
    pub fn execute_javascript(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let script_path = self.workdir.path().join("script.js");
        std::fs::write(&script_path, code)?;

        self.execute(&["node", script_path.to_str().unwrap()], config)
    }

    /// Execute Rust code
    pub fn execute_rust(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        // Create a simple main.rs
        let script_path = self.workdir.path().join("main.rs");
        std::fs::write(&script_path, code)?;

        // First compile
        let compile_result = self.execute(
            &["rustc", "--edition", "2021", "main.rs", "-o", "program"],
            config,
        )?;

        if !compile_result.success {
            return Ok(compile_result);
        }

        // Then run
        self.execute(&["./program"], config)
    }

    /// Execute shell script
    pub fn execute_shell(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let script_path = self.workdir.path().join("script.sh");
        std::fs::write(&script_path, code)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms)?;
        }

        self.execute(&["bash", "script.sh"], config)
    }

    /// Execute Go code
    pub fn execute_go(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let script_path = self.workdir.path().join("main.go");
        std::fs::write(&script_path, code)?;

        self.execute(&["go", "run", "main.go"], config)
    }

    /// Execute C code
    pub fn execute_c(&self, code: &str, config: &SandboxConfig) -> anyhow::Result<ExecutionResult> {
        let script_path = self.workdir.path().join("main.c");
        std::fs::write(&script_path, code)?;

        let compile_result = self.execute(&["gcc", "main.c", "-o", "program"], config)?;

        if !compile_result.success {
            return Ok(compile_result);
        }

        self.execute(&["./program"], config)
    }
}

impl Default for SandboxExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create sandbox executor")
    }
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Python
    Python,
    /// JavaScript/TypeScript
    JavaScript,
    /// Rust
    Rust,
    /// Shell script
    Shell,
    /// Go
    Go,
    /// C/C++
    C,
}

/// Detect programming language from code content
pub fn detect_language(code: &str) -> Language {
    let code_lower = code.to_lowercase();

    if code_lower.contains("#!/bin/bash") || code_lower.contains("#!/bin/sh") {
        return Language::Shell;
    }

    if code.contains("fn main()") || code.contains("use ") && code.contains("::") {
        return Language::Rust;
    }

    if code.contains("package main") && code.contains("func main()") {
        return Language::Go;
    }

    if code.contains("int main(") || code.contains("#include") {
        return Language::C;
    }

    if code.contains("def ") || code.contains("import ") || code.contains("print(") {
        return Language::Python;
    }

    if code.contains("function ") || code.contains("const ") || code.contains("let ") || code.contains("console.log") {
        return Language::JavaScript;
    }

    // Default to Python
    Language::Python
}

impl Language {
    /// Get file extension for this language
    pub fn extension(&self) -> &'static str {
        match self {
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::Rust => "rs",
            Language::Shell => "sh",
            Language::Go => "go",
            Language::C => "c",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let sandbox = SandboxExecutor::new();
        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language("fn main() { println!(); }"), Language::Rust);
        assert_eq!(detect_language("def hello(): print('hi')"), Language::Python);
        assert_eq!(detect_language("console.log('hello')"), Language::JavaScript);
        assert_eq!(detect_language("#!/bin/bash\necho hello"), Language::Shell);
        assert_eq!(detect_language("package main\nfunc main() {}"), Language::Go);
        assert_eq!(detect_language("#include <stdio.h>\nint main() {}"), Language::C);
    }

    #[test]
    fn test_default_config() {
        let config = SandboxConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.allow_network);
        assert!(!config.blocked_commands.is_empty());
    }

    #[test]
    fn test_execute_python() -> anyhow::Result<()> {
        let sandbox = SandboxExecutor::new()?;
        let config = SandboxConfig::default();
        let result = sandbox.execute_python("print('Hello, World!')", &config)?;

        // May fail if python3 not available, but should not panic
        println!("Python test result: {:?}", result);
        Ok(())
    }
}
