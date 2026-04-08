//! MCP protocol client — handles initialize, tools, resources, prompts, sampling.

use crate::error::McpError;
use crate::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::transport::Transport;
use crate::types::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Information about a connected MCP server.
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
}

/// Raw tool info from the MCP server (before wrapping in McpToolDescriptor).
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
}

/// MCP protocol client — lifecycle, tool calls, resources, prompts.
pub struct McpClient {
    transport: Arc<dyn Transport>,
    next_id: AtomicU64,
    server_info: tokio::sync::Mutex<Option<McpServerInfo>>,
    client_capabilities: ClientCapabilities,
}

impl McpClient {
    pub fn new(transport: Arc<dyn Transport>, client_capabilities: ClientCapabilities) -> Self {
        Self {
            transport,
            next_id: AtomicU64::new(1),
            server_info: tokio::sync::Mutex::new(None),
            client_capabilities,
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Initialize the MCP connection (handshake).
    pub async fn initialize(
        &self,
        client_name: &str,
        client_version: &str,
    ) -> Result<McpServerInfo, McpError> {
        let mut capabilities = serde_json::Map::new();

        // Always request tools
        capabilities.insert("tools".to_string(), serde_json::json!({}));

        if self.client_capabilities.sampling {
            capabilities.insert("sampling".to_string(), serde_json::json!({}));
        }
        if self.client_capabilities.roots {
            capabilities.insert(
                "roots".to_string(),
                serde_json::json!({"listChanged": true}),
            );
        }
        if self.client_capabilities.elicitation {
            capabilities.insert("elicitation".to_string(), serde_json::json!({}));
        }

        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": capabilities,
            "clientInfo": {
                "name": client_name,
                "version": client_version,
            }
        });

        let req = JsonRpcRequest::new(self.next_id(), "initialize", Some(params));
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        // Parse server capabilities
        let server_caps = result.get("capabilities").cloned().unwrap_or_default();
        let capabilities = ServerCapabilities {
            tools: server_caps.get("tools").is_some(),
            resources: server_caps.get("resources").is_some(),
            resource_subscribe: server_caps
                .get("resources")
                .and_then(|r| r.get("subscribe"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            prompts: server_caps.get("prompts").is_some(),
            logging: server_caps.get("logging").is_some(),
        };

        let server_info_value = result.get("serverInfo").cloned().unwrap_or_default();
        let info = McpServerInfo {
            name: server_info_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            version: server_info_value
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0")
                .to_string(),
            protocol_version: result
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            capabilities,
        };

        // Send initialized notification
        let notif = JsonRpcNotification::new("notifications/initialized", None);
        self.transport.send_notification(&notif).await?;

        let mut guard = self.server_info.lock().await;
        *guard = Some(info.clone());

        tracing::info!(
            server = %info.name,
            version = %info.version,
            protocol = %info.protocol_version,
            "MCP server initialized"
        );

        Ok(info)
    }

    /// List tools from the MCP server.
    pub async fn list_tools(&self) -> Result<Vec<McpToolInfo>, McpError> {
        let req = JsonRpcRequest::new(self.next_id(), "tools/list", None);
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        let tools_array = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let tools = tools_array
            .into_iter()
            .filter_map(|t| {
                Some(McpToolInfo {
                    name: t.get("name")?.as_str()?.to_string(),
                    description: t
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    input_schema: t
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or(serde_json::json!({"type": "object"})),
                    output_schema: t.get("outputSchema").cloned(),
                })
            })
            .collect();

        Ok(tools)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });

        let req = JsonRpcRequest::new(self.next_id(), "tools/call", Some(params));
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let structured = result.get("structuredContent").cloned();

        let content = result
            .get("content")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        if item.get("type")?.as_str()? == "text" {
                            item.get("text").and_then(|t| t.as_str()).map(String::from)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        Ok(McpToolResult {
            content,
            is_error,
            structured,
        })
    }

    // ── Resources ───────────────────────────────────────────────────

    /// List resources from the MCP server.
    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        let req = JsonRpcRequest::new(self.next_id(), "resources/list", None);
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        let resources_array = result
            .get("resources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let resources = resources_array
            .into_iter()
            .filter_map(|r| {
                Some(McpResource {
                    uri: r.get("uri")?.as_str()?.to_string(),
                    name: r.get("name")?.as_str()?.to_string(),
                    title: r.get("title").and_then(|v| v.as_str()).map(String::from),
                    description: r
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    mime_type: r.get("mimeType").and_then(|v| v.as_str()).map(String::from),
                })
            })
            .collect();

        Ok(resources)
    }

    /// Read a resource by URI.
    pub async fn read_resource(&self, uri: &str) -> Result<Vec<McpResourceContent>, McpError> {
        let params = serde_json::json!({"uri": uri});
        let req = JsonRpcRequest::new(self.next_id(), "resources/read", Some(params));
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        let contents_array = result
            .get("contents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let contents = contents_array
            .into_iter()
            .map(|c| McpResourceContent {
                uri: c
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or(uri)
                    .to_string(),
                mime_type: c.get("mimeType").and_then(|v| v.as_str()).map(String::from),
                text: c.get("text").and_then(|v| v.as_str()).map(String::from),
                blob: c.get("blob").and_then(|v| v.as_str()).map(String::from),
            })
            .collect();

        Ok(contents)
    }

    // ── Prompts ─────────────────────────────────────────────────────

    /// List prompt templates from the MCP server.
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        let req = JsonRpcRequest::new(self.next_id(), "prompts/list", None);
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        let prompts_array = result
            .get("prompts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let prompts = prompts_array
            .into_iter()
            .filter_map(|p| serde_json::from_value(p).ok())
            .collect();

        Ok(prompts)
    }

    /// Get a prompt by name with arguments.
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<Vec<McpPromptMessage>, McpError> {
        let mut params = serde_json::json!({"name": name});
        if let Some(args) = arguments {
            params["arguments"] = args;
        }

        let req = JsonRpcRequest::new(self.next_id(), "prompts/get", Some(params));
        let resp = self.transport.send_request(&req).await?;
        let result = resp.into_result()?;

        let messages_array = result
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let messages = messages_array
            .into_iter()
            .filter_map(|m| serde_json::from_value(m).ok())
            .collect();

        Ok(messages)
    }

    // ── Sampling (handle server→client requests) ────────────────────

    /// Send a sampling response back to the server.
    pub async fn respond_to_sampling(
        &self,
        request_id: serde_json::Value,
        response: SamplingResponse,
    ) -> Result<(), McpError> {
        let resp = JsonRpcResponse::success(request_id, serde_json::to_value(response)?);
        self.transport.send_response(&resp).await
    }

    /// Send an elicitation response back to the server.
    pub async fn respond_to_elicitation(
        &self,
        request_id: serde_json::Value,
        response: ElicitationResponse,
    ) -> Result<(), McpError> {
        let resp = JsonRpcResponse::success(request_id, serde_json::to_value(response)?);
        self.transport.send_response(&resp).await
    }

    // ── Utility ─────────────────────────────────────────────────────

    /// Ping the server.
    pub async fn ping(&self) -> Result<(), McpError> {
        let req = JsonRpcRequest::new(self.next_id(), "ping", None);
        let resp = self.transport.send_request(&req).await?;
        let _ = resp.into_result()?;
        Ok(())
    }

    /// Get the transport reference.
    pub fn transport(&self) -> &Arc<dyn Transport> {
        &self.transport
    }

    /// Get cached server info (available after initialize).
    pub async fn server_info(&self) -> Option<McpServerInfo> {
        self.server_info.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_tool_info_fields() {
        let info = McpToolInfo {
            name: "test".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: None,
        };
        assert_eq!(info.name, "test");
        assert_eq!(info.description, "A test tool");
    }

    #[test]
    fn server_capabilities_default() {
        let caps = ServerCapabilities::default();
        assert!(!caps.tools);
        assert!(!caps.resources);
        assert!(!caps.prompts);
    }

    #[test]
    fn client_capabilities_default() {
        let caps = ClientCapabilities::default();
        assert!(!caps.sampling);
        assert!(!caps.roots);
        assert!(!caps.elicitation);
    }
}
