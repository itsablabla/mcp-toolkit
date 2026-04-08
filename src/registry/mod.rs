//! Self-foraging registry — discover and install MCP servers by capability.
//!
//! Sources:
//! - Built-in registry (14 verified servers)
//! - Smithery.ai API (5,000+ servers)
//! - npm search
//! - Official MCP Registry

pub mod builtin;
pub mod remote;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A discovered MCP server entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub transport: String,
    /// Command to run (for stdio).
    pub command: Option<String>,
    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// URL (for http/sse).
    pub url: Option<String>,
    /// Headers (for http/sse).
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Environment variables needed.
    #[serde(default)]
    pub env_vars: Vec<String>,
    /// Keywords for search matching.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Source of this entry.
    pub source: RegistrySource,
    /// Trust score (0.0 - 1.0).
    pub trust_score: f64,
    /// npm package name (if applicable).
    pub npm_package: Option<String>,
}

/// Where a registry entry came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegistrySource {
    Builtin,
    Smithery,
    Npm,
    McpRegistry,
    Custom,
}

impl std::fmt::Display for RegistrySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrySource::Builtin => write!(f, "builtin"),
            RegistrySource::Smithery => write!(f, "smithery"),
            RegistrySource::Npm => write!(f, "npm"),
            RegistrySource::McpRegistry => write!(f, "mcp-registry"),
            RegistrySource::Custom => write!(f, "custom"),
        }
    }
}

/// Search for MCP servers by keyword across all sources.
pub async fn search(query: &str) -> Vec<RegistryEntry> {
    search_with_sources(query, &[RegistrySource::Builtin]).await
}

/// Search for MCP servers by keyword from specific sources.
pub async fn search_with_sources(query: &str, sources: &[RegistrySource]) -> Vec<RegistryEntry> {
    let mut results = Vec::new();

    for source in sources {
        match source {
            RegistrySource::Builtin => {
                results.extend(builtin::search(query));
            }
            RegistrySource::Smithery => match remote::search_smithery(query).await {
                Ok(entries) => results.extend(entries),
                Err(e) => tracing::warn!(error = %e, "Smithery search failed"),
            },
            RegistrySource::Npm => match remote::search_npm(query).await {
                Ok(entries) => results.extend(entries),
                Err(e) => tracing::warn!(error = %e, "npm search failed"),
            },
            RegistrySource::McpRegistry => match remote::search_mcp_registry(query).await {
                Ok(entries) => results.extend(entries),
                Err(e) => tracing::warn!(error = %e, "MCP Registry search failed"),
            },
            RegistrySource::Custom => {}
        }
    }

    // Sort by trust score descending
    results.sort_by(|a, b| {
        b.trust_score
            .partial_cmp(&a.trust_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

/// Get all entries from the built-in registry.
pub fn builtin_entries() -> Vec<RegistryEntry> {
    builtin::all_entries()
}
