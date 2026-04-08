//! Agent-framework-agnostic types for the MCP toolkit.
//!
//! These replace the temm1e-core types — any agent can use these directly
//! or map them to their own type system.

use serde::{Deserialize, Serialize};

// ── Tool Descriptors (what agents see) ──────────────────────────────

/// Description of a tool exposed by an MCP server.
/// Framework-agnostic — agents map this to their own tool trait.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    /// Unique display name (namespaced if collisions exist).
    pub display_name: String,
    /// Original tool name on the MCP server.
    pub tool_name: String,
    /// Which MCP server this tool belongs to.
    pub server_name: String,
    /// Human-readable description for the AI model.
    pub description: String,
    /// JSON Schema for tool parameters.
    pub input_schema: serde_json::Value,
    /// JSON Schema for structured output (MCP 2025-06-18).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<serde_json::Value>,
}

/// Result from calling an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: String,
    pub is_error: bool,
    /// Structured output if the tool declared an output schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured: Option<serde_json::Value>,
}

// ── Resource Types ──────────────────────────────────────────────────

/// A resource exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI (e.g., "file:///project/src/main.rs").
    pub uri: String,
    /// Short name.
    pub name: String,
    /// Human-readable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Content of a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Text content (if text-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64-encoded binary content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

// ── Prompt Types ────────────────────────────────────────────────────

/// A prompt template exposed by an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

/// An argument for a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

/// A message in a prompt template result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    pub role: String,
    pub content: McpMessageContent,
}

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpMessageContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: McpResourceContent },
}

// ── Sampling Types (server→client LLM calls) ───────────────────────

/// A sampling request from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingRequest {
    pub messages: Vec<SamplingMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    1024
}

/// A message in a sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    pub role: String,
    pub content: McpMessageContent,
}

/// Model preferences for sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    #[serde(default)]
    pub hints: Vec<ModelHint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<f64>,
}

/// Hint for model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    pub name: String,
}

/// Response to a sampling request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingResponse {
    pub role: String,
    pub content: McpMessageContent,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}

// ── Elicitation Types ───────────────────────────────────────────────

/// An elicitation request from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_schema: Option<serde_json::Value>,
}

/// Response to an elicitation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationResponse {
    pub action: ElicitationAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
}

/// User's action on an elicitation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationAction {
    Accept,
    Decline,
    Cancel,
}

// ── Server Capabilities ─────────────────────────────────────────────

/// Capabilities declared by an MCP server after initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub resources: bool,
    #[serde(default)]
    pub resource_subscribe: bool,
    #[serde(default)]
    pub prompts: bool,
    #[serde(default)]
    pub logging: bool,
}

/// Capabilities declared by the client during initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub sampling: bool,
    #[serde(default)]
    pub roots: bool,
    #[serde(default)]
    pub elicitation: bool,
}

// ── Security Types ──────────────────────────────────────────────────

/// Trust level for an MCP server.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Run normally with full access.
    Trusted,
    /// Run with restricted capabilities.
    #[default]
    Sandboxed,
    /// Do not run.
    Blocked,
}

/// Pinned capabilities for a server (capability attestation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityPin {
    pub server_name: String,
    /// SHA-256 hash of the tool list JSON.
    pub tools_hash: String,
    /// When the pin was created.
    pub pinned_at: chrono::DateTime<chrono::Utc>,
    /// Number of tools at pin time.
    pub tool_count: usize,
}

// ── Progress & Cancellation ─────────────────────────────────────────

/// Progress notification from a long-running operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    pub progress_token: String,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
