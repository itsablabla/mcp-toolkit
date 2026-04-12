//! MCP Server mode — expose tools as an MCP server for other agents.
//!
//! This lets the toolkit act as a hub: aggregate multiple MCP servers
//! behind a single endpoint, or expose custom Rust tools as MCP servers.

#[cfg(feature = "server")]
pub mod http_server;

use crate::error::McpError;
use crate::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A tool handler that can be registered with the MCP server.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    async fn call(&self, arguments: serde_json::Value) -> Result<ToolHandlerResult, McpError>;
}

/// Result from a tool handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHandlerResult {
    pub content: String,
    pub is_error: bool,
}

/// MCP Server — serves tools to connecting clients.
pub struct McpServer {
    tools: RwLock<HashMap<String, Arc<dyn ToolHandler>>>,
    server_name: String,
    server_version: String,
}

impl McpServer {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
            server_name: name.to_string(),
            server_version: version.to_string(),
        }
    }

    /// Register a tool handler.
    pub async fn register_tool(&self, handler: Arc<dyn ToolHandler>) {
        let name = handler.name().to_string();
        let mut tools = self.tools.write().await;
        tools.insert(name, handler);
    }

    /// Handle an incoming JSON-RPC request (has id, expects response).
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id).await,
            "notifications/initialized" => {
                // Some clients send this as a request with id instead of notification
                JsonRpcResponse::success(request.id, serde_json::json!({}))
            }
            "tools/list" => self.handle_tools_list(request.id).await,
            "tools/call" => self.handle_tools_call(request.id, request.params).await,
            "resources/list" => self.handle_resources_list(request.id).await,
            "prompts/list" => self.handle_prompts_list(request.id).await,
            "ping" => JsonRpcResponse::success(request.id, serde_json::json!({})),
            _ => JsonRpcResponse::error(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
            ),
        }
    }

    /// Handle an incoming notification (no id, no response expected).
    pub async fn handle_notification(&self, method: &str, _params: Option<serde_json::Value>) {
        match method {
            "notifications/initialized" => {
                tracing::info!("Client sent initialized notification");
            }
            "notifications/cancelled" => {
                tracing::info!("Client cancelled request");
            }
            _ => {
                tracing::debug!(method, "Received unknown notification");
            }
        }
    }

    async fn handle_initialize(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": self.server_name,
                    "version": self.server_version,
                }
            }),
        )
    }

    async fn handle_tools_list(&self, id: serde_json::Value) -> JsonRpcResponse {
        let tools = self.tools.read().await;
        let tool_list: Vec<serde_json::Value> = tools
            .values()
            .map(|handler| {
                serde_json::json!({
                    "name": handler.name(),
                    "description": handler.description(),
                    "inputSchema": handler.input_schema(),
                })
            })
            .collect();

        JsonRpcResponse::success(id, serde_json::json!({"tools": tool_list}))
    }

    async fn handle_tools_call(
        &self,
        id: serde_json::Value,
        params: Option<serde_json::Value>,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(id, -32602, "Missing params");
            }
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name.to_string(),
            None => {
                return JsonRpcResponse::error(id, -32602, "Missing tool name");
            }
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let tools = self.tools.read().await;
        let handler = match tools.get(&tool_name) {
            Some(h) => h.clone(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    &format!("Tool not found: {}", tool_name),
                );
            }
        };
        drop(tools);

        match handler.call(arguments).await {
            Ok(result) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": result.content,
                    }],
                    "isError": result.is_error,
                }),
            ),
            Err(e) => JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Error: {}", e),
                    }],
                    "isError": true,
                }),
            ),
        }
    }

    async fn handle_resources_list(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, serde_json::json!({"resources": []}))
    }

    async fn handle_prompts_list(&self, id: serde_json::Value) -> JsonRpcResponse {
        JsonRpcResponse::success(id, serde_json::json!({"prompts": []}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait]
    impl ToolHandler for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echoes input back"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                }
            })
        }
        async fn call(&self, args: serde_json::Value) -> Result<ToolHandlerResult, McpError> {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("(empty)");
            Ok(ToolHandlerResult {
                content: text.to_string(),
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn server_initialize() {
        let server = McpServer::new("test-server", "0.1.0");
        let req = JsonRpcRequest::new(1u64, "initialize", Some(serde_json::json!({})));
        let resp = server.handle_request(req).await;
        assert!(!resp.is_error());
    }

    #[tokio::test]
    async fn server_tools_list() {
        let server = McpServer::new("test-server", "0.1.0");
        server.register_tool(Arc::new(EchoTool)).await;

        let req = JsonRpcRequest::new(2u64, "tools/list", None);
        let resp = server.handle_request(req).await;
        let result = resp.into_result().unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");
    }

    #[tokio::test]
    async fn server_tools_call() {
        let server = McpServer::new("test-server", "0.1.0");
        server.register_tool(Arc::new(EchoTool)).await;

        let req = JsonRpcRequest::new(
            3u64,
            "tools/call",
            Some(serde_json::json!({
                "name": "echo",
                "arguments": {"text": "hello"}
            })),
        );
        let resp = server.handle_request(req).await;
        let result = resp.into_result().unwrap();
        assert_eq!(result["content"][0]["text"], "hello");
    }

    #[tokio::test]
    async fn server_tool_not_found() {
        let server = McpServer::new("test-server", "0.1.0");
        let req = JsonRpcRequest::new(
            4u64,
            "tools/call",
            Some(serde_json::json!({"name": "nonexistent"})),
        );
        let resp = server.handle_request(req).await;
        assert!(resp.is_error());
    }

    #[tokio::test]
    async fn server_method_not_found() {
        let server = McpServer::new("test-server", "0.1.0");
        let req = JsonRpcRequest::new(5u64, "unknown/method", None);
        let resp = server.handle_request(req).await;
        assert!(resp.is_error());
    }

    #[tokio::test]
    async fn server_resources_list() {
        let server = McpServer::new("test-server", "0.1.0");
        let req = JsonRpcRequest::new(6u64, "resources/list", None);
        let resp = server.handle_request(req).await;
        assert!(!resp.is_error());
    }

    #[tokio::test]
    async fn server_prompts_list() {
        let server = McpServer::new("test-server", "0.1.0");
        let req = JsonRpcRequest::new(7u64, "prompts/list", None);
        let resp = server.handle_request(req).await;
        assert!(!resp.is_error());
    }
}
