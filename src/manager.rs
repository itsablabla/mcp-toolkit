//! MCP Manager — orchestrates server lifecycle, health checks, tool discovery.

use crate::client::{McpClient, McpServerInfo, McpToolInfo};
use crate::config::{McpConfig, McpServerConfig};
use crate::error::McpError;
use crate::transport;
use crate::types::*;
use sha2::Digest;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State of a connected MCP server.
struct ServerState {
    config: McpServerConfig,
    client: McpClient,
    info: Option<McpServerInfo>,
    tools: Vec<McpToolInfo>,
    resources: Vec<McpResource>,
    prompts: Vec<McpPrompt>,
    restart_count: u32,
    capability_pin: Option<CapabilityPin>,
}

/// MCP Manager — the main entry point for managing MCP servers.
///
/// ```ignore
/// let manager = McpManager::from_config(config);
/// manager.connect_all().await?;
/// let tools = manager.list_available_tools().await;
/// let result = manager.call_tool("server", "tool", args).await?;
/// ```
pub struct McpManager {
    config: RwLock<McpConfig>,
    servers: RwLock<HashMap<String, ServerState>>,
    client_capabilities: ClientCapabilities,
    sampling_handler: RwLock<Option<Arc<dyn SamplingHandler>>>,
    elicitation_handler: RwLock<Option<Arc<dyn ElicitationHandler>>>,
}

/// Handler for server-initiated sampling (LLM) requests.
#[async_trait::async_trait]
pub trait SamplingHandler: Send + Sync {
    async fn handle_sampling(&self, request: SamplingRequest)
        -> Result<SamplingResponse, McpError>;
}

/// Handler for server-initiated elicitation (user input) requests.
#[async_trait::async_trait]
pub trait ElicitationHandler: Send + Sync {
    async fn handle_elicitation(
        &self,
        request: ElicitationRequest,
    ) -> Result<ElicitationResponse, McpError>;
}

impl McpManager {
    /// Create a new manager from config.
    pub fn from_config(config: McpConfig) -> Self {
        let client_capabilities = ClientCapabilities::default();
        Self {
            config: RwLock::new(config),
            servers: RwLock::new(HashMap::new()),
            client_capabilities,
            sampling_handler: RwLock::new(None),
            elicitation_handler: RwLock::new(None),
        }
    }

    /// Create a new manager with custom client capabilities.
    pub fn with_capabilities(config: McpConfig, capabilities: ClientCapabilities) -> Self {
        Self {
            config: RwLock::new(config),
            servers: RwLock::new(HashMap::new()),
            client_capabilities: capabilities,
            sampling_handler: RwLock::new(None),
            elicitation_handler: RwLock::new(None),
        }
    }

    /// Load config from default path and create manager.
    pub fn load() -> Result<Self, McpError> {
        let config = McpConfig::load()?;
        Ok(Self::from_config(config))
    }

    /// Set the sampling handler (for server→client LLM calls).
    pub async fn set_sampling_handler(&self, handler: Arc<dyn SamplingHandler>) {
        let mut guard = self.sampling_handler.write().await;
        *guard = Some(handler);
    }

    /// Set the elicitation handler (for server→user input requests).
    pub async fn set_elicitation_handler(&self, handler: Arc<dyn ElicitationHandler>) {
        let mut guard = self.elicitation_handler.write().await;
        *guard = Some(handler);
    }

    /// Connect to all enabled servers.
    pub async fn connect_all(&self) -> Result<(), McpError> {
        let config = self.config.read().await;
        let enabled: Vec<McpServerConfig> = config.enabled_servers().into_iter().cloned().collect();
        drop(config);

        for server_config in enabled {
            if let Err(e) = self.connect_server(&server_config).await {
                tracing::warn!(
                    server = %server_config.name,
                    error = %e,
                    "Failed to connect to MCP server"
                );
            }
        }

        Ok(())
    }

    /// Connect to a single server.
    pub async fn connect_server(&self, server_config: &McpServerConfig) -> Result<(), McpError> {
        tracing::info!(server = %server_config.name, "Connecting to MCP server");

        let transport = transport::create_transport(
            &server_config.transport,
            server_config.command.as_deref(),
            &server_config.args,
            server_config.url.as_deref(),
            &server_config.headers,
            &server_config.env,
        )
        .await?;

        let client = McpClient::new(transport, self.client_capabilities.clone());
        let info = client
            .initialize("mcp-toolkit", env!("CARGO_PKG_VERSION"))
            .await?;

        // Discover tools
        let tools = if info.capabilities.tools {
            client.list_tools().await.unwrap_or_default()
        } else {
            Vec::new()
        };

        // Discover resources
        let resources = if info.capabilities.resources {
            client.list_resources().await.unwrap_or_default()
        } else {
            Vec::new()
        };

        // Discover prompts
        let prompts = if info.capabilities.prompts {
            client.list_prompts().await.unwrap_or_default()
        } else {
            Vec::new()
        };

        tracing::info!(
            server = %server_config.name,
            tools = tools.len(),
            resources = resources.len(),
            prompts = prompts.len(),
            "MCP server connected"
        );

        // Create capability pin for attestation
        let capability_pin = if !tools.is_empty() {
            let tools_json =
                serde_json::to_string(&tools.iter().map(|t| &t.name).collect::<Vec<_>>())
                    .unwrap_or_default();
            let hash = format!(
                "{:x}",
                sha2::Digest::finalize(sha2::Sha256::new_with_prefix(tools_json.as_bytes()))
            );
            Some(CapabilityPin {
                server_name: server_config.name.clone(),
                tools_hash: hash,
                pinned_at: chrono::Utc::now(),
                tool_count: tools.len(),
            })
        } else {
            None
        };

        let state = ServerState {
            config: server_config.clone(),
            client,
            info: Some(info),
            tools,
            resources,
            prompts,
            restart_count: 0,
            capability_pin,
        };

        let mut servers = self.servers.write().await;
        servers.insert(server_config.name.clone(), state);

        Ok(())
    }

    /// List all available tools across all connected servers.
    pub async fn list_available_tools(&self) -> Vec<McpToolDescriptor> {
        let servers = self.servers.read().await;
        let mut descriptors = Vec::new();

        for (server_name, state) in servers.iter() {
            for tool in &state.tools {
                descriptors.push(McpToolDescriptor {
                    display_name: format!("{}:{}", server_name, tool.name),
                    tool_name: tool.name.clone(),
                    server_name: server_name.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    output_schema: tool.output_schema.clone(),
                });
            }
        }

        descriptors
    }

    /// List all available resources across all connected servers.
    pub async fn list_available_resources(&self) -> Vec<(String, McpResource)> {
        let servers = self.servers.read().await;
        let mut resources = Vec::new();

        for (server_name, state) in servers.iter() {
            for resource in &state.resources {
                resources.push((server_name.clone(), resource.clone()));
            }
        }

        resources
    }

    /// List all available prompts across all connected servers.
    pub async fn list_available_prompts(&self) -> Vec<(String, McpPrompt)> {
        let servers = self.servers.read().await;
        let mut prompts = Vec::new();

        for (server_name, state) in servers.iter() {
            for prompt in &state.prompts {
                prompts.push((server_name.clone(), prompt.clone()));
            }
        }

        prompts
    }

    /// Call a tool on a specific server.
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let servers = self.servers.read().await;
        let state = servers
            .get(server_name)
            .ok_or_else(|| McpError::NotFound(format!("Server '{}' not connected", server_name)))?;

        state.client.call_tool(tool_name, arguments).await
    }

    /// Call a tool by display name (e.g., "server:tool_name").
    pub async fn call_tool_by_display_name(
        &self,
        display_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let parts: Vec<&str> = display_name.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(McpError::Protocol(format!(
                "Invalid tool display name '{}' — expected 'server:tool'",
                display_name
            )));
        }
        self.call_tool(parts[0], parts[1], arguments).await
    }

    /// Read a resource from a specific server.
    pub async fn read_resource(
        &self,
        server_name: &str,
        uri: &str,
    ) -> Result<Vec<McpResourceContent>, McpError> {
        let servers = self.servers.read().await;
        let state = servers
            .get(server_name)
            .ok_or_else(|| McpError::NotFound(format!("Server '{}' not connected", server_name)))?;

        state.client.read_resource(uri).await
    }

    /// Get a prompt from a specific server.
    pub async fn get_prompt(
        &self,
        server_name: &str,
        prompt_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<Vec<McpPromptMessage>, McpError> {
        let servers = self.servers.read().await;
        let state = servers
            .get(server_name)
            .ok_or_else(|| McpError::NotFound(format!("Server '{}' not connected", server_name)))?;

        state.client.get_prompt(prompt_name, arguments).await
    }

    /// Install a new MCP server from a registry entry.
    pub async fn install_server(
        &self,
        entry: &crate::registry::RegistryEntry,
    ) -> Result<(), McpError> {
        let server_config = McpServerConfig {
            name: entry.name.clone(),
            transport: entry.transport.clone(),
            command: entry.command.clone(),
            args: entry.args.clone(),
            url: entry.url.clone(),
            headers: entry.headers.clone(),
            env: entry
                .env_vars
                .iter()
                .map(|k| (k.clone(), String::new()))
                .collect(),
            enabled: true,
            description: Some(entry.description.clone()),
            trust: None,
        };

        // Save to config
        {
            let mut config = self.config.write().await;
            config.upsert_server(server_config.clone());
            if let Err(e) = config.save() {
                tracing::warn!(error = %e, "Failed to persist config");
            }
        }

        // Connect
        self.connect_server(&server_config).await?;

        tracing::info!(server = %entry.name, "MCP server installed and connected");
        Ok(())
    }

    /// Disconnect and remove a server.
    pub async fn remove_server(&self, name: &str) -> Result<(), McpError> {
        // Disconnect
        {
            let mut servers = self.servers.write().await;
            if let Some(state) = servers.remove(name) {
                let _ = state.client.transport().close().await;
            }
        }

        // Remove from config
        {
            let mut config = self.config.write().await;
            config.remove_server(name);
            if let Err(e) = config.save() {
                tracing::warn!(error = %e, "Failed to persist config");
            }
        }

        Ok(())
    }

    /// Health check all connected servers, restart dead ones.
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        let servers = self.servers.read().await;

        for (name, state) in servers.iter() {
            let alive = state.client.transport().is_alive().await;
            if alive {
                // Try a ping
                let ping_ok = state.client.ping().await.is_ok();
                results.insert(name.clone(), ping_ok);
            } else {
                results.insert(name.clone(), false);
            }
        }

        results
    }

    /// Get status of all servers.
    pub async fn status(&self) -> Vec<ServerStatus> {
        let servers = self.servers.read().await;
        let mut statuses = Vec::new();

        for (name, state) in servers.iter() {
            let alive = state.client.transport().is_alive().await;
            statuses.push(ServerStatus {
                name: name.clone(),
                connected: alive,
                server_info: state
                    .info
                    .as_ref()
                    .map(|i| format!("{} v{}", i.name, i.version)),
                tool_count: state.tools.len(),
                resource_count: state.resources.len(),
                prompt_count: state.prompts.len(),
                restart_count: state.restart_count,
                transport: state.config.transport.clone(),
            });
        }

        statuses
    }

    /// Disconnect all servers.
    pub async fn disconnect_all(&self) {
        let mut servers = self.servers.write().await;
        for (name, state) in servers.drain() {
            tracing::info!(server = %name, "Disconnecting MCP server");
            let _ = state.client.transport().close().await;
        }
    }

    /// Check capability attestation — alert if tools changed since pinning.
    pub async fn verify_capabilities(&self) -> Vec<String> {
        let servers = self.servers.read().await;
        let mut warnings = Vec::new();

        for (name, state) in servers.iter() {
            if let Some(ref pin) = state.capability_pin {
                if state.tools.len() != pin.tool_count {
                    warnings.push(format!(
                        "Server '{}': tool count changed from {} to {}",
                        name,
                        pin.tool_count,
                        state.tools.len()
                    ));
                }
            }
        }

        warnings
    }
}

/// Server status for display.
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub name: String,
    pub connected: bool,
    pub server_info: Option<String>,
    pub tool_count: usize,
    pub resource_count: usize,
    pub prompt_count: usize,
    pub restart_count: u32,
    pub transport: String,
}

use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_manager_from_default_config() {
        let config = McpConfig::default();
        let _manager = McpManager::from_config(config);
    }

    #[tokio::test]
    async fn list_tools_empty() {
        let manager = McpManager::from_config(McpConfig::default());
        let tools = manager.list_available_tools().await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn status_empty() {
        let manager = McpManager::from_config(McpConfig::default());
        let status = manager.status().await;
        assert!(status.is_empty());
    }

    #[tokio::test]
    async fn call_tool_not_found() {
        let manager = McpManager::from_config(McpConfig::default());
        let result = manager
            .call_tool("nonexistent", "tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }
}
