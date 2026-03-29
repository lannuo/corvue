//! HTTP request tool for Corvus

use async_trait::async_trait;
use corvus_core::error::Result;
use corvus_core::tool::{Tool, ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// HTTP request arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpArgs {
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,
    /// URL to request
    pub url: String,
    /// Request headers (optional)
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Request body (optional)
    #[serde(default)]
    pub body: Option<String>,
    /// Request timeout in seconds (default: 30)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    30
}

/// HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: std::collections::HashMap<String, String>,
    /// Response body
    pub body: String,
}

/// HTTP request tool
pub struct HttpTool;

impl HttpTool {
    /// Create a new HTTP tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> &str {
        "Make HTTP requests (GET, POST, PUT, DELETE, etc.)"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "description": "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)",
                        "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to request"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers as key-value pairs",
                        "additionalProperties": { "type": "string" }
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST, PUT, PATCH)"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Request timeout in seconds (default: 30)"
                    }
                },
                "required": ["method", "url"]
            }),
        )
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolResult> {
        let args: HttpArgs = serde_json::from_value(arguments)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(args.timeout))
            .build()
            .map_err(|e| corvus_core::error::ToolError::Execution(e.to_string()))?;

        let method = match args.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => {
                return Ok(ToolResult::error(
                    "",
                    format!("Unsupported HTTP method: {}", args.method),
                ));
            }
        };

        let mut request = client.request(method, &args.url);

        // Add headers
        if let Some(headers) = args.headers {
            for (key, value) in headers {
                if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value) {
                        request = request.header(header_name, header_value);
                    }
                }
            }
        }

        // Add body
        if let Some(body) = args.body {
            request = request.body(body);
        }

        // Send request
        let response = request.send().await
            .map_err(|e| corvus_core::error::ToolError::Execution(e.to_string()))?;

        let status = response.status().as_u16();

        // Get response headers
        let mut headers = std::collections::HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(key.to_string(), value_str.to_string());
            }
        }

        // Get response body
        let body = response.text().await
            .map_err(|e| corvus_core::error::ToolError::Execution(e.to_string()))?;

        let http_response = HttpResponse {
            status,
            headers,
            body,
        };

        let result = json!({
            "success": true,
            "response": http_response
        });

        Ok(ToolResult::success("", serde_json::to_string(&result)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_tool_creation() {
        let tool = HttpTool::new();
        assert_eq!(tool.name(), "http_request");
    }
}
