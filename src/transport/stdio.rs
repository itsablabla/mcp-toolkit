//! Stdio transport — spawn MCP servers as child processes.

use crate::error::McpError;
use crate::jsonrpc::{
    parse_incoming, IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use crate::transport::Transport;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex};

/// Stdio transport — communicates with an MCP server via stdin/stdout.
pub struct StdioTransport {
    stdin: Mutex<tokio::process::ChildStdin>,
    child: Mutex<Option<Child>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    alive: Arc<AtomicBool>,
    _reader_handle: tokio::task::JoinHandle<()>,
    _incoming_tx: mpsc::Sender<IncomingMessage>,
    incoming_rx: Mutex<Option<mpsc::Receiver<IncomingMessage>>>,
    _request_id: AtomicU64,
}

impl StdioTransport {
    /// Spawn a child process and wire up stdin/stdout.
    pub async fn spawn(
        command: &str,
        args: &[String],
        env_vars: &HashMap<String, String>,
    ) -> Result<Self, McpError> {
        tracing::info!(command = %command, args = ?args, "Spawning MCP server process");

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::Transport(format!("Failed to spawn '{}': {}", command, e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("Failed to capture stdout".to_string()))?;

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let alive = Arc::new(AtomicBool::new(true));
        let (incoming_tx, incoming_rx) = mpsc::channel(256);

        let reader_pending = pending.clone();
        let reader_alive = alive.clone();
        let reader_tx = incoming_tx.clone();

        let reader_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match parse_incoming(trimmed) {
                    Some(IncomingMessage::Response(resp)) => {
                        if let Some(id) = resp.id {
                            let mut map = reader_pending.lock().await;
                            if let Some(tx) = map.remove(&id) {
                                let _ = tx.send(resp);
                            }
                        }
                    }
                    Some(msg @ IncomingMessage::Notification(_))
                    | Some(msg @ IncomingMessage::Request(_)) => {
                        let _ = reader_tx.send(msg).await;
                    }
                    None => {
                        tracing::trace!(line = %trimmed, "Ignoring non-JSON-RPC line");
                    }
                }
            }

            reader_alive.store(false, Ordering::SeqCst);
            tracing::debug!("Stdio reader loop ended");
        });

        Ok(Self {
            stdin: Mutex::new(stdin),
            child: Mutex::new(Some(child)),
            pending,
            alive,
            _reader_handle: reader_handle,
            _incoming_tx: incoming_tx,
            incoming_rx: Mutex::new(Some(incoming_rx)),
            _request_id: AtomicU64::new(1),
        })
    }

    /// Get the PID of the child process.
    pub async fn pid(&self) -> Option<u32> {
        let guard = self.child.lock().await;
        guard.as_ref().and_then(|c| c.id())
    }

    async fn write_message(&self, data: &[u8]) -> Result<(), McpError> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(data).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        if !self.alive.load(Ordering::SeqCst) {
            return Err(McpError::Transport("Process is dead".to_string()));
        }

        let id = request.id;
        let (tx, rx) = oneshot::channel();

        {
            let mut map = self.pending.lock().await;
            map.insert(id, tx);
        }

        let data = serde_json::to_vec(request)?;
        self.write_message(&data).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => {
                let mut map = self.pending.lock().await;
                map.remove(&id);
                Err(McpError::Transport("Response channel closed".to_string()))
            }
            Err(_) => {
                let mut map = self.pending.lock().await;
                map.remove(&id);
                Err(McpError::Timeout(format!(
                    "Request {} timed out after 30s",
                    id
                )))
            }
        }
    }

    async fn send_notification(&self, notification: &JsonRpcNotification) -> Result<(), McpError> {
        let data = serde_json::to_vec(notification)?;
        self.write_message(&data).await
    }

    async fn send_response(&self, response: &JsonRpcResponse) -> Result<(), McpError> {
        let data = serde_json::to_vec(response)?;
        self.write_message(&data).await
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
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            let _ = child.kill().await;
        }
        Ok(())
    }

    async fn is_alive(&self) -> bool {
        if !self.alive.load(Ordering::SeqCst) {
            return false;
        }
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_echo_process() {
        // Use `cat` as a simple echo server for testing
        let result = StdioTransport::spawn("cat", &[], &HashMap::new()).await;
        assert!(result.is_ok());
        let transport = result.unwrap();
        assert!(transport.is_alive().await);
        transport.close().await.unwrap();
    }

    #[tokio::test]
    async fn spawn_nonexistent_fails() {
        let result = StdioTransport::spawn("__nonexistent_binary__", &[], &HashMap::new()).await;
        assert!(result.is_err());
    }
}
