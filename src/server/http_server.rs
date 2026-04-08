//! HTTP server for MCP — Streamable HTTP transport (MCP spec 2025-03-26).
//!
//! Implements the full Streamable HTTP transport:
//! - POST /mcp: JSON-RPC requests, notifications, batches
//! - GET /mcp: SSE stream (or 405)
//! - DELETE /mcp: session termination
//! - Mcp-Session-Id header management
//! - CORS headers
//! - GET /health: health check
//! - GET /: server info

use crate::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::server::McpServer;
use axum::{
    extract::State,
    http::{header, HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

/// Shared server state with session tracking.
pub struct AppState {
    pub server: Arc<McpServer>,
    pub session_id: String,
    pub initialized: RwLock<bool>,
}

/// Start the MCP HTTP server with full Streamable HTTP transport support.
pub async fn serve(server: Arc<McpServer>, bind_addr: &str) -> Result<(), anyhow::Error> {
    let state = Arc::new(AppState {
        server,
        session_id: Uuid::new_v4().to_string(),
        initialized: RwLock::new(false),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::HeaderName::from_static("mcp-session-id"),
        ])
        .expose_headers([header::HeaderName::from_static("mcp-session-id")]);

    let app = Router::new()
        .route("/mcp", post(handle_mcp_post))
        .route("/mcp", get(handle_mcp_get))
        .route("/mcp", axum::routing::delete(handle_mcp_delete))
        .route("/health", get(handle_health))
        .route("/", get(handle_root))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!(addr = %bind_addr, "MCP HTTP server listening (Streamable HTTP transport)");
    axum::serve(listener, app).await?;
    Ok(())
}

/// POST /mcp — handle JSON-RPC messages (requests, notifications, batches).
async fn handle_mcp_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    // Parse body as JSON
    let body_value: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(JsonRpcResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    &format!("Parse error: {}", e),
                )),
            )
                .into_response();
        }
    };

    // Check Accept header (spec requires application/json and text/event-stream)
    // We're lenient here — accept requests even without the header
    let _ = headers.get(header::ACCEPT);

    // Determine if this is a batch (array) or single message
    if let Some(arr) = body_value.as_array() {
        handle_batch(state, arr).await
    } else {
        handle_single_message(state, &body_value).await
    }
}

/// Handle a single JSON-RPC message (request or notification).
async fn handle_single_message(state: Arc<AppState>, value: &serde_json::Value) -> Response {
    let has_id = value.get("id").is_some();
    let has_method = value.get("method").is_some();

    if !has_method {
        // This is a JSON-RPC response from the client — accept it
        return StatusCode::ACCEPTED.into_response();
    }

    if has_id {
        // This is a request — deserialize and handle
        let request: JsonRpcRequest = match serde_json::from_value(value.clone()) {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(JsonRpcResponse::error(
                        value
                            .get("id")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                        -32600,
                        &format!("Invalid request: {}", e),
                    )),
                )
                    .into_response();
            }
        };

        let is_initialize = request.method == "initialize";
        let response = state.server.handle_request(request).await;

        // Build response with appropriate headers
        let mut resp = Json(&response).into_response();

        // Add Mcp-Session-Id on initialize response
        if is_initialize {
            *state.initialized.write().await = true;
            resp.headers_mut().insert(
                header::HeaderName::from_static("mcp-session-id"),
                header::HeaderValue::from_str(&state.session_id).unwrap(),
            );
        }

        resp
    } else {
        // This is a notification — no response expected
        let method = value
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let params = value.get("params").cloned();
        state.server.handle_notification(method, params).await;

        StatusCode::ACCEPTED.into_response()
    }
}

/// Handle a batch of JSON-RPC messages.
async fn handle_batch(state: Arc<AppState>, messages: &[serde_json::Value]) -> Response {
    let mut responses: Vec<JsonRpcResponse> = Vec::new();
    let mut has_requests = false;
    let mut is_initialize_batch = false;

    for msg in messages {
        let has_id = msg.get("id").is_some();
        let has_method = msg.get("method").is_some();

        if has_id && has_method {
            // Request
            has_requests = true;
            if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(msg.clone()) {
                if request.method == "initialize" {
                    is_initialize_batch = true;
                }
                let resp = state.server.handle_request(request).await;
                responses.push(resp);
            } else {
                responses.push(JsonRpcResponse::error(
                    msg.get("id")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                    -32600,
                    "Invalid request",
                ));
            }
        } else if has_method {
            // Notification
            let method = msg
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let params = msg.get("params").cloned();
            state.server.handle_notification(method, params).await;
        }
        // Responses from client are silently accepted
    }

    if !has_requests {
        // Batch contained only notifications/responses → 202
        return StatusCode::ACCEPTED.into_response();
    }

    let mut resp = Json(&responses).into_response();
    if is_initialize_batch {
        *state.initialized.write().await = true;
        resp.headers_mut().insert(
            header::HeaderName::from_static("mcp-session-id"),
            header::HeaderValue::from_str(&state.session_id).unwrap(),
        );
    }
    resp
}

/// GET /mcp — SSE stream endpoint. Return 405 if not supported.
async fn handle_mcp_get() -> Response {
    // Per spec: server MUST either return SSE stream or 405.
    // We return 405 for now (basic server without server-initiated messages).
    (StatusCode::METHOD_NOT_ALLOWED, "SSE streaming not supported on this server").into_response()
}

/// DELETE /mcp — session termination.
async fn handle_mcp_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let session_header = headers
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok());

    if let Some(sid) = session_header {
        if sid == state.session_id {
            *state.initialized.write().await = false;
            return StatusCode::OK.into_response();
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

async fn handle_health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

async fn handle_root() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "name": "mcp-toolkit",
            "version": env!("CARGO_PKG_VERSION"),
            "protocol": "MCP",
            "transport": "streamable-http",
            "endpoint": "/mcp",
        })),
    )
}
