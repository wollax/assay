//! Types for team state checkpointing.
//!
//! These types represent a point-in-time snapshot of the agent team working
//! on a project: which agents are active, what tasks they're working on,
//! and the health of the context window.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-Level Checkpoint
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of the agent team state.
///
/// Captures which agents are active, what tasks are in progress,
/// and the health of the context window. Written to `.assay/checkpoints/`
/// as JSON frontmatter + markdown.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TeamCheckpoint {
    /// Schema version (always 1).
    pub version: u32,
    /// Session UUID from the Claude Code JSONL file.
    pub session_id: String,
    /// Absolute path to the project directory.
    pub project: String,
    /// ISO 8601 timestamp of when this checkpoint was captured.
    pub timestamp: String,
    /// What triggered this checkpoint (e.g., "manual", "hook:PostToolUse:TaskUpdate").
    pub trigger: String,
    /// All agents discovered in the session.
    pub agents: Vec<AgentState>,
    /// All tasks discovered from TaskCreate/TaskUpdate tool uses.
    pub tasks: Vec<TaskState>,
    /// Context window health snapshot, if usage data was available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_health: Option<ContextHealthSnapshot>,
}

// ---------------------------------------------------------------------------
// Agent State
// ---------------------------------------------------------------------------

/// State of a single agent (primary or subagent) in the session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentState {
    /// Agent identifier. `"primary"` for the main agent, or the `agentId`
    /// from JSONL entries for subagents.
    pub agent_id: String,
    /// Model identifier (e.g., "claude-opus-4-6").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current status of this agent.
    pub status: AgentStatus,
    /// Task this agent is currently working on, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    /// Working directory at the time of the agent's last entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    /// Whether this agent is a sidechain (subagent).
    pub is_sidechain: bool,
    /// ISO 8601 timestamp of this agent's last activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<String>,
}

/// Operational status of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is actively processing.
    Active,
    /// Agent exists but is not currently processing.
    Idle,
    /// Agent has completed its work.
    Done,
    /// Agent status could not be determined.
    Unknown,
}


impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Idle => write!(f, "idle"),
            Self::Done => write!(f, "done"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// Task State
// ---------------------------------------------------------------------------

/// State of a task discovered from TaskCreate/TaskUpdate tool uses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskState {
    /// Task identifier (sequential index from TaskCreate order, or taskId from TaskUpdate).
    pub task_id: String,
    /// Task subject line.
    pub subject: String,
    /// Optional task description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Current task status.
    pub status: TaskStatus,
    /// Agent assigned to this task, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_agent: Option<String>,
    /// ISO 8601 timestamp of the last status update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update: Option<String>,
}

/// Status of a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task has been created but not started.
    Pending,
    /// Task is actively being worked on.
    InProgress,
    /// Task has been completed.
    Completed,
    /// Task has been cancelled.
    Cancelled,
}


impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ---------------------------------------------------------------------------
// Context Health
// ---------------------------------------------------------------------------

/// Snapshot of the context window health at checkpoint time.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextHealthSnapshot {
    /// Total tokens currently in the context window.
    pub context_tokens: u64,
    /// Maximum context window size for the model.
    pub context_window: u64,
    /// Context utilization as a percentage (0.0 - 100.0).
    pub utilization_pct: f64,
    /// ISO 8601 timestamp of the last compaction, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_compaction: Option<String>,
    /// What triggered the last compaction (e.g., "auto", "manual").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction_trigger: Option<String>,
}

// ---------------------------------------------------------------------------
// Schema Registration
// ---------------------------------------------------------------------------

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "team-checkpoint",
        generate: || schemars::schema_for!(TeamCheckpoint),
    }
}
