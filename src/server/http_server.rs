//! HTTP server for MCP — expose tools via HTTP endpoint.

use crate::server::McpServer;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

/// Start the MCP HTTP server.
pub async fn serve(server: Arc<McpServer>, bind_addr: &str) -> Result<(), anyhow::Error> {
    let app = Router::new()
        .route("/mcp", post(handle_mcp_request))
        .route("/health", get(handle_health))
        .route("/", get(handle_root))
        .with_state(server);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!(addr = %bind_addr, "MCP HTTP server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_mcp_request(
    State(server): State<Arc<McpServer>>,
    Json(request): Json<crate::jsonrpc::JsonRpcRequest>,
) -> impl IntoResponse {
    let response = server.handle_request(request).await;
    Json(response)
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
            "endpoint": "/mcp",
        })),
    )
}
