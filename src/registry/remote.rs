//! Remote registry sources — Smithery, npm, official MCP Registry.

use super::{RegistryEntry, RegistrySource};
use crate::error::McpError;

/// Search Smithery.ai for MCP servers.
pub async fn search_smithery(query: &str) -> Result<Vec<RegistryEntry>, McpError> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://registry.smithery.ai/servers?q={}&pageSize=10",
        urlencoding::encode(query)
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| McpError::Registry(format!("Smithery API error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(McpError::Registry(format!(
            "Smithery returned HTTP {}",
            resp.status()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| McpError::Registry(format!("Smithery JSON parse error: {}", e)))?;

    let servers = body
        .get("servers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let entries = servers
        .into_iter()
        .filter_map(|s| {
            let name = s.get("qualifiedName")?.as_str()?.to_string();
            let description = s
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Determine transport and connection info
            let (transport, command, args, url) = if let Some(conn) = s
                .get("connections")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
            {
                let conn_type = conn.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");
                match conn_type {
                    "stdio" => {
                        let cmd = conn
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("npx")
                            .to_string();
                        let args: Vec<String> = conn
                            .get("args")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|a| a.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        ("stdio".to_string(), Some(cmd), args, None)
                    }
                    _ => {
                        let endpoint = conn.get("url").and_then(|v| v.as_str()).map(String::from);
                        ("http".to_string(), None, vec![], endpoint)
                    }
                }
            } else {
                // Default: assume npm package via npx
                (
                    "stdio".to_string(),
                    Some("npx".to_string()),
                    vec!["-y".to_string(), name.clone()],
                    None,
                )
            };

            Some(RegistryEntry {
                name: name.split('/').next_back().unwrap_or(&name).to_string(),
                description,
                transport,
                command,
                args,
                url,
                headers: Default::default(),
                env_vars: vec![],
                keywords: vec![],
                source: RegistrySource::Smithery,
                trust_score: 0.7,
                npm_package: Some(name),
            })
        })
        .collect();

    Ok(entries)
}

/// Search npm for MCP server packages.
pub async fn search_npm(query: &str) -> Result<Vec<RegistryEntry>, McpError> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://registry.npmjs.org/-/v1/search?text=mcp+server+{}&size=10",
        urlencoding::encode(query)
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Registry(format!("npm API error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(McpError::Registry(format!(
            "npm returned HTTP {}",
            resp.status()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| McpError::Registry(format!("npm JSON parse error: {}", e)))?;

    let objects = body
        .get("objects")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let entries = objects
        .into_iter()
        .filter_map(|obj| {
            let pkg = obj.get("package")?;
            let name = pkg.get("name")?.as_str()?.to_string();
            let description = pkg
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Calculate a basic trust score based on search score
            let search_score = obj
                .get("searchScore")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let trust = (search_score / 100000.0).clamp(0.1, 0.8);

            let keywords: Vec<String> = pkg
                .get("keywords")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|k| k.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Some(RegistryEntry {
                name: name.split('/').next_back().unwrap_or(&name).to_string(),
                description,
                transport: "stdio".to_string(),
                command: Some("npx".to_string()),
                args: vec!["-y".to_string(), name.clone()],
                url: None,
                headers: Default::default(),
                env_vars: vec![],
                keywords,
                source: RegistrySource::Npm,
                trust_score: trust,
                npm_package: Some(name),
            })
        })
        .collect();

    Ok(entries)
}

/// Search the official MCP Registry.
pub async fn search_mcp_registry(query: &str) -> Result<Vec<RegistryEntry>, McpError> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://registry.modelcontextprotocol.io/api/servers?q={}",
        urlencoding::encode(query)
    );

    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| McpError::Registry(format!("MCP Registry API error: {}", e)))?;

    if !resp.status().is_success() {
        // The official registry may not be publicly available yet
        return Ok(vec![]);
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| McpError::Registry(format!("MCP Registry JSON parse error: {}", e)))?;

    let servers = body
        .as_array()
        .cloned()
        .or_else(|| body.get("servers").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();

    let entries = servers
        .into_iter()
        .filter_map(|s| {
            let name = s.get("name")?.as_str()?.to_string();
            let description = s
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            Some(RegistryEntry {
                name: name.clone(),
                description,
                transport: "stdio".to_string(),
                command: Some("npx".to_string()),
                args: vec!["-y".to_string(), name.clone()],
                url: None,
                headers: Default::default(),
                env_vars: vec![],
                keywords: vec![],
                source: RegistrySource::McpRegistry,
                trust_score: 0.8,
                npm_package: Some(name),
            })
        })
        .collect();

    Ok(entries)
}

/// URL-encode a string (minimal implementation).
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                b' ' => encoded.push('+'),
                _ => {
                    encoded.push('%');
                    encoded.push_str(&format!("{:02X}", byte));
                }
            }
        }
        encoded
    }
}
