//! Corvus Tools - Built-in tools for the agent
//!
//! Built-in tools for Corvus AI agent including code execution,
//! file operations, and more.

#![warn(missing_docs)]
#![allow(missing_docs)] // Temporarily allow missing docs for release

pub mod execute;
pub mod file;
pub mod shell;
pub mod git;
pub mod search;
pub mod http;
pub mod system;

pub use execute::{ExecuteArgs, ExecuteTool};
pub use file::{FileArgs, FileInfo, FileTool};
pub use shell::{ShellArgs, ShellOutput, ShellTool};
pub use git::{GitArgs, GitOutput, GitTool};
pub use search::{SearchArgs, SearchTool};
pub use http::{HttpArgs, HttpResponse, HttpTool};
pub use system::{SystemArgs, SystemInfo, SystemTool};
