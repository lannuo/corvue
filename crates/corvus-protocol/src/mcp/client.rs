//! MCP client implementation
//!
//! Provides a high-level client for communicating with MCP servers.

use crate::error::{ProtocolError, Result};
use crate::mcp::protocol::*;
use crate::transport::{Transport, TransportMessage};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// MCP client for communicating with MCP servers
#[allow(dead_code)]
pub struct McpClient {
    /// Request ID counter
    request_id: AtomicI64,
    /// Pending requests waiting for responses
    pending_requests: Arc<Mutex<HashMap<RequestId, oneshot::Sender<JsonRpcResponse>>>>,
    /// Transport sender
    transport_sender: mpsc::Sender<ClientMessage>,
    /// Server info from initialize
    server_info: Option<Implementation>,
    /// Server capabilities from initialize
    server_capabilities: Option<ServerCapabilities>,
    /// Server instructions from initialize
    server_instructions: Option<String>,
    /// Available tools
    tools: Vec<Tool>,
}

/// Message sent to the client task
enum ClientMessage {
    /// Send a request
    Request(JsonRpcRequest, oneshot::Sender<JsonRpcResponse>),
    /// Send a notification
    Notification(JsonRpcNotification),
    /// Shutdown the client
    Shutdown,
}

impl McpClient {
    /// Create a new MCP client with the given transport
    pub fn new<T: Transport + 'static>(transport: T) -> Self {
        let (transport_sender, mut transport_receiver) = mpsc::channel(32);
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));

        // Spawn the transport task
        let pending_requests_clone = pending_requests.clone();
        tokio::spawn(async move {
            Self::transport_task(transport, pending_requests_clone, &mut transport_receiver).await;
        });

        Self {
            request_id: AtomicI64::new(1),
            pending_requests,
            transport_sender,
            server_info: None,
            server_capabilities: None,
            server_instructions: None,
            tools: Vec::new(),
        }
    }

    /// Generate the next request ID
    fn next_request_id(&self) -> RequestId {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        RequestId::Number(id)
    }

    /// Transport task that handles sending and receiving messages
    async fn transport_task<T: Transport>(
        mut transport: T,
        pending_requests: Arc<Mutex<HashMap<RequestId, oneshot::Sender<JsonRpcResponse>>>>,
        receiver: &mut mpsc::Receiver<ClientMessage>,
    ) {
        loop {
            tokio::select! {
                // Handle outgoing messages
                msg = receiver.recv() => {
                    match msg {
                        Some(ClientMessage::Request(request, sender)) => {
                            let id = request.id.clone();
                            pending_requests.lock().await.insert(id, sender);
                            if let Err(e) = transport.send_request(&request).await {
                                tracing::error!("Failed to send request: {}", e);
                            }
                        }
                        Some(ClientMessage::Notification(notification)) => {
                            if let Err(e) = transport.send_notification(&notification).await {
                                tracing::error!("Failed to send notification: {}", e);
                            }
                        }
                        Some(ClientMessage::Shutdown) | None => {
                            let _ = transport.close().await;
                            break;
                        }
                    }
                }

                // Handle incoming messages
                msg = transport.receive_message() => {
                    match msg {
                        Ok(TransportMessage::Response(response)) => {
                            let mut pending = pending_requests.lock().await;
                            if let Some(sender) = pending.remove(&response.id) {
                                let _ = sender.send(response);
                            }
                        }
                        Ok(TransportMessage::Request(request)) => {
                            tracing::debug!("Received request from server: {:?}", request);
                        }
                        Ok(TransportMessage::Notification(notification)) => {
                            tracing::debug!("Received notification: {:?}", notification);
                            Self::handle_notification(notification).await;
                        }
                        Err(e) => {
                            tracing::error!("Transport error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Handle incoming notifications
    async fn handle_notification(notification: JsonRpcNotification) {
        match notification.method.as_str() {
            "notifications/tools/list_changed" => {
                tracing::debug!("Tools list changed");
            }
            "notifications/resources/list_changed" => {
                tracing::debug!("Resources list changed");
            }
            "notifications/prompts/list_changed" => {
                tracing::debug!("Prompts list changed");
            }
            "notifications/roots/list_changed" => {
                tracing::debug!("Roots list changed");
            }
            "notifications/message" => {
                tracing::debug!("Log message: {:?}", notification.params);
            }
            _ => {
                tracing::debug!("Unknown notification: {}", notification.method);
            }
        }
    }

    /// Send a request and wait for response
    async fn send_request(&self, method: String, params: Option<serde_json::Value>) -> Result<JsonRpcResponse> {
        let id = self.next_request_id();
        let request = JsonRpcRequest::new(id.clone(), method, params);

        let (sender, receiver) = oneshot::channel();

        self.transport_sender
            .send(ClientMessage::Request(request, sender))
            .await
            .map_err(|_| ProtocolError::Transport("Client closed".to_string()))?;

        let response = receiver
            .await
            .map_err(|_| ProtocolError::Transport("Request cancelled".to_string()))?;

        if let Some(error) = &response.error {
            return Err(ProtocolError::Protocol(format!(
                "Server error: {} (code {})",
                error.message, error.code
            )));
        }

        Ok(response)
    }

    /// Initialize the connection to the MCP server
    pub async fn initialize(
        &mut self,
        client_name: String,
        client_version: String,
    ) -> Result<InitializeResponse> {
        let request = InitializeRequest {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: client_name,
                version: client_version,
            },
        };

        let params = serde_json::to_value(request)?;
        let response = self.send_request("initialize".to_string(), Some(params)).await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in initialize response".to_string())
        })?;

        let init_response: InitializeResponse = serde_json::from_value(result)?;

        self.server_info = Some(init_response.server_info.clone());
        self.server_capabilities = Some(init_response.capabilities.clone());
        self.server_instructions = init_response.instructions.clone();

        // Send initialized notification
        let notification = JsonRpcNotification::new("notifications/initialized".to_string(), None);
        self.transport_sender
            .send(ClientMessage::Notification(notification))
            .await
            .map_err(|_| ProtocolError::Transport("Client closed".to_string()))?;

        Ok(init_response)
    }

    /// Ping the server
    pub async fn ping(&self) -> Result<()> {
        let _ = self.send_request("ping".to_string(), None).await?;
        Ok(())
    }

    /// List available tools from the server
    pub async fn list_tools(&mut self) -> Result<&[Tool]> {
        let request = ListToolsRequest { cursor: None };
        let params = serde_json::to_value(request)?;
        let response = self.send_request("tools/list".to_string(), Some(params)).await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in tools/list response".to_string())
        })?;

        let list_response: ListToolsResponse = serde_json::from_value(result)?;
        self.tools = list_response.tools;

        Ok(&self.tools)
    }

    /// Call a tool
    pub async fn call_tool(&self, name: String, arguments: serde_json::Value) -> Result<CallToolResponse> {
        let request = CallToolRequest { name, arguments };
        let params = serde_json::to_value(request)?;
        let response = self.send_request("tools/call".to_string(), Some(params)).await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in tools/call response".to_string())
        })?;

        let call_response: CallToolResponse = serde_json::from_value(result)?;
        Ok(call_response)
    }

    /// List resources
    pub async fn list_resources(&self) -> Result<ListResourcesResponse> {
        let request = ListResourcesRequest { cursor: None };
        let params = serde_json::to_value(request)?;
        let response = self
            .send_request("resources/list".to_string(), Some(params))
            .await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in resources/list response".to_string())
        })?;

        let list_response: ListResourcesResponse = serde_json::from_value(result)?;
        Ok(list_response)
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: String) -> Result<ReadResourceResponse> {
        let request = ReadResourceRequest { uri };
        let params = serde_json::to_value(request)?;
        let response = self
            .send_request("resources/read".to_string(), Some(params))
            .await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in resources/read response".to_string())
        })?;

        let read_response: ReadResourceResponse = serde_json::from_value(result)?;
        Ok(read_response)
    }

    /// List prompts
    pub async fn list_prompts(&self) -> Result<ListPromptsResponse> {
        let request = ListPromptsRequest { cursor: None };
        let params = serde_json::to_value(request)?;
        let response = self
            .send_request("prompts/list".to_string(), Some(params))
            .await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in prompts/list response".to_string())
        })?;

        let list_response: ListPromptsResponse = serde_json::from_value(result)?;
        Ok(list_response)
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<GetPromptResponse> {
        let request = GetPromptRequest { name, arguments };
        let params = serde_json::to_value(request)?;
        let response = self
            .send_request("prompts/get".to_string(), Some(params))
            .await?;

        let result = response.result.ok_or_else(|| {
            ProtocolError::Protocol("No result in prompts/get response".to_string())
        })?;

        let get_response: GetPromptResponse = serde_json::from_value(result)?;
        Ok(get_response)
    }

    /// Get the server info (after initialization)
    pub fn server_info(&self) -> Option<&Implementation> {
        self.server_info.as_ref()
    }

    /// Get the server capabilities (after initialization)
    pub fn server_capabilities(&self) -> Option<&ServerCapabilities> {
        self.server_capabilities.as_ref()
    }

    /// Get the server instructions (after initialization)
    pub fn server_instructions(&self) -> Option<&str> {
        self.server_instructions.as_deref()
    }

    /// Get the cached tools (after list_tools)
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// Shutdown the client
    pub async fn shutdown(self) -> Result<()> {
        let _ = self.transport_sender.send(ClientMessage::Shutdown).await;
        Ok(())
    }
}
