//! Integration Tests for Corvus
//!
//! These tests verify that all components work together correctly.

use anyhow::Result;
use corvus_memory::InMemoryMemory;
use corvus_tools::{FileTool, ShellTool};
use tempfile::tempdir;

#[tokio::test]
async fn test_file_and_memory_workflow() -> Result<()> {
    let work_dir = tempdir()?;

    // Test 1: File operations
    let file_tool = FileTool::new(work_dir.path());

    // Write a file
    file_tool.write_file("test.txt", "Hello, world!")?;
    assert!(file_tool.exists("test.txt"));

    // Read the file back
    let content = file_tool.read_file("test.txt")?;
    assert_eq!(content, "Hello, world!");

    // Get file info
    let info = file_tool.file_info("test.txt")?;
    assert!(info.is_file);
    assert_eq!(info.size, 13); // "Hello, world!" is 13 bytes

    // Test 2: Memory system
    let memory = InMemoryMemory::new();

    // Store a memory about the file
    use corvus_core::memory::{ContentType, MemoryItem};
    use std::time::SystemTime;

    let item = MemoryItem {
        id: None,
        content: "Created a test file with 'Hello, world!'".to_string(),
        content_type: ContentType::Text,
        tags: vec!["file".to_string(), "test".to_string()],
        embedding: None,
        metadata: Default::default(),
        timestamp: SystemTime::now(),
        source: None,
    };

    let id = memory.store(item).await?;

    // Retrieve the memory
    let retrieved = memory.get(&id).await?;
    assert!(retrieved.content.contains("test file"));
    assert!(retrieved.tags.contains(&"file".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_shell_operations() -> Result<()> {
    let work_dir = tempdir()?;
    let shell_tool = ShellTool::new(work_dir.path());

    if cfg!(windows) {
        // Skip on Windows for now
        return Ok(());
    }

    // Test shell command
    let output = shell_tool.execute_command("echo 'test output'")?;
    assert!(output.success);
    assert!(output.stdout.contains("test output"));

    Ok(())
}

#[tokio::test]
async fn test_memory_search() -> Result<()> {
    let memory = InMemoryMemory::new();

    // Store multiple memories
    use corvus_core::memory::{ContentType, MemoryItem, MemoryQuery};
    use std::time::SystemTime;

    let items = vec![
        MemoryItem {
            id: None,
            content: "Rust is a systems programming language".to_string(),
            content_type: ContentType::Text,
            tags: vec!["rust".to_string(), "programming".to_string()],
            embedding: None,
            metadata: Default::default(),
            timestamp: SystemTime::now(),
            source: None,
        },
        MemoryItem {
            id: None,
            content: "Python is great for scripting".to_string(),
            content_type: ContentType::Text,
            tags: vec!["python".to_string(), "scripting".to_string()],
            embedding: None,
            metadata: Default::default(),
            timestamp: SystemTime::now(),
            source: None,
        },
        MemoryItem {
            id: None,
            content: "Both Rust and Python are useful".to_string(),
            content_type: ContentType::Text,
            tags: vec!["rust".to_string(), "python".to_string()],
            embedding: None,
            metadata: Default::default(),
            timestamp: SystemTime::now(),
            source: None,
        },
    ];

    for item in items {
        memory.store(item).await?;
    }

    // Search by tag
    let rust_memories = memory.search_by_tags(vec!["rust".to_string()]).await?;
    assert_eq!(rust_memories.len(), 2);

    // Search by text
    let query = MemoryQuery {
        text: Some("Python".to_string()),
        tags: vec![],
        content_types: vec![],
        embedding: None,
        time_range: None,
        limit: 10,
        offset: 0,
    };

    let results = memory.retrieve(query).await?;
    assert!(!results.is_empty());

    Ok(())
}
