//! HTTP transport — connect to remote MCP servers via Streamable HTTP.

use crate::error::McpError;
use crate::jsonrpc::{IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::transport::Transport;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// HTTP transport — communicates with an MCP server via HTTP POST.
pub struct HttpTransport {
    endpoint: String,
    client: reqwest::Client,
    headers: HashMap<String, String>,
    session_id: Mutex<Option<String>>,
    alive: Arc<AtomicBool>,
}

impl HttpTransport {
    pub fn new(endpoint: &str, headers: HashMap<String, String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.to_string(),
            client,
            headers,
            session_id: Mutex::new(None),
            alive: Arc::new(AtomicBool::new(true)),
        }
    }

    async fn post_json(&self, body: &[u8]) -> Result<String, McpError> {
        let mut req = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");

        for (key, value) in &self.headers {
            req = req.header(key, value);
        }

        let session = self.session_id.lock().await;
        if let Some(ref sid) = *session {
            req = req.header("Mcp-Session-Id", sid);
        }
        drop(session);

        let resp = req.body(body.to_vec()).send().await?;

        // Capture session ID from response header
        if let Some(sid) = resp.headers().get("mcp-session-id") {
            if let Ok(sid_str) = sid.to_str() {
                let mut session = self.session_id.lock().await;
                *session = Some(sid_str.to_string());
            }
        }

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(McpError::Http(format!(
                "HTTP {}: {}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }

        Ok(text)
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let data = serde_json::to_vec(request)?;
        let resp_text = self.post_json(&data).await?;

        let response: JsonRpcResponse = serde_json::from_str(&resp_text).map_err(|e| {
            McpError::Protocol(format!(
                "Failed to parse HTTP response as JSON-RPC: {} (body: {})",
                e,
                resp_text.chars().take(200).collect::<String>()
            ))
        })?;

        Ok(response)
    }

    async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), McpError> {
        let data = serde_json::to_vec(notification)?;
        self.post_json(&data).await?;
        Ok(())
    }

    async fn send_response(&self, response: &JsonRpcResponse) -> Result<(), McpError> {
        let data = serde_json::to_vec(response)?;
        self.post_json(&data).await?;
        Ok(())
    }

    async fn subscribe(&self) -> Result<mpsc::Receiver<IncomingMessage>, McpError> {
        // HTTP transport doesn't support persistent subscriptions
        // (use SSE transport for that). Return an empty channel.
        let (_tx, rx) = mpsc::channel(1);
        Ok(rx)
    }

    async fn close(&self) -> Result<(), McpError> {
        self.alive.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_http_transport() {
        let transport = HttpTransport::new("http://localhost:3000/mcp", HashMap::new());
        assert_eq!(transport.endpoint, "http://localhost:3000/mcp");
    }

    #[test]
    fn create_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test".to_string());
        let transport = HttpTransport::new("http://localhost:3000", headers);
        assert_eq!(transport.headers.len(), 1);
    }
}
