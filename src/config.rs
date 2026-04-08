//! Configuration — load/save MCP server configs from TOML.

use crate::error::McpError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level MCP config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub settings: McpSettings,
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

/// Global MCP settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    /// Whether to auto-connect all servers on startup.
    #[serde(default = "default_true")]
    pub auto_connect: bool,
    /// Default timeout for tool calls (seconds).
    #[serde(default = "default_timeout")]
    pub default_timeout_secs: u64,
    /// Whether to enable the self-foraging registry.
    #[serde(default = "default_true")]
    pub registry_enabled: bool,
    /// Whether to auto-restart crashed servers.
    #[serde(default = "default_true")]
    pub auto_restart: bool,
    /// Maximum number of restart attempts.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    /// Health check interval in seconds.
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,
    /// Trust level for servers not explicitly configured.
    #[serde(default)]
    pub default_trust: crate::types::TrustLevel,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            auto_connect: true,
            default_timeout_secs: 30,
            registry_enabled: true,
            auto_restart: true,
            max_restarts: 3,
            health_check_interval_secs: 60,
            default_trust: crate::types::TrustLevel::Sandboxed,
        }
    }
}

/// Configuration for a single MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique server name.
    pub name: String,
    /// Transport type: "stdio", "http", "sse".
    #[serde(default = "default_transport")]
    pub transport: String,
    /// Command to run (for stdio transport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// URL (for http/sse transport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTTP headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Environment variables to pass to the process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether this server is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Trust level override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust: Option<crate::types::TrustLevel>,
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    30
}
fn default_max_restarts() -> u32 {
    3
}
fn default_health_interval() -> u64 {
    60
}
fn default_transport() -> String {
    "stdio".to_string()
}

impl McpConfig {
    /// Load config from the default path (~/.mcp-toolkit/config.toml).
    pub fn load() -> Result<Self, McpError> {
        let path = Self::default_path();
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &PathBuf) -> Result<Self, McpError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| McpError::Config(format!("Failed to read {}: {}", path.display(), e)))?;
        toml::from_str(&content)
            .map_err(|e| McpError::Config(format!("Failed to parse TOML: {}", e)))
    }

    /// Save config to the default path.
    pub fn save(&self) -> Result<(), McpError> {
        let path = Self::default_path();
        self.save_to(&path)
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: &PathBuf) -> Result<(), McpError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| McpError::Config(format!("Failed to create config dir: {}", e)))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| McpError::Config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, content)
            .map_err(|e| McpError::Config(format!("Failed to write {}: {}", path.display(), e)))?;
        Ok(())
    }

    /// Default config path.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mcp-toolkit")
            .join("config.toml")
    }

    /// Add or update a server config.
    pub fn upsert_server(&mut self, server: McpServerConfig) {
        if let Some(existing) = self.servers.iter_mut().find(|s| s.name == server.name) {
            *existing = server;
        } else {
            self.servers.push(server);
        }
    }

    /// Remove a server config by name.
    pub fn remove_server(&mut self, name: &str) -> bool {
        let len_before = self.servers.len();
        self.servers.retain(|s| s.name != name);
        self.servers.len() < len_before
    }

    /// Get a server config by name.
    pub fn get_server(&self, name: &str) -> Option<&McpServerConfig> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// List enabled servers.
    pub fn enabled_servers(&self) -> Vec<&McpServerConfig> {
        self.servers.iter().filter(|s| s.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = McpConfig::default();
        assert!(config.settings.auto_connect);
        assert_eq!(config.settings.default_timeout_secs, 30);
        assert!(config.servers.is_empty());
    }

    #[test]
    fn parse_toml_config() {
        let toml_str = r#"
[settings]
auto_connect = true
default_timeout_secs = 60

[[servers]]
name = "filesystem"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
enabled = true
description = "Access local filesystem"

[[servers]]
name = "remote-api"
transport = "http"
url = "https://api.example.com/mcp"
enabled = true

[servers.headers]
Authorization = "Bearer token123"
"#;
        let config: McpConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "filesystem");
        assert_eq!(config.servers[0].transport, "stdio");
        assert_eq!(config.servers[1].name, "remote-api");
        assert_eq!(config.servers[1].transport, "http");
        assert!(config.servers[1].headers.contains_key("Authorization"));
    }

    #[test]
    fn upsert_and_remove_server() {
        let mut config = McpConfig::default();
        let server = McpServerConfig {
            name: "test".to_string(),
            transport: "stdio".to_string(),
            command: Some("echo".to_string()),
            args: vec![],
            url: None,
            headers: HashMap::new(),
            env: HashMap::new(),
            enabled: true,
            description: None,
            trust: None,
        };

        config.upsert_server(server.clone());
        assert_eq!(config.servers.len(), 1);

        // Update existing
        let mut updated = server;
        updated.description = Some("Updated".to_string());
        config.upsert_server(updated);
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].description.as_deref(), Some("Updated"));

        // Remove
        assert!(config.remove_server("test"));
        assert!(config.servers.is_empty());
        assert!(!config.remove_server("nonexistent"));
    }

    #[test]
    fn serialize_config_roundtrip() {
        let mut config = McpConfig::default();
        config.upsert_server(McpServerConfig {
            name: "test".to_string(),
            transport: "stdio".to_string(),
            command: Some("echo".to_string()),
            args: vec!["hello".to_string()],
            url: None,
            headers: HashMap::new(),
            env: HashMap::new(),
            enabled: true,
            description: Some("Test server".to_string()),
            trust: None,
        });

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: McpConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.servers.len(), 1);
        assert_eq!(deserialized.servers[0].name, "test");
    }
}
