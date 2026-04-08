//! MCP Toolkit CLI — manage MCP servers from the command line.

use clap::{Parser, Subcommand};
use mcp_toolkit::{McpConfig, McpManager, McpServerConfig};
use std::collections::HashMap;

#[derive(Parser)]
#[command(
    name = "mcp-toolkit",
    about = "Autonomous, self-foraging MCP toolkit for AI agents",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all configured and connected servers
    List,

    /// Show status of all connected servers
    Status,

    /// Add a new MCP server
    Add {
        /// Server name
        name: String,
        /// Transport type (stdio, http, sse)
        #[arg(short, long, default_value = "stdio")]
        transport: String,
        /// Command to run (for stdio)
        #[arg(short, long)]
        command: Option<String>,
        /// URL (for http/sse)
        #[arg(short, long)]
        url: Option<String>,
        /// Arguments (for stdio command)
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Remove an MCP server
    Remove {
        /// Server name
        name: String,
    },

    /// Search the registry for MCP servers
    Search {
        /// Search query
        query: String,
        /// Include Smithery results
        #[arg(long)]
        smithery: bool,
        /// Include npm results
        #[arg(long)]
        npm: bool,
    },

    /// Install an MCP server from the registry
    Install {
        /// Server name from the registry
        name: String,
    },

    /// Call a tool on a connected server
    Call {
        /// Server name
        server: String,
        /// Tool name
        tool: String,
        /// Arguments as JSON string
        #[arg(default_value = "{}")]
        arguments: String,
    },

    /// List tools from a specific server
    Tools {
        /// Server name (omit for all servers)
        server: Option<String>,
    },

    /// List resources from connected servers
    Resources,

    /// List prompts from connected servers
    Prompts,

    /// Health check all servers
    Health,

    /// Start as an MCP server (serve tools via HTTP)
    #[cfg(feature = "server")]
    Serve {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0:3000")]
        bind: String,
    },

    /// Show or edit configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Connect to all configured servers and run interactively
    Connect,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Show config file path
    Path,
    /// Initialize default config
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::List => cmd_list().await?,
        Commands::Status => cmd_status().await?,
        Commands::Add {
            name,
            transport,
            command,
            url,
            args,
        } => cmd_add(name, transport, command, url, args).await?,
        Commands::Remove { name } => cmd_remove(name).await?,
        Commands::Search {
            query,
            smithery,
            npm,
        } => cmd_search(query, smithery, npm).await?,
        Commands::Install { name } => cmd_install(name).await?,
        Commands::Call {
            server,
            tool,
            arguments,
        } => cmd_call(server, tool, arguments).await?,
        Commands::Tools { server } => cmd_tools(server).await?,
        Commands::Resources => cmd_resources().await?,
        Commands::Prompts => cmd_prompts().await?,
        Commands::Health => cmd_health().await?,
        #[cfg(feature = "server")]
        Commands::Serve { bind } => cmd_serve(bind).await?,
        Commands::Config { action } => cmd_config(action).await?,
        Commands::Connect => cmd_connect().await?,
    }

    Ok(())
}

async fn cmd_list() -> anyhow::Result<()> {
    let config = McpConfig::load()?;
    if config.servers.is_empty() {
        println!("No servers configured. Use `mcp-toolkit add` or `mcp-toolkit install`.");
        return Ok(());
    }
    println!(
        "{:<20} {:<10} {:<8} DESCRIPTION",
        "NAME", "TRANSPORT", "ENABLED"
    );
    println!("{}", "-".repeat(70));
    for server in &config.servers {
        println!(
            "{:<20} {:<10} {:<8} {}",
            server.name,
            server.transport,
            if server.enabled { "yes" } else { "no" },
            server.description.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

async fn cmd_status() -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    let statuses = manager.status().await;
    if statuses.is_empty() {
        println!("No servers connected.");
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<8} {:<8} {:<8} SERVER INFO",
        "NAME", "STATUS", "TOOLS", "RESOURCES", "PROMPTS"
    );
    println!("{}", "-".repeat(80));
    for s in &statuses {
        println!(
            "{:<20} {:<10} {:<8} {:<8} {:<8} {}",
            s.name,
            if s.connected { "connected" } else { "dead" },
            s.tool_count,
            s.resource_count,
            s.prompt_count,
            s.server_info.as_deref().unwrap_or("-"),
        );
    }

    manager.disconnect_all().await;
    Ok(())
}

async fn cmd_add(
    name: String,
    transport: String,
    command: Option<String>,
    url: Option<String>,
    args: Vec<String>,
) -> anyhow::Result<()> {
    let mut config = McpConfig::load()?;
    let server = McpServerConfig {
        name: name.clone(),
        transport,
        command,
        args,
        url,
        headers: HashMap::new(),
        env: HashMap::new(),
        enabled: true,
        description: None,
        trust: None,
    };
    config.upsert_server(server);
    config.save()?;
    println!("Added server '{}'", name);
    Ok(())
}

async fn cmd_remove(name: String) -> anyhow::Result<()> {
    let mut config = McpConfig::load()?;
    if config.remove_server(&name) {
        config.save()?;
        println!("Removed server '{}'", name);
    } else {
        println!("Server '{}' not found", name);
    }
    Ok(())
}

async fn cmd_search(query: String, smithery: bool, npm: bool) -> anyhow::Result<()> {
    use mcp_toolkit::registry::{self, RegistrySource};

    let mut sources = vec![RegistrySource::Builtin];
    if smithery {
        sources.push(RegistrySource::Smithery);
    }
    if npm {
        sources.push(RegistrySource::Npm);
    }

    let results = registry::search_with_sources(&query, &sources).await;

    if results.is_empty() {
        println!("No servers found for '{}'", query);
        return Ok(());
    }

    println!("{:<25} {:<10} {:<6} DESCRIPTION", "NAME", "SOURCE", "TRUST");
    println!("{}", "-".repeat(80));
    for entry in &results {
        println!(
            "{:<25} {:<10} {:<6.1} {}",
            entry.name,
            entry.source.to_string(),
            entry.trust_score,
            truncate(&entry.description, 40),
        );
    }

    Ok(())
}

async fn cmd_install(name: String) -> anyhow::Result<()> {
    // Search builtin first
    let entries = mcp_toolkit::registry::builtin::search(&name);
    let entry = entries
        .first()
        .ok_or_else(|| anyhow::anyhow!("Server '{}' not found in registry", name))?;

    let manager = McpManager::load()?;
    manager.install_server(entry).await?;
    println!("Installed and connected '{}'", name);
    manager.disconnect_all().await;
    Ok(())
}

async fn cmd_call(server: String, tool: String, arguments: String) -> anyhow::Result<()> {
    let args: serde_json::Value = serde_json::from_str(&arguments)?;
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    let result = manager.call_tool(&server, &tool, args).await?;
    if result.is_error {
        eprintln!("Error: {}", result.content);
    } else {
        println!("{}", result.content);
    }

    manager.disconnect_all().await;
    Ok(())
}

async fn cmd_tools(server_filter: Option<String>) -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    let tools = manager.list_available_tools().await;
    let filtered: Vec<_> = if let Some(ref filter) = server_filter {
        tools.iter().filter(|t| &t.server_name == filter).collect()
    } else {
        tools.iter().collect()
    };

    if filtered.is_empty() {
        println!("No tools available.");
    } else {
        println!("{:<30} {:<20} DESCRIPTION", "TOOL", "SERVER");
        println!("{}", "-".repeat(80));
        for tool in filtered {
            println!(
                "{:<30} {:<20} {}",
                tool.tool_name,
                tool.server_name,
                truncate(&tool.description, 30),
            );
        }
    }

    manager.disconnect_all().await;
    Ok(())
}

async fn cmd_resources() -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    let resources = manager.list_available_resources().await;
    if resources.is_empty() {
        println!("No resources available.");
    } else {
        println!("{:<20} {:<40} NAME", "SERVER", "URI");
        println!("{}", "-".repeat(80));
        for (server, resource) in &resources {
            println!("{:<20} {:<40} {}", server, resource.uri, resource.name);
        }
    }

    manager.disconnect_all().await;
    Ok(())
}

async fn cmd_prompts() -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    let prompts = manager.list_available_prompts().await;
    if prompts.is_empty() {
        println!("No prompts available.");
    } else {
        println!("{:<20} {:<25} DESCRIPTION", "SERVER", "NAME");
        println!("{}", "-".repeat(70));
        for (server, prompt) in &prompts {
            println!(
                "{:<20} {:<25} {}",
                server,
                prompt.name,
                prompt.description.as_deref().unwrap_or("-")
            );
        }
    }

    manager.disconnect_all().await;
    Ok(())
}

async fn cmd_health() -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    manager.connect_all().await?;

    let health = manager.health_check().await;
    if health.is_empty() {
        println!("No servers to check.");
    } else {
        for (name, ok) in &health {
            println!("{}: {}", name, if *ok { "healthy" } else { "unhealthy" });
        }
    }

    manager.disconnect_all().await;
    Ok(())
}

#[cfg(feature = "server")]
async fn cmd_serve(bind: String) -> anyhow::Result<()> {
    use mcp_toolkit::server::http_server;
    use std::sync::Arc;

    let mcp_server = Arc::new(mcp_toolkit::McpServer::new(
        "mcp-toolkit",
        env!("CARGO_PKG_VERSION"),
    ));

    // TODO: register aggregated tools from connected servers

    println!("Starting MCP server on {}", bind);
    http_server::serve(mcp_server, &bind).await?;
    Ok(())
}

async fn cmd_config(action: ConfigAction) -> anyhow::Result<()> {
    match action {
        ConfigAction::Show => {
            let config = McpConfig::load()?;
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Path => {
            println!("{}", McpConfig::default_path().display());
        }
        ConfigAction::Init => {
            let path = McpConfig::default_path();
            if path.exists() {
                println!("Config already exists at {}", path.display());
            } else {
                let config = McpConfig::default();
                config.save()?;
                println!("Created default config at {}", path.display());
            }
        }
    }
    Ok(())
}

async fn cmd_connect() -> anyhow::Result<()> {
    let manager = McpManager::load()?;
    println!("Connecting to all configured servers...");
    manager.connect_all().await?;

    let statuses = manager.status().await;
    for s in &statuses {
        println!(
            "  {} — {} ({} tools, {} resources, {} prompts)",
            s.name,
            if s.connected { "connected" } else { "failed" },
            s.tool_count,
            s.resource_count,
            s.prompt_count,
        );
    }

    println!("\nPress Ctrl+C to disconnect and exit.");
    tokio::signal::ctrl_c().await?;

    println!("Disconnecting...");
    manager.disconnect_all().await;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.min(s.len())])
    }
}
