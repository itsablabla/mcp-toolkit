# MCP Toolkit

Autonomous, self-foraging MCP (Model Context Protocol) toolkit for AI agents.

Isolated from [temm1e](https://github.com/temm1e-labs/temm1e) and rebuilt as a standalone, framework-agnostic toolkit that any AI agent can use to discover, connect, and orchestrate MCP servers.

## Features

- **Full MCP Protocol** — Tools, Resources, Prompts, Sampling, Elicitation
- **Self-Foraging Registry** — Search 5,000+ MCP servers via Smithery.ai, npm, and the official MCP Registry
- **Three Transports** — stdio (subprocess), HTTP (remote), SSE (streaming)
- **MCP Server Mode** — Act as an MCP server itself, aggregating tools from multiple backends
- **CLI** — Manage servers, search registries, call tools from the command line
- **Capability Attestation** — SHA-256 pinning detects tool drift
- **Trust Scoring** — Rate servers by source reliability (builtin > official > smithery > npm)
- **Docker Ready** — Multi-stage Dockerfile for VPS deployment

## Quick Start

### Install from source

```bash
cargo install --path .
```

### Configure

```bash
# Initialize default config
mcp-toolkit config init

# Add a server manually
mcp-toolkit add filesystem --command npx -- -y @modelcontextprotocol/server-filesystem /tmp

# Or install from the built-in registry
mcp-toolkit install playwright
mcp-toolkit install github
```

### Use

```bash
# List configured servers
mcp-toolkit list

# Connect and show status
mcp-toolkit status

# Search for servers by capability
mcp-toolkit search "web browser"
mcp-toolkit search "database" --smithery --npm

# List available tools
mcp-toolkit tools

# Call a tool
mcp-toolkit call filesystem read_file '{"path": "/tmp/hello.txt"}'

# Health check
mcp-toolkit health
```

### Run as MCP Server

```bash
# Start HTTP server on port 3000
mcp-toolkit serve --bind 0.0.0.0:3000
```

### Docker

```bash
# Build and run
docker compose up -d

# Check logs
docker compose logs -f mcp-toolkit

# The MCP endpoint is at http://localhost:3100/mcp
```

## Configuration

Config lives at `~/.mcp-toolkit/config.toml`:

```toml
[settings]
auto_connect = true
default_timeout_secs = 30
registry_enabled = true
auto_restart = true

[[servers]]
name = "filesystem"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
enabled = true

[[servers]]
name = "remote-api"
transport = "http"
url = "https://api.example.com/mcp"
enabled = true

[servers.headers]
Authorization = "Bearer token123"
```

## Library Usage

```rust
use mcp_toolkit::{McpManager, McpConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    // List all tools across all servers
    let tools = manager.list_available_tools().await;
    for tool in &tools {
        println!("{}: {}", tool.display_name, tool.description);
    }

    // Call a tool
    let result = manager.call_tool("filesystem", "read_file",
        serde_json::json!({"path": "/tmp/hello.txt"})).await?;
    println!("{}", result.content);

    // Self-forage: discover and install a capability
    let candidates = mcp_toolkit::registry::search("web search").await;
    if let Some(entry) = candidates.first() {
        manager.install_server(entry).await?;
    }

    manager.disconnect_all().await;
    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────┐
│                 McpManager                   │
│  (orchestrates lifecycle, health, discovery) │
├─────────────────────────────────────────────┤
│  McpClient          │  McpServer            │
│  (protocol client)  │  (serve tools via     │
│                     │   HTTP/stdio)          │
├─────────────────────────────────────────────┤
│  Transport Layer                            │
│  ┌──────┐  ┌──────┐  ┌──────┐              │
│  │stdio │  │ HTTP │  │ SSE  │              │
│  └──────┘  └──────┘  └──────┘              │
├─────────────────────────────────────────────┤
│  Registry (self-foraging)                   │
│  ┌─────────┐ ┌─────┐ ┌──────────┐          │
│  │Smithery │ │ npm │ │Official  │          │
│  │  5000+  │ │     │ │Registry  │          │
│  └─────────┘ └─────┘ └──────────┘          │
└─────────────────────────────────────────────┘
```

## Built-in Registry

14 verified MCP servers available out of the box:

| Server | Description |
|--------|-------------|
| playwright | Browser automation via Playwright |
| puppeteer | Headless browser control |
| filesystem | Local file operations |
| postgres | PostgreSQL queries |
| sqlite | SQLite queries |
| github | GitHub repos, issues, PRs |
| brave-search | Web search via Brave |
| memory | Knowledge graph memory |
| fetch | Fetch web pages as markdown |
| slack | Slack messaging |
| redis | Redis key-value store |
| sequential-thinking | Step-by-step reasoning |
| google-maps | Maps, directions, geocoding |
| everart | AI image generation |

## License

MIT
