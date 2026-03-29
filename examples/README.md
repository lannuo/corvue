# Corvus Examples

This directory contains example code demonstrating how to use Corvus.

## Examples

### 1. Basic Workflow (`basic_workflow.rs`)

Demonstrates basic usage of Corvus components without requiring API keys:
- File operations with FileTool
- Shell operations with ShellTool
- Memory system usage

```bash
cargo run --example basic_workflow
```

### 2. Simple MCP Server (`simple_mcp_server.rs`)

Shows how to create a custom MCP (Model Context Protocol) server:
- Calculator server with add/subtract/multiply/divide tools
- Implements the McpServerHandler trait

```bash
cargo run --example simple_mcp_server
```

## Running Examples

All examples can be run from the project root:

```bash
# List all examples
cargo run --example

# Run a specific example
cargo run --example <example_name>
```

## Integration Tests

The `tests/` directory contains end-to-end integration tests that simulate real usage.

```bash
# Run all tests including integration tests
cargo test

# Run only integration tests
cargo test --test '*'
```
