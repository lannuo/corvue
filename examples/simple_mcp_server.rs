//! Simple MCP Server Example
//!
//! This example shows how to create a simple MCP server using the
//! corvus-protocol crate.

use anyhow::Result;
use corvus_protocol::mcp::server::{McpServer, McpServerHandler};
use corvus_protocol::mcp::protocol::*;
use serde_json::json;

/// A simple calculator MCP server
struct CalculatorServer;

#[async_trait::async_trait]
impl McpServerHandler for CalculatorServer {
    fn server_info(&self) -> Implementation {
        Implementation {
            name: "calculator-server".to_string(),
            version: "0.1.0".to_string(),
        }
    }

    fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            tools: Some(ToolsCapabilities {
                list_changed: Some(false),
            }),
            ..Default::default()
        }
    }

    fn instructions(&self) -> Option<String> {
        Some("A simple calculator server that can add, subtract, multiply, and divide numbers.".to_string())
    }

    async fn on_list_tools(&self, _request: ListToolsRequest) -> Result<ListToolsResponse> {
        Ok(ListToolsResponse {
            tools: vec![
                Tool {
                    name: "add".to_string(),
                    description: "Add two numbers together".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "a": { "type": "number", "description": "First number" },
                            "b": { "type": "number", "description": "Second number" }
                        },
                        "required": ["a", "b"]
                    }),
                },
                Tool {
                    name: "subtract".to_string(),
                    description: "Subtract two numbers".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "a": { "type": "number", "description": "First number" },
                            "b": { "type": "number", "description": "Second number" }
                        },
                        "required": ["a", "b"]
                    }),
                },
                Tool {
                    name: "multiply".to_string(),
                    description: "Multiply two numbers".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "a": { "type": "number", "description": "First number" },
                            "b": { "type": "number", "description": "Second number" }
                        },
                        "required": ["a", "b"]
                    }),
                },
                Tool {
                    name: "divide".to_string(),
                    description: "Divide two numbers".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "a": { "type": "number", "description": "First number (dividend)" },
                            "b": { "type": "number", "description": "Second number (divisor)" }
                        },
                        "required": ["a", "b"]
                    }),
                },
            ],
            next_cursor: None,
        })
    }

    async fn on_call_tool(&self, request: CallToolRequest) -> Result<CallToolResponse> {
        let result = match request.name.as_str() {
            "add" => {
                let a: f64 = serde_json::from_value(request.arguments["a"].clone())?;
                let b: f64 = serde_json::from_value(request.arguments["b"].clone())?;
                format!("{} + {} = {}", a, b, a + b)
            }
            "subtract" => {
                let a: f64 = serde_json::from_value(request.arguments["a"].clone())?;
                let b: f64 = serde_json::from_value(request.arguments["b"].clone())?;
                format!("{} - {} = {}", a, b, a - b)
            }
            "multiply" => {
                let a: f64 = serde_json::from_value(request.arguments["a"].clone())?;
                let b: f64 = serde_json::from_value(request.arguments["b"].clone())?;
                format!("{} * {} = {}", a, b, a * b)
            }
            "divide" => {
                let a: f64 = serde_json::from_value(request.arguments["a"].clone())?;
                let b: f64 = serde_json::from_value(request.arguments["b"].clone())?;
                if b == 0.0 {
                    return Ok(CallToolResponse {
                        content: vec![Content::Text {
                            text: "Error: Division by zero".to_string(),
                        }],
                        is_error: Some(true),
                    });
                }
                format!("{} / {} = {}", a, b, a / b)
            }
            _ => {
                return Ok(CallToolResponse {
                    content: vec![Content::Text {
                        text: format!("Unknown tool: {}", request.name),
                    }],
                    is_error: Some(true),
                });
            }
        };

        Ok(CallToolResponse {
            content: vec![Content::Text { text: result }],
            is_error: Some(false),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Calculator MCP Server starting...");
    println!("Note: This is a simple example and doesn't implement stdio transport yet.");
    println!("Run with 'cargo run --example simple_mcp_server'");

    // In a real implementation, you would set up stdio transport here
    // and handle incoming JSON-RPC messages.

    Ok(())
}
