//! Tests for MCP protocol types

#[cfg(test)]
mod tests {
    use super::super::protocol::*;
    use serde_json::json;

    #[test]
    fn test_request_id_serialization() {
        // Test string ID
        let id = RequestId::String("test-123".to_string());
        let serialized = serde_json::to_string(&id).unwrap();
        assert_eq!(serialized, "\"test-123\"");

        // Test numeric ID
        let id = RequestId::Number(42);
        let serialized = serde_json::to_string(&id).unwrap();
        assert_eq!(serialized, "42");
    }

    #[test]
    fn test_json_rpc_request_creation() {
        let id = RequestId::Number(1);
        let params = json!({"key": "value"});
        let request = JsonRpcRequest::new(id.clone(), "test_method".to_string(), Some(params.clone()));

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, id);
        assert_eq!(request.method, "test_method");
        assert_eq!(request.params, Some(params));
    }

    #[test]
    fn test_json_rpc_response_success() {
        let id = RequestId::Number(1);
        let result = json!({"status": "ok"});
        let response = JsonRpcResponse::success(id.clone(), result.clone());

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, id);
        assert_eq!(response.result, Some(result));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let id = RequestId::Number(1);
        let response = JsonRpcResponse::error(
            id.clone(),
            -32601,
            "Method not found".to_string(),
            None,
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, id);
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
    }

    #[test]
    fn test_initialize_request() {
        let request = InitializeRequest {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "test-client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized["protocol_version"], "2024-11-05");
        assert_eq!(serialized["client_info"]["name"], "test-client");
        assert_eq!(serialized["client_info"]["version"], "1.0.0");
    }

    #[test]
    fn test_initialize_response() {
        let response = InitializeResponse {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Welcome!".to_string()),
        };

        let serialized = serde_json::to_value(&response).unwrap();
        assert_eq!(serialized["protocol_version"], "2024-11-05");
        assert_eq!(serialized["server_info"]["name"], "test-server");
        assert_eq!(serialized["instructions"], "Welcome!");
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "param": { "type": "string" }
                }
            }),
        };

        let serialized = serde_json::to_value(&tool).unwrap();
        assert_eq!(serialized["name"], "test_tool");
        assert_eq!(serialized["description"], "A test tool");
    }

    #[test]
    fn test_call_tool_request() {
        let request = CallToolRequest {
            name: "test_tool".to_string(),
            arguments: json!({"param": "value"}),
        };

        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized["name"], "test_tool");
        assert_eq!(serialized["arguments"]["param"], "value");
    }

    #[test]
    fn test_call_tool_response() {
        let response = CallToolResponse {
            content: vec![Content::Text {
                text: "Result".to_string(),
            }],
            is_error: Some(false),
        };

        let serialized = serde_json::to_value(&response).unwrap();
        assert_eq!(serialized["content"][0]["type"], "text");
        assert_eq!(serialized["content"][0]["text"], "Result");
    }

    #[test]
    fn test_content_serialization() {
        // Test text content
        let text_content = Content::Text {
            text: "Hello".to_string(),
        };
        let serialized = serde_json::to_value(&text_content).unwrap();
        assert_eq!(serialized["type"], "text");
        assert_eq!(serialized["text"], "Hello");

        // Test image content
        let image_content = Content::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        };
        let serialized = serde_json::to_value(&image_content).unwrap();
        assert_eq!(serialized["type"], "image");
        assert_eq!(serialized["data"], "base64data");
        assert_eq!(serialized["mime_type"], "image/png");
    }

    #[test]
    fn test_list_tools_response() {
        let response = ListToolsResponse {
            tools: vec![Tool {
                name: "tool1".to_string(),
                description: "Tool 1".to_string(),
                input_schema: json!({}),
            }],
            next_cursor: Some("next".to_string()),
        };

        let serialized = serde_json::to_value(&response).unwrap();
        assert_eq!(serialized["tools"][0]["name"], "tool1");
        assert_eq!(serialized["next_cursor"], "next");
    }

    #[test]
    fn test_ping_request_response() {
        let request = PingRequest {};
        let serialized = serde_json::to_value(&request).unwrap();
        assert_eq!(serialized, json!({}));

        let response = PingResponse {};
        let serialized = serde_json::to_value(&response).unwrap();
        assert_eq!(serialized, json!({}));
    }
}
