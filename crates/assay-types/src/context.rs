//! Types for Claude Code session JSONL parsing and token diagnostics.
//!
//! These types support session file discovery, JSONL entry deserialization,
//! token usage extraction, bloat categorization, and diagnostics reporting.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// JSONL Entry Types
// ---------------------------------------------------------------------------

/// A single entry from a Claude Code session JSONL file.
///
/// Each line in a session file is a JSON object with a `type` field that
/// discriminates between entry kinds. Unknown types are captured gracefully
/// via `Unknown` to tolerate format evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SessionEntry {
    /// User message or tool result.
    User(UserEntry),
    /// Model response with content blocks and optional usage data.
    Assistant(AssistantEntry),
    /// Hook, agent, or bash progress tick.
    Progress(ProgressEntry),
    /// System entry (compact_boundary, stop_hook_summary, etc.).
    System(SystemEntry),
    /// File state snapshot — captured as raw JSON (not needed for diagnostics).
    FileHistorySnapshot(serde_json::Value),
    /// Queue management entry — captured as raw JSON.
    QueueOperation(serde_json::Value),
    /// PR reference — captured as raw JSON.
    PrLink(serde_json::Value),
    /// Catch-all for future entry types.
    #[serde(other)]
    Unknown,
}

/// Common metadata fields present on all typed JSONL entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetadata {
    /// Unique entry identifier.
    pub uuid: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Session UUID.
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Parent entry UUID (for threading).
    #[serde(rename = "parentUuid", default)]
    pub parent_uuid: Option<String>,
    /// Whether this entry belongs to a sidechain (subagent) conversation.
    #[serde(rename = "isSidechain", default)]
    pub is_sidechain: bool,
    /// Working directory at the time of the entry.
    #[serde(default)]
    pub cwd: Option<String>,
    /// Claude Code version string.
    #[serde(default)]
    pub version: Option<String>,
}

/// A user message or tool result entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEntry {
    /// Common metadata fields.
    #[serde(flatten)]
    pub meta: EntryMetadata,
    /// User message content (variable structure).
    #[serde(default)]
    pub message: Option<serde_json::Value>,
}

/// An assistant (model) response entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantEntry {
    /// Common metadata fields.
    #[serde(flatten)]
    pub meta: EntryMetadata,
    /// Structured assistant message with content blocks and usage.
    #[serde(default)]
    pub message: Option<AssistantMessage>,
}

/// The message body of an assistant entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Model identifier (e.g., "claude-sonnet-4-5-20250514").
    #[serde(default)]
    pub model: Option<String>,
    /// Content blocks (text, thinking, tool_use, tool_result).
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    /// Token usage data (present only on final response of a turn).
    #[serde(default)]
    pub usage: Option<UsageData>,
    /// Reason the model stopped generating.
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// A progress tick entry (hook_progress, agent_progress, bash_progress).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// Common metadata fields.
    #[serde(flatten)]
    pub meta: EntryMetadata,
    /// Progress data (variable structure depending on subtype).
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// A system entry (compact_boundary, stop_hook_summary, turn_duration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEntry {
    /// Common metadata fields.
    #[serde(flatten)]
    pub meta: EntryMetadata,
    /// System data (variable structure).
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Content Block Types
// ---------------------------------------------------------------------------

/// A content block within an assistant message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text response block.
    Text { text: String },
    /// Extended thinking block (ephemeral, not counted in context window).
    Thinking { thinking: String },
    /// Tool invocation block.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result block.
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
    },
    /// Catch-all for future content block types.
    #[serde(other)]
    Unknown,
}

// ---------------------------------------------------------------------------
// Token / Usage Types
// ---------------------------------------------------------------------------

/// Token usage data from the Anthropic API response.
///
/// Present on the final assistant entry of each turn (the one with `stop_reason`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct UsageData {
    /// Fresh (non-cached) input tokens.
    #[serde(default)]
    pub input_tokens: u64,
    /// Output tokens generated.
    #[serde(default)]
    pub output_tokens: u64,
    /// Tokens written to the cache during this call.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    /// Tokens read from cache during this call.
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

impl UsageData {
    /// Total tokens in the context window for this API call.
    ///
    /// This is the sum of all input-side tokens:
    /// `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`
    pub fn context_tokens(&self) -> u64 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }
}

// ---------------------------------------------------------------------------
// Bloat Categorization
// ---------------------------------------------------------------------------

/// Categories of token bloat in session files.
///
/// Each category represents a class of content that inflates session size
/// without proportional value to the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BloatCategory {
    /// Progress tick entries (hook, agent, bash progress).
    ProgressTicks,
    /// Extended thinking blocks (ephemeral, not in context window).
    ThinkingBlocks,
    /// Re-reads of files already in context.
    StaleReads,
    /// Tool result content (often large output).
    ToolOutput,
    /// Structural metadata entries (file-history-snapshot, queue-operation, etc.).
    Metadata,
    /// Injected system reminder tags within message content.
    SystemReminders,
}

impl BloatCategory {
    /// Human-readable label for this category.
    pub fn label(&self) -> &'static str {
        match self {
            Self::ProgressTicks => "Progress ticks",
            Self::ThinkingBlocks => "Thinking blocks",
            Self::StaleReads => "Stale reads",
            Self::ToolOutput => "Tool output",
            Self::Metadata => "Metadata",
            Self::SystemReminders => "System reminders",
        }
    }

    /// All bloat category variants.
    pub fn all() -> &'static [BloatCategory] {
        &[
            Self::ProgressTicks,
            Self::ThinkingBlocks,
            Self::StaleReads,
            Self::ToolOutput,
            Self::Metadata,
            Self::SystemReminders,
        ]
    }
}

/// Breakdown of bloat by category.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct BloatBreakdown {
    /// Individual bloat entries by category.
    pub entries: Vec<BloatEntry>,
}

/// A single bloat category measurement.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BloatEntry {
    /// The bloat category.
    pub category: BloatCategory,
    /// Total bytes attributed to this category.
    pub bytes: u64,
    /// Number of occurrences.
    pub count: u64,
    /// Percentage of total file size.
    pub percentage: f64,
}

// ---------------------------------------------------------------------------
// Diagnostics Report
// ---------------------------------------------------------------------------

/// Full diagnostics report for a session file.
///
/// Output of the `context diagnose` CLI command and `context_diagnose` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagnosticsReport {
    /// Session UUID.
    pub session_id: String,
    /// Path to the JSONL session file.
    pub file_path: String,
    /// Size of the session file in bytes.
    pub file_size_bytes: u64,
    /// Total number of JSONL entries parsed.
    pub total_entries: u64,
    /// Number of user + assistant message entries.
    pub message_count: u64,
    /// Model identifier from the last assistant message.
    pub model: Option<String>,
    /// Context window size for the detected model.
    pub context_window: u64,
    /// Estimated system overhead tokens (system prompt, tool definitions).
    pub system_overhead: u64,
    /// Token usage from the last assistant message.
    pub usage: Option<UsageData>,
    /// Context utilization as a percentage of the context window.
    pub context_utilization_pct: Option<f64>,
    /// Bloat breakdown by category.
    pub bloat: BloatBreakdown,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "diagnostics-report",
        generate: || schemars::schema_for!(DiagnosticsReport),
    }
}

// ---------------------------------------------------------------------------
// Session Info
// ---------------------------------------------------------------------------

/// Summary metadata for a session file (used by `context list`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionInfo {
    /// Session UUID.
    pub session_id: String,
    /// Project path (if resolved from history).
    pub project: Option<String>,
    /// Path to the JSONL session file.
    pub file_path: String,
    /// Size of the session file in bytes.
    pub file_size_bytes: u64,
    /// Number of JSONL entries in the file.
    pub entry_count: u64,
    /// Last modification time (ISO 8601).
    pub last_modified: Option<String>,
    /// Token count from last assistant usage (only populated with `--tokens` flag).
    pub token_count: Option<u64>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "session-info",
        generate: || schemars::schema_for!(SessionInfo),
    }
}

// ---------------------------------------------------------------------------
// Token Estimate
// ---------------------------------------------------------------------------

/// Token estimate for an active session (MCP `estimate_tokens` response).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenEstimate {
    /// Session UUID.
    pub session_id: String,
    /// Total context tokens (input + cache).
    pub context_tokens: u64,
    /// Output tokens from the last turn.
    pub output_tokens: u64,
    /// Context window size for the detected model.
    pub context_window: u64,
    /// Context utilization as a percentage.
    pub context_utilization_pct: f64,
    /// Health assessment based on utilization.
    pub health: ContextHealth,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "token-estimate",
        generate: || schemars::schema_for!(TokenEstimate),
    }
}

/// Context health assessment based on utilization percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextHealth {
    /// Utilization is within normal range.
    Healthy,
    /// Utilization is elevated — consider compaction.
    Warning,
    /// Utilization is near or at the context window limit.
    Critical,
}

// ---------------------------------------------------------------------------
// Claude History Entry
// ---------------------------------------------------------------------------

/// An entry from `~/.claude/history.jsonl` (session index).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeHistoryEntry {
    /// Display text (first user message or command).
    #[serde(default)]
    pub display: Option<String>,
    /// Project absolute path.
    #[serde(default)]
    pub project: Option<String>,
    /// Session UUID.
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Timestamp in milliseconds since epoch.
    #[serde(default)]
    pub timestamp: Option<u64>,
}
