//! Transport layer for MCP communication
//!
//! Provides transport implementations for MCP protocol, including stdio transport.

use crate::error::{ProtocolError, Result};
use crate::mcp::protocol::{
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use std::process::Stdio;

/// Transport trait for sending and receiving JSON-RPC messages
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON-RPC request
    async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<()>;

    /// Send a JSON-RPC response
    async fn send_response(&mut self, response: &JsonRpcResponse) -> Result<()>;

    /// Send a JSON-RPC notification
    async fn send_notification(&mut self, notification: &JsonRpcNotification) -> Result<()>;

    /// Receive a message (could be request, response, or notification)
    async fn receive_message(&mut self) -> Result<TransportMessage>;

    /// Close the transport
    async fn close(&mut self) -> Result<()>;
}

/// A message received from the transport
#[derive(Debug, Clone)]
pub enum TransportMessage {
    /// JSON-RPC request
    Request(JsonRpcRequest),
    /// JSON-RPC response
    Response(JsonRpcResponse),
    /// JSON-RPC notification
    Notification(JsonRpcNotification),
}

/// Stdio-based transport for MCP servers
pub struct StdioTransport {
    /// Child process (if any)
    child: Option<Child>,
    /// Standard input writer
    stdin: ChildStdin,
    /// Standard output reader
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    /// Create a new stdio transport by spawning a command
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self> {
        tracing::debug!("Spawning MCP server: {} {:?}", command, args);

        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| ProtocolError::Transport(format!("Failed to spawn process: {}", e)))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ProtocolError::Transport("Failed to take stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ProtocolError::Transport("Failed to take stdout".to_string())
        })?;

        Ok(Self {
            child: Some(child),
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Create a stdio transport from existing stdin/stdout
    pub fn new(stdin: ChildStdin, stdout: ChildStdout, child: Option<Child>) -> Self {
        Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    /// Read a line from stdout
    async fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .await
            .map_err(ProtocolError::Io)?;

        if line.is_empty() {
            return Err(ProtocolError::Transport("EOF reached".to_string()));
        }

        Ok(line.trim_end().to_string())
    }

    /// Write a line to stdin
    async fn write_line(&mut self, line: &str) -> Result<()> {
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(ProtocolError::Io)?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(ProtocolError::Io)?;
        self.stdin
            .flush()
            .await
            .map_err(ProtocolError::Io)?;
        Ok(())
    }

    /// Parse a JSON line into a transport message
    fn parse_json_line(line: &str) -> Result<TransportMessage> {
        let value: Value = serde_json::from_str(line)
            .map_err(ProtocolError::Json)?;

        // Check if it has an id (request or response)
        if let Some(_id) = value.get("id") {
            // Check if it has method (request) or result/error (response)
            if value.get("method").is_some() {
                let request: JsonRpcRequest = serde_json::from_value(value)
                    .map_err(ProtocolError::Json)?;
                Ok(TransportMessage::Request(request))
            } else {
                let response: JsonRpcResponse = serde_json::from_value(value)
                    .map_err(ProtocolError::Json)?;
                Ok(TransportMessage::Response(response))
            }
        } else {
            // No id, it's a notification
            let notification: JsonRpcNotification = serde_json::from_value(value)
                .map_err(ProtocolError::Json)?;
            Ok(TransportMessage::Notification(notification))
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<()> {
        let json = request.to_json()?;
        tracing::debug!("Sending request: {}", json);
        self.write_line(&json).await
    }

    async fn send_response(&mut self, response: &JsonRpcResponse) -> Result<()> {
        let json = response.to_json()?;
        tracing::debug!("Sending response: {}", json);
        self.write_line(&json).await
    }

    async fn send_notification(&mut self, notification: &JsonRpcNotification) -> Result<()> {
        let json = notification.to_json()?;
        tracing::debug!("Sending notification: {}", json);
        self.write_line(&json).await
    }

    async fn receive_message(&mut self) -> Result<TransportMessage> {
        let line = self.read_line().await?;
        tracing::debug!("Received line: {}", line);
        Self::parse_json_line(&line)
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            tracing::debug!("Killing child process");
            let _ = child.kill().await;
        }
        Ok(())
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // We can't await in drop, but we can try to kill the child synchronously
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}
