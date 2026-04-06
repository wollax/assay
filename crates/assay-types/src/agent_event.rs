//! Typed agent event model for tool-level session visibility.
//!
//! [`AgentEvent`] normalizes the streaming output from agent CLIs (currently
//! Claude's `--output-format stream-json` NDJSON) into a typed enum that
//! downstream consumers (gate evaluation, session summaries, TUI) can work
//! against without knowing the raw event format.
//!
//! Unknown or unrecognized event types are handled by the *parser*, not by this
//! type — per decision D015, the enum does **not** use `deny_unknown_fields`
//! and does **not** include an `Unknown` variant. The `#[non_exhaustive]`
//! attribute enforces that all `match` arms must include a catch-all, making
//! forward-compatibility the compiler's responsibility.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema_registry;

/// A normalized event from an agent session.
///
/// Produced by parsing the streaming output of an agent CLI (e.g. Claude's
/// NDJSON stream). Non-Claude adapters emit a single synthetic
/// [`AgentEvent::SessionStopped`] on exit.
#[non_exhaustive]
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

    /// An incremental text chunk from a content block delta.
    ///
    /// Emitted per-token during streaming for live display consumers (TUI).
    /// Each delta carries the content block index so consumers can reconstruct
    /// per-block text streams.
    TextDelta {
        /// The text content of this delta.
        text: String,
        /// The content block index this delta belongs to (0-based).
        block_index: u32,
    },

    /// A complete text content block from an assistant message.
    ///
    /// Emitted once per text block for batch consumers (gates, summaries).
    /// Contains the full text of the block — no reconstruction needed.
    TextBlock {
        /// The full text content of the block.
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_delta_serde_roundtrip() {
        let event = AgentEvent::TextDelta {
            text: "Hello ".into(),
            block_index: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text_delta\""));
        assert!(json.contains("\"text\":\"Hello \""));
        assert!(json.contains("\"block_index\":0"));
        let roundtrip: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, roundtrip);
    }

    #[test]
    fn test_text_block_serde_roundtrip() {
        let event = AgentEvent::TextBlock {
            text: "Hello world".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text_block\""));
        assert!(json.contains("\"text\":\"Hello world\""));
        let roundtrip: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, roundtrip);
    }

    /// Verify deserialization from canonical wire-format JSON (the shape S02's
    /// parser will produce). Tests that field names and tag values are correct.
    #[test]
    fn test_text_delta_deserializes_from_wire_json() {
        let wire = r#"{"type":"text_delta","text":"hello","block_index":2}"#;
        let event: AgentEvent = serde_json::from_str(wire).unwrap();
        assert_eq!(
            event,
            AgentEvent::TextDelta {
                text: "hello".into(),
                block_index: 2
            }
        );
    }

    /// Verify deserialization from canonical wire-format JSON for TextBlock.
    #[test]
    fn test_text_block_deserializes_from_wire_json() {
        let wire = r#"{"type":"text_block","text":"complete paragraph"}"#;
        let event: AgentEvent = serde_json::from_str(wire).unwrap();
        assert_eq!(
            event,
            AgentEvent::TextBlock {
                text: "complete paragraph".into()
            }
        );
    }

    /// Verify that an empty string is valid for both text fields.
    #[test]
    fn test_text_delta_empty_text() {
        let event = AgentEvent::TextDelta {
            text: String::new(),
            block_index: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        let roundtrip: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, roundtrip);
    }
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "agent-event",
        generate: || schemars::schema_for!(AgentEvent),
    }
}
