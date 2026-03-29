//! Corvus Execution - Sandboxed code execution
//!
//! Cross-platform sandboxed code execution for Corvus AI Agent.

#![warn(missing_docs)]

pub mod simple;
pub mod sandbox;

pub use simple::SimpleExecutor;
pub use sandbox::{detect_language, ExecutionResult, Language, SandboxConfig, SandboxExecutor};
