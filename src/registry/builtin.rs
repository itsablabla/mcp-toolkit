//! Built-in MCP server registry — 14 verified servers.

use super::{RegistryEntry, RegistrySource};

/// All built-in registry entries.
pub fn all_entries() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            name: "playwright".to_string(),
            description:
                "Browser automation via Playwright MCP — navigate, click, fill, screenshot"
                    .to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec!["-y".to_string(), "@playwright/mcp@latest".to_string()],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "browser".into(),
                "web".into(),
                "automation".into(),
                "click".into(),
                "screenshot".into(),
                "navigate".into(),
                "playwright".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@playwright/mcp".to_string()),
        },
        RegistryEntry {
            name: "puppeteer".to_string(),
            description: "Headless browser control via Puppeteer — navigate, screenshot, scrape"
                .to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-puppeteer@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "browser".into(),
                "web".into(),
                "puppeteer".into(),
                "headless".into(),
                "scrape".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-puppeteer".to_string()),
        },
        RegistryEntry {
            name: "filesystem".to_string(),
            description: "Read, write, search files on the local filesystem".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem@latest".to_string(),
                "/tmp".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "file".into(),
                "filesystem".into(),
                "read".into(),
                "write".into(),
                "directory".into(),
                "search".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-filesystem".to_string()),
        },
        RegistryEntry {
            name: "postgres".to_string(),
            description: "Query PostgreSQL databases — read-only by default".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-postgres@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["POSTGRES_CONNECTION_STRING".to_string()],
            keywords: vec![
                "database".into(),
                "postgres".into(),
                "sql".into(),
                "query".into(),
                "postgresql".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-postgres".to_string()),
        },
        RegistryEntry {
            name: "sqlite".to_string(),
            description: "Query SQLite databases".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-sqlite@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "database".into(),
                "sqlite".into(),
                "sql".into(),
                "query".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-sqlite".to_string()),
        },
        RegistryEntry {
            name: "github".to_string(),
            description: "Interact with GitHub — repos, issues, PRs, code search".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["GITHUB_PERSONAL_ACCESS_TOKEN".to_string()],
            keywords: vec![
                "github".into(),
                "git".into(),
                "repo".into(),
                "issues".into(),
                "pull request".into(),
                "code".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-github".to_string()),
        },
        RegistryEntry {
            name: "brave-search".to_string(),
            description: "Web search via Brave Search API".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["BRAVE_API_KEY".to_string()],
            keywords: vec![
                "search".into(),
                "web".into(),
                "brave".into(),
                "internet".into(),
                "query".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-brave-search".to_string()),
        },
        RegistryEntry {
            name: "memory".to_string(),
            description: "Knowledge graph-based persistent memory for AI agents".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-memory@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "memory".into(),
                "knowledge".into(),
                "graph".into(),
                "persistent".into(),
                "remember".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-memory".to_string()),
        },
        RegistryEntry {
            name: "fetch".to_string(),
            description: "Fetch web pages and convert to markdown".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-fetch@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "fetch".into(),
                "http".into(),
                "web".into(),
                "url".into(),
                "download".into(),
                "markdown".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-fetch".to_string()),
        },
        RegistryEntry {
            name: "slack".to_string(),
            description: "Send and read Slack messages, manage channels".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-slack@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["SLACK_BOT_TOKEN".to_string()],
            keywords: vec![
                "slack".into(),
                "chat".into(),
                "messaging".into(),
                "team".into(),
                "channel".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-slack".to_string()),
        },
        RegistryEntry {
            name: "redis".to_string(),
            description: "Read/write Redis key-value store".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-redis@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["REDIS_URL".to_string()],
            keywords: vec![
                "redis".into(),
                "cache".into(),
                "key-value".into(),
                "database".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-redis".to_string()),
        },
        RegistryEntry {
            name: "sequential-thinking".to_string(),
            description: "Step-by-step reasoning and problem decomposition".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-sequential-thinking@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec![],
            keywords: vec![
                "thinking".into(),
                "reasoning".into(),
                "logic".into(),
                "planning".into(),
                "decompose".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-sequential-thinking".to_string()),
        },
        RegistryEntry {
            name: "google-maps".to_string(),
            description: "Search places, get directions, geocoding via Google Maps".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-google-maps@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["GOOGLE_MAPS_API_KEY".to_string()],
            keywords: vec![
                "maps".into(),
                "location".into(),
                "directions".into(),
                "geocoding".into(),
                "places".into(),
                "google".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-google-maps".to_string()),
        },
        RegistryEntry {
            name: "everart".to_string(),
            description: "AI image generation via EverArt".to_string(),
            transport: "stdio".to_string(),
            command: Some("npx".to_string()),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-everart@latest".to_string(),
            ],
            url: None,
            headers: Default::default(),
            env_vars: vec!["EVERART_API_KEY".to_string()],
            keywords: vec![
                "image".into(),
                "art".into(),
                "generate".into(),
                "ai".into(),
                "creative".into(),
            ],
            source: RegistrySource::Builtin,
            trust_score: 1.0,
            npm_package: Some("@modelcontextprotocol/server-everart".to_string()),
        },
    ]
}

/// Search built-in entries by keyword.
pub fn search(query: &str) -> Vec<RegistryEntry> {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    all_entries()
        .into_iter()
        .filter(|entry| {
            query_words.iter().any(|word| {
                entry.name.to_lowercase().contains(word)
                    || entry.description.to_lowercase().contains(word)
                    || entry
                        .keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(word))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_entries_has_14() {
        assert_eq!(all_entries().len(), 14);
    }

    #[test]
    fn search_browser() {
        let results = search("browser");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.name == "playwright"));
    }

    #[test]
    fn search_database() {
        let results = search("database");
        assert!(results.iter().any(|r| r.name == "postgres"));
        assert!(results.iter().any(|r| r.name == "sqlite"));
    }

    #[test]
    fn search_no_match() {
        let results = search("xyzzy_nonexistent_12345");
        assert!(results.is_empty());
    }

    #[test]
    fn all_entries_have_trust_score() {
        for entry in all_entries() {
            assert_eq!(entry.trust_score, 1.0);
            assert_eq!(entry.source, RegistrySource::Builtin);
        }
    }
}
