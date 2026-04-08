//! SSE (Server-Sent Events) transport — for streaming MCP communication.
//!
//! Supports the MCP Streamable HTTP pattern where:
//! - Client sends POST requests
//! - Server responds with SSE streams for long-running operations
//! - Server can push notifications via SSE

use crate::error::McpError;
use crate::jsonrpc::{
    parse_incoming, IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use crate::transport::Transport;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// Serialize a JSON-RPC id value to a canonical string key for pending map lookup.
fn id_to_key(id: &serde_json::Value) -> String {
    serde_json::to_string(id).unwrap_or_default()
}

/// SSE transport — HTTP with Server-Sent Events for bidirectional communication.
pub struct SseTransport {
    endpoint: String,
    client: reqwest::Client,
    headers: HashMap<String, String>,
    session_id: Mutex<Option<String>>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<JsonRpcResponse>>>>,
    alive: Arc<AtomicBool>,
    incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
}

impl SseTransport {
    pub fn new(endpoint: &str, headers: HashMap<String, String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_default();

        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        Self {
            endpoint: endpoint.to_string(),
            client,
            headers,
            session_id: Mutex::new(None),
            pending: Arc::new(Mutex::new(HashMap::new())),
            alive: Arc::new(AtomicBool::new(true)),
            incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
        }
    }

    /// Parse an SSE stream from response bytes.
    async fn process_sse_response(&self, text: &str) -> Vec<IncomingMessage> {
        let mut messages = Vec::new();
        let mut data_buffer = String::new();

        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                data_buffer.push_str(data);
            } else if line.is_empty() && !data_buffer.is_empty() {
                if let Some(msg) = parse_incoming(&data_buffer) {
                    messages.push(msg);
                }
                data_buffer.clear();
            }
        }

        // Handle trailing data without empty line terminator
        if !data_buffer.is_empty() {
            if let Some(msg) = parse_incoming(&data_buffer) {
                messages.push(msg);
            }
        }

        messages
    }

    async fn post_and_parse(&self, body: &[u8]) -> Result<Vec<IncomingMessage>, McpError> {
        let mut req = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream, application/json");

        for (key, value) in &self.headers {
            req = req.header(key, value);
        }

        let session = self.session_id.lock().await;
        if let Some(ref sid) = *session {
            req = req.header("Mcp-Session-Id", sid);
        }
        drop(session);

        let resp = req.body(body.to_vec()).send().await?;

        if let Some(sid) = resp.headers().get("mcp-session-id") {
            if let Ok(sid_str) = sid.to_str() {
                let mut session = self.session_id.lock().await;
                *session = Some(sid_str.to_string());
            }
        }

        let status = resp.status();
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(McpError::Http(format!(
                "HTTP {}: {}",
                status,
                text.chars().take(200).collect::<String>()
            )));
        }

        if content_type.contains("text/event-stream") {
            Ok(self.process_sse_response(&text).await)
        } else {
            // Plain JSON response
            if let Some(msg) = parse_incoming(&text) {
                Ok(vec![msg])
            } else {
                Ok(vec![])
            }
        }
    }
}

#[async_trait]
impl Transport for SseTransport {
    async fn send_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let key = id_to_key(&request.id);
        let data = serde_json::to_vec(request)?;
        let messages = self.post_and_parse(&data).await?;

        for msg in messages {
            match msg {
                IncomingMessage::Response(ref resp) if id_to_key(&resp.id) == key => {
                    if let IncomingMessage::Response(resp) = msg {
                        return Ok(resp);
                    }
                }
                IncomingMessage::Response(resp) => {
                    // Response for a different ID — route to pending
                    let resp_key = id_to_key(&resp.id);
                    let mut map = self.pending.lock().await;
                    if let Some(tx) = map.remove(&resp_key) {
                        let _ = tx.send(resp);
                    }
                }
                other => {
                    let _ = self.incoming_tx.send(other).await;
                }
            }
        }

        Err(McpError::Protocol(format!(
            "No response received for request ID {}",
            key
        )))
    }

    async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), McpError> {
        let data = serde_json::to_vec(notification)?;
        let _ = self.post_and_parse(&data).await?;
        Ok(())
    }

    async fn send_response(&self, response: &JsonRpcResponse) -> Result<(), McpError> {
        let data = serde_json::to_vec(response)?;
        let _ = self.post_and_parse(&data).await?;
        Ok(())
    }

    async fn subscribe(&self) -> Result<mpsc::Receiver<IncomingMessage>, McpError> {
        let mut rx_opt = self.incoming_rx.lock().await;
        rx_opt.take().ok_or_else(|| {
            McpError::Transport(
                "Subscribe already called — only one subscriber allowed".to_string(),
            )
        })
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
    fn create_sse_transport() {
        let transport = SseTransport::new("http://localhost:3000/mcp", HashMap::new());
        assert_eq!(transport.endpoint, "http://localhost:3000/mcp");
    }

    #[tokio::test]
    async fn parse_sse_data() {
        let transport = SseTransport::new("http://localhost:3000", HashMap::new());
        let sse_text = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n\n";
        let msgs = transport.process_sse_response(sse_text).await;
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], IncomingMessage::Response(_)));
    }

    #[tokio::test]
    async fn parse_sse_multiple_events() {
        let transport = SseTransport::new("http://localhost:3000", HashMap::new());
        let sse_text = concat!(
            "data: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\",\"params\":{}}\n\n",
            "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n"
        );
        let msgs = transport.process_sse_response(sse_text).await;
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn id_key_consistency() {
        // Verify that the same id value produces the same key
        let id = serde_json::json!("abc-123");
        assert_eq!(id_to_key(&id), id_to_key(&id));

        // Different types produce different keys
        assert_ne!(id_to_key(&serde_json::json!(1)), id_to_key(&serde_json::json!("1")));
    }
}
