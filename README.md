# MCP Toolkit

Autonomous, self-foraging MCP (Model Context Protocol) toolkit for AI agents.

Isolated from [temm1e](https://github.com/temm1e-labs/temm1e) and rebuilt as a standalone, framework-agnostic toolkit that any AI agent can use to discover, connect, and orchestrate MCP servers.

## Live Endpoint

**`https://mcp.garzaos.cloud/mcp`** — Streamable HTTP transport, ready for any MCP-compatible agent.

| URL | Purpose |
|-----|---------|
| `https://mcp.garzaos.cloud/mcp` | MCP JSON-RPC endpoint |
| `https://mcp.garzaos.cloud/health` | Health check |
| `https://mcp.garzaos.cloud/` | Server info |

## Connect Your Agent

### Claude Desktop / Claude Code

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mcp-toolkit": {
      "url": "https://mcp.garzaos.cloud/mcp",
      "transport": "http"
    }
  }
}
```

### Cursor

Add to `.cursor/mcp.json` in your project root:

```json
{
  "mcpServers": {
    "mcp-toolkit": {
      "url": "https://mcp.garzaos.cloud/mcp",
      "transport": "http"
    }
  }
}
```

### Windsurf

Add to `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "mcp-toolkit": {
      "url": "https://mcp.garzaos.cloud/mcp",
      "transport": "http"
    }
  }
}
```

### OpenAI Agents SDK (Python)

```python
from agents.mcp import MCPServerStreamableHttp
from agents import Agent

agent = Agent(
    name="my-agent",
    instructions="You have access to MCP tools.",
    mcp_servers=[
        MCPServerStreamableHttp(
            name="mcp-toolkit",
            url="https://mcp.garzaos.cloud/mcp",
        )
    ],
)
```

### TypeScript / Node.js

```typescript
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

const transport = new StreamableHTTPClientTransport(
  new URL("https://mcp.garzaos.cloud/mcp")
);
const client = new Client({ name: "my-agent", version: "1.0.0" });
await client.connect(transport);

const tools = await client.listTools();
for (const tool of tools.tools) {
  console.log(`${tool.name}: ${tool.description}`);
}
```

### Python MCP Client

```python
from mcp.client.streamable_http import streamablehttp_client
from mcp import ClientSession

async with streamablehttp_client("https://mcp.garzaos.cloud/mcp") as (read, write, _):
    async with ClientSession(read, write) as session:
        await session.initialize()
        tools = await session.list_tools()
        for tool in tools.tools:
            print(f"{tool.name}: {tool.description}")
```

### Any HTTP Client (curl)

```bash
# Initialize
curl -X POST https://mcp.garzaos.cloud/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}'

# List tools
curl -X POST https://mcp.garzaos.cloud/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
```

## Features

- **Full MCP Protocol** — Tools, Resources, Prompts, Sampling, Elicitation
- **Self-Foraging Registry** — Search 5,000+ MCP servers via Smithery.ai, npm, and the official MCP Registry
- **Three Transports** — stdio (subprocess), HTTP (remote), SSE (streaming)
- **MCP Server Mode** — Act as an MCP server itself, aggregating tools from multiple backends
- **CLI** — Manage servers, search registries, call tools from the command line
- **Capability Attestation** — SHA-256 pinning detects tool drift
- **Trust Scoring** — Rate servers by source reliability (builtin > official > smithery > npm)
- **Docker Ready** — Multi-stage Dockerfile for VPS deployment

## Self-Hosting

### Docker (recommended)

```bash
git clone https://github.com/itsablabla/mcp-toolkit.git
cd mcp-toolkit
mkdir -p config
docker compose up -d

# MCP endpoint: http://localhost:3300/mcp
```

### From Source

```bash
cargo install --path .
mcp-toolkit serve --bind 0.0.0.0:3000
```

## CLI Usage

```bash
# Initialize default config
mcp-toolkit config init

# Add a server manually
mcp-toolkit add filesystem --command npx -- -y @modelcontextprotocol/server-filesystem /tmp

# Install from the built-in registry
mcp-toolkit install playwright
mcp-toolkit install github

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

# Start as MCP server
mcp-toolkit serve --bind 0.0.0.0:3000
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

## Library Usage (Rust)

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
