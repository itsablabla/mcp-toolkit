//! Transport layer — abstraction over stdio and HTTP.

pub mod http;
pub mod sse;
pub mod stdio;

use crate::error::McpError;
use crate::jsonrpc::{IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Transport trait — send requests and receive responses over any channel.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON-RPC request and wait for the response.
    async fn send_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;

    /// Send a JSON-RPC notification (no response expected).
    async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), McpError>;

    /// Send a JSON-RPC response (for server-initiated requests like sampling).
    async fn send_response(&self, response: &JsonRpcResponse) -> Result<(), McpError>;

    /// Subscribe to incoming server-initiated messages (notifications + requests).
    /// Returns a receiver that yields messages as they arrive.
    async fn subscribe(&self) -> Result<mpsc::Receiver<IncomingMessage>, McpError>;

    /// Close the transport.
    async fn close(&self) -> Result<(), McpError>;

    /// Check if the transport is still alive.
    async fn is_alive(&self) -> bool;
}

/// Null transport — placeholder that rejects all operations.
pub struct NullTransport;

#[async_trait]
impl Transport for NullTransport {
    async fn send_request(&self, _request: &JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        Err(McpError::Transport(
            "NullTransport: not connected".to_string(),
        ))
    }

    async fn send_notification(&self, _notification: &JsonRpcNotification) -> Result<(), McpError> {
        Err(McpError::Transport(
            "NullTransport: not connected".to_string(),
        ))
    }

    async fn send_response(&self, _response: &JsonRpcResponse) -> Result<(), McpError> {
        Err(McpError::Transport(
            "NullTransport: not connected".to_string(),
        ))
    }

    async fn subscribe(&self) -> Result<mpsc::Receiver<IncomingMessage>, McpError> {
        Err(McpError::Transport(
            "NullTransport: not connected".to_string(),
        ))
    }

    async fn close(&self) -> Result<(), McpError> {
        Ok(())
    }

    async fn is_alive(&self) -> bool {
        false
    }
}

/// Create a transport from config.
pub async fn create_transport(
    transport_type: &str,
    command: Option<&str>,
    args: &[String],
    url: Option<&str>,
    headers: &std::collections::HashMap<String, String>,
    env_vars: &std::collections::HashMap<String, String>,
) -> Result<Arc<dyn Transport>, McpError> {
    match transport_type {
        "stdio" => {
            let cmd = command.ok_or_else(|| {
                McpError::Config("stdio transport requires a command".to_string())
            })?;
            let transport = stdio::StdioTransport::spawn(cmd, args, env_vars).await?;
            Ok(Arc::new(transport))
        }
        "http" | "streamable-http" => {
            let endpoint =
                url.ok_or_else(|| McpError::Config("HTTP transport requires a URL".to_string()))?;
            let transport = http::HttpTransport::new(endpoint, headers.clone());
            Ok(Arc::new(transport))
        }
        "sse" => {
            let endpoint =
                url.ok_or_else(|| McpError::Config("SSE transport requires a URL".to_string()))?;
            let transport = sse::SseTransport::new(endpoint, headers.clone());
            Ok(Arc::new(transport))
        }
        other => Err(McpError::Config(format!(
            "Unknown transport type: {}",
            other
        ))),
    }
}
