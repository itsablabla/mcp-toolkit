//! JSON-RPC 2.0 types for MCP communication.

use crate::error::McpError;
use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request (has an `id`, expects a response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification (no `id`, no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Incoming message from an MCP server — response, notification, or request.
#[derive(Debug, Clone)]
pub enum IncomingMessage {
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
    /// Server-initiated request (for sampling, elicitation, roots).
    Request(JsonRpcRequest),
}

impl JsonRpcRequest {
    pub fn new(id: impl Into<serde_json::Value>, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.to_string(),
            params,
        }
    }
}

impl JsonRpcNotification {
    pub fn new(method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }
}

impl JsonRpcResponse {
    /// Check if this response is an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Extract the result value, or return the error as McpError.
    pub fn into_result(self) -> Result<serde_json::Value, McpError> {
        if let Some(err) = self.error {
            Err(McpError::Protocol(format!(
                "JSON-RPC error {}: {}",
                err.code, err.message
            )))
        } else {
            Ok(self.result.unwrap_or(serde_json::Value::Null))
        }
    }

    /// Create a success response.
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: serde_json::Value, code: i64, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

/// Try to parse a line as a JSON-RPC response, notification, or request.
pub fn parse_incoming(line: &str) -> Option<IncomingMessage> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;

    if value.get("id").is_some() && value.get("method").is_some() {
        // Has both id and method = server-initiated request
        let req: JsonRpcRequest = serde_json::from_value(value).ok()?;
        Some(IncomingMessage::Request(req))
    } else if value.get("id").is_some() {
        // Has id but no method = response
        let resp: JsonRpcResponse = serde_json::from_value(value).ok()?;
        Some(IncomingMessage::Response(resp))
    } else if value.get("method").is_some() {
        // No id + has method = notification
        let notif: JsonRpcNotification = serde_json::from_value(value).ok()?;
        Some(IncomingMessage::Notification(notif))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_request() {
        let req = JsonRpcRequest::new(1u64, "initialize", Some(serde_json::json!({"key": "value"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn serialize_request_without_params() {
        let req = JsonRpcRequest::new(2u64, "tools/list", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("params"));
    }

    #[test]
    fn deserialize_success_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, serde_json::json!(1));
        assert!(!resp.is_error());
    }

    #[test]
    fn deserialize_error_response() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.is_error());
        let err = resp.into_result().unwrap_err();
        assert!(err.to_string().contains("Method not found"));
    }

    #[test]
    fn parse_incoming_response() {
        let line = r#"{"jsonrpc":"2.0","id":5,"result":{}}"#;
        match parse_incoming(line) {
            Some(IncomingMessage::Response(r)) => assert_eq!(r.id, serde_json::json!(5)),
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn parse_incoming_notification() {
        let line = r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#;
        match parse_incoming(line) {
            Some(IncomingMessage::Notification(n)) => {
                assert_eq!(n.method, "notifications/tools/list_changed")
            }
            _ => panic!("Expected Notification"),
        }
    }

    #[test]
    fn parse_incoming_server_request() {
        let line = r#"{"jsonrpc":"2.0","id":10,"method":"sampling/createMessage","params":{}}"#;
        match parse_incoming(line) {
            Some(IncomingMessage::Request(r)) => {
                assert_eq!(r.method, "sampling/createMessage");
                assert_eq!(r.id, serde_json::json!(10));
            }
            _ => panic!("Expected Request"),
        }
    }

    #[test]
    fn parse_incoming_garbage() {
        assert!(parse_incoming("not json").is_none());
        assert!(parse_incoming("").is_none());
        assert!(parse_incoming("{}").is_none());
    }

    #[test]
    fn response_success_helper() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"ok": true}));
        assert!(!resp.is_error());
        let val = resp.into_result().unwrap();
        assert_eq!(val["ok"], true);
    }

    #[test]
    fn response_error_helper() {
        let resp = JsonRpcResponse::error(serde_json::json!(1), -32600, "Invalid request");
        assert!(resp.is_error());
    }

    #[test]
    fn string_id_request() {
        let req = JsonRpcRequest::new(serde_json::json!("abc-123"), "tools/list", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"id\":\"abc-123\""));
    }
}
