use thiserror::Error;

/// Standalone MCP error type — no external dependencies.
#[derive(Error, Debug)]
pub enum McpError {
    #[error("MCP protocol error: {0}")]
    Protocol(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),
}

/// Convert reqwest errors into McpError.
impl From<reqwest::Error> for McpError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            McpError::Timeout(e.to_string())
        } else {
            McpError::Http(e.to_string())
        }
    }
}
