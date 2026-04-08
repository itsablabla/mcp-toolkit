//! # MCP Toolkit — Autonomous, Self-Foraging MCP for AI Agents
//!
//! A standalone Model Context Protocol toolkit that any AI agent can use to:
//!
//! - **Connect** to MCP servers via stdio (subprocess) or HTTP/SSE
//! - **Discover** tools, resources, and prompts from connected servers
//! - **Self-forage** — search a registry of 5,000+ MCP servers by capability
//!   keyword, auto-install, and immediately use them
//! - **Serve** — act as an MCP server itself, aggregating tools from multiple
//!   backends behind a single endpoint
//!
//! ## Quick Start
//!
//! ```ignore
//! use mcp_toolkit::{McpManager, McpConfig};
//!
//! let manager = McpManager::load()?;           // Load ~/.mcp-toolkit/config.toml
//! manager.connect_all().await?;                 // Connect to all configured servers
//!
//! // List all available tools
//! let tools = manager.list_available_tools().await;
//!
//! // Call a tool
//! let result = manager.call_tool("filesystem", "read_file",
//!     serde_json::json!({"path": "/tmp/hello.txt"})).await?;
//!
//! // Self-forage: discover and install a new capability
//! let candidates = mcp_toolkit::registry::search("web search").await;
//! manager.install_server(&candidates[0]).await?;
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod jsonrpc;
pub mod manager;
pub mod registry;
pub mod server;
pub mod transport;
pub mod types;

// Re-export main types for convenience
pub use client::{McpClient, McpServerInfo, McpToolInfo};
pub use config::{McpConfig, McpServerConfig, McpSettings};
pub use error::McpError;
pub use manager::{ElicitationHandler, McpManager, SamplingHandler, ServerStatus};
pub use registry::{RegistryEntry, RegistrySource};
pub use server::{McpServer, ToolHandler, ToolHandlerResult};
pub use types::*;
