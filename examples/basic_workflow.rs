//! Basic Workflow Example
//!
//! This example demonstrates a basic workflow using Corvus components
//! without requiring actual API keys.

use anyhow::Result;
use corvus_core::agent::Agent;
use corvus_memory::InMemoryMemory;
use corvus_tools::{FileTool, ShellTool};
use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Corvus Basic Workflow Example ===\n");

    // Create a temporary directory for our example
    let work_dir = tempdir()?;
    println!("Working in temporary directory: {:?}\n", work_dir.path());

    // Example 1: Using FileTool directly
    println!("--- Example 1: File Operations ---");
    let file_tool = FileTool::new(work_dir.path());

    // Write a test file
    file_tool.write_file("hello.txt", "Hello, Corvus!")?;
    println!("✓ Wrote 'hello.txt'");

    // Read the file back
    let content = file_tool.read_file("hello.txt")?;
    println!("✓ Read 'hello.txt': {}", content);

    // List directory
    let entries = file_tool.list_dir(".")?;
    println!("✓ Directory contents: {:?}", entries);

    // Example 2: Using ShellTool directly
    println!("\n--- Example 2: Shell Operations ---");
    let shell_tool = ShellTool::new(work_dir.path());

    // Run a simple command
    if cfg!(windows) {
        println!("Skipping shell example on Windows");
    } else {
        let output = shell_tool.execute_command("echo 'Hello from shell!'")?;
        if output.success {
            println!("✓ Shell command output: {}", output.stdout.trim());
        }
    }

    // Example 3: In-memory memory system
    println!("\n--- Example 3: Memory System ---");
    let mut memory = InMemoryMemory::new();

    // Store some memories
    memory.store(
        "project-corvus".to_string(),
        "An AI agent CLI tool written in Rust".to_string(),
        vec!["rust".to_string(), "ai".to_string(), "agent".to_string()],
    )?;
    println!("✓ Stored memory about Corvus");

    memory.store(
        "rust-lang".to_string(),
        "A systems programming language focused on safety and performance".to_string(),
        vec!["rust".to_string(), "programming".to_string()],
    )?;
    println!("✓ Stored memory about Rust");

    // Search by tag
    let rust_memories = memory.search_by_tag("rust", 10)?;
    println!("✓ Found {} memories tagged with 'rust'", rust_memories.len());

    // List all memories
    let all_memories = memory.list(10)?;
    println!("✓ Total memories: {}", all_memories.len());

    println!("\n=== All examples completed successfully! ===");

    Ok(())
}
