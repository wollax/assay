//! Typed agent event model for tool-level session visibility.
//!
//! [`AgentEvent`] normalizes the streaming output from agent CLIs (currently
//! Claude's `--output-format stream-json` NDJSON) into a typed enum that
//! downstream consumers (gate evaluation, session summaries, TUI) can work
//! against without knowing the raw event format.
//!
//! Unknown or unrecognized event types are handled by the *parser*, not by this
//! type — per decision D015, the enum does **not** use `deny_unknown_fields`
//! and does **not** include an `Unknown` variant.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema_registry;

/// A normalized event from an agent session.
///
/// Produced by parsing the streaming output of an agent CLI (e.g. Claude's
/// NDJSON stream). Non-Claude adapters emit a single synthetic
/// [`AgentEvent::SessionStopped`] on exit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A tool was invoked by the agent.
    ToolCalled {
        /// Tool name (e.g. `"bash"`, `"edit"`, `"read"`).
        name: String,
        /// JSON-encoded tool input parameters.
        input_json: String,
    },

    /// A tool returned a result to the agent.
    ToolResult {
        /// Tool name that produced this result.
        name: String,
        /// Tool output content (may be truncated by the caller).
        output: String,
        /// Whether the tool invocation resulted in an error.
        is_error: bool,
    },

    /// A conversation turn boundary.
    TurnEnded {
        /// Zero-based turn index within the session.
        turn_index: u32,
    },

    /// The agent session has stopped.
    SessionStopped {
        /// Stop reason (e.g. `"success"`, `"error"`, `"max_turns"`).
        reason: String,
        /// Total API cost in USD, if reported by the agent.
        cost_usd: Option<f64>,
        /// Number of conversation turns completed.
        num_turns: u32,
    },
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "agent-event",
        generate: || schemars::schema_for!(AgentEvent),
    }
}
