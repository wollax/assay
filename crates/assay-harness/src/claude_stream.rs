//! Claude streaming NDJSON parser.
//!
//! Transforms raw Claude `--output-format stream-json --verbose --print` NDJSON
//! output into typed [`AgentEvent`] values. Unknown event types and malformed
//! lines are logged at warn level and skipped — the function always succeeds.

use std::io::BufRead;

use assay_types::AgentEvent;

/// Parse Claude streaming NDJSON into typed agent events.
///
/// Reads lines from `reader`, parses each as JSON, and maps recognized event
/// types to [`AgentEvent`] variants. Unknown event types and malformed lines
/// are logged at warn level and skipped. The function never errors — an empty
/// `Vec` is a valid outcome (agent ran but produced nothing recognizable).
pub fn parse_claude_events(reader: impl BufRead) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    let mut turn_index: u32 = 0;
    let mut seen_first_assistant = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!(error = %e, "error reading NDJSON line");
                continue;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                // Truncate to the first 80 *characters* (not bytes) to avoid
                // panicking on a multi-byte UTF-8 sequence split at a byte boundary.
                let snippet: String = trimmed.chars().take(80).collect();
                tracing::warn!(line = %snippet, "malformed NDJSON line");
                continue;
            }
        };

        let event_type = match value["type"].as_str() {
            Some(t) => t,
            None => {
                tracing::warn!("NDJSON line missing 'type' field");
                continue;
            }
        };

        match event_type {
            "assistant" => {
                // Emit TurnEnded before processing subsequent assistant events.
                if seen_first_assistant {
                    events.push(AgentEvent::TurnEnded { turn_index });
                    turn_index += 1;
                }
                seen_first_assistant = true;

                // Iterate over message.content[] blocks.
                if let Some(contents) = value["message"]["content"].as_array() {
                    for block in contents {
                        match block["type"].as_str() {
                            Some("tool_use") => {
                                let name = block["name"].as_str().unwrap_or("unknown").to_string();
                                let input_json =
                                    serde_json::to_string(&block["input"]).unwrap_or_default();
                                tracing::debug!(
                                    name = %name,
                                    "parsed ToolCalled event"
                                );
                                events.push(AgentEvent::ToolCalled { name, input_json });
                            }
                            Some("tool_result") => {
                                // Real Claude NDJSON `tool_result` blocks carry
                                // `tool_use_id` (the call reference) but NOT a
                                // `name` field.  We keep `name` as a fallback
                                // for any adapter that does include it, but in
                                // practice this field will hold the `tool_use_id`
                                // (e.g. "toolu_01"), NOT the tool name.
                                // S03's `compute_tool_call_summary` should count
                                // `ToolCalled` events (which carry the real name)
                                // rather than `ToolResult` events.
                                let name = block["tool_use_id"]
                                    .as_str()
                                    .or_else(|| block["name"].as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let output = match &block["content"] {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => serde_json::to_string(other).unwrap_or_default(),
                                };
                                let is_error = block["is_error"].as_bool().unwrap_or(false);
                                tracing::debug!(
                                    name = %name,
                                    is_error,
                                    "parsed ToolResult event"
                                );
                                events.push(AgentEvent::ToolResult {
                                    name,
                                    output,
                                    is_error,
                                });
                            }
                            // Text blocks and other content types are not
                            // relevant for tool-level event tracking.
                            _ => {}
                        }
                    }
                }
            }
            "result" => {
                let reason = value["subtype"].as_str().unwrap_or("unknown").to_string();
                let cost_usd = value["total_cost_usd"].as_f64();
                let num_turns = value["num_turns"].as_u64().unwrap_or(0) as u32;
                tracing::debug!(
                    reason = %reason,
                    cost_usd = ?cost_usd,
                    num_turns,
                    "parsed SessionStopped event"
                );
                events.push(AgentEvent::SessionStopped {
                    reason,
                    cost_usd,
                    num_turns,
                });
            }
            other => {
                tracing::warn!(event_type = other, "skipping unknown claude event type");
            }
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ── Inline NDJSON fixtures from real Claude output ──────────────────

    /// A `system/init` event — should be skipped as unknown.
    const FIXTURE_SYSTEM_INIT: &str = r#"{"type":"system","subtype":"init","session_id":"sess_abc","tools":[],"mcp_servers":[],"model":"claude-sonnet-4-20250514"}"#;

    /// An `assistant` event with a `tool_use` content block.
    const FIXTURE_ASSISTANT_TOOL_USE: &str = r#"{"type":"assistant","message":{"id":"msg_01","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_01","name":"bash","input":{"command":"ls -la"}}],"model":"claude-sonnet-4-20250514","stop_reason":"tool_use","usage":{"input_tokens":100,"output_tokens":50}},"session_id":"sess_abc"}"#;

    /// An `assistant` event with a `tool_result` content block.
    ///
    /// Note: real Claude NDJSON does NOT include `"name"` on `tool_result`
    /// blocks — only `tool_use_id` is present.  This fixture reflects actual
    /// Claude output so the `tool_use_id` fallback path is exercised.
    const FIXTURE_ASSISTANT_TOOL_RESULT: &str = r#"{"type":"assistant","message":{"id":"msg_02","type":"message","role":"assistant","content":[{"type":"tool_result","tool_use_id":"toolu_01","content":"total 42\ndrwxr-xr-x  5 user group 160 Jan  1 00:00 .","is_error":false}],"model":"claude-sonnet-4-20250514","stop_reason":"end_turn","usage":{"input_tokens":200,"output_tokens":30}},"session_id":"sess_abc"}"#;

    /// An `assistant` event with TWO `tool_use` blocks in a single message.
    const FIXTURE_ASSISTANT_TWO_TOOL_USE: &str = r#"{"type":"assistant","message":{"id":"msg_03","type":"message","role":"assistant","content":[{"type":"tool_use","id":"toolu_02","name":"read","input":{"path":"src/main.rs"}},{"type":"tool_use","id":"toolu_03","name":"edit","input":{"path":"src/main.rs","old_string":"foo","new_string":"bar"}}],"model":"claude-sonnet-4-20250514","stop_reason":"tool_use","usage":{"input_tokens":300,"output_tokens":60}},"session_id":"sess_abc"}"#;

    /// A `result` event with `subtype: "error"`.
    const FIXTURE_RESULT_ERROR: &str = r#"{"type":"result","subtype":"error","total_cost_usd":0.0010,"num_turns":1,"session_id":"sess_abc","is_error":true}"#;

    /// A `result` event missing the optional `num_turns` field.
    const FIXTURE_RESULT_NO_NUM_TURNS: &str =
        r#"{"type":"result","subtype":"success","total_cost_usd":0.0005,"session_id":"sess_abc"}"#;

    /// An `assistant` event with only a `text` content block (no tool calls).
    const FIXTURE_ASSISTANT_TEXT_ONLY: &str = r#"{"type":"assistant","message":{"id":"msg_04","type":"message","role":"assistant","content":[{"type":"text","text":"I will now read the file."}],"model":"claude-sonnet-4-20250514","stop_reason":"end_turn","usage":{"input_tokens":50,"output_tokens":20}},"session_id":"sess_abc"}"#;

    /// A `result` event with `subtype: "success"`.
    const FIXTURE_RESULT_SUCCESS: &str = r#"{"type":"result","subtype":"success","total_cost_usd":0.0042,"num_turns":3,"session_id":"sess_abc","is_error":false}"#;

    /// A malformed line (not valid JSON).
    const FIXTURE_MALFORMED: &str = r#"this is not json at all {{{ garbage"#;

    // ── Tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_tool_use_event() {
        let reader = Cursor::new(FIXTURE_ASSISTANT_TOOL_USE);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::ToolCalled { name, input_json } => {
                assert_eq!(name, "bash");
                // input_json should contain the command
                assert!(input_json.contains("ls -la"), "input_json: {input_json}");
            }
            other => panic!("expected ToolCalled, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_tool_result_event() {
        let reader = Cursor::new(FIXTURE_ASSISTANT_TOOL_RESULT);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::ToolResult {
                name,
                output,
                is_error,
            } => {
                // Real Claude NDJSON uses tool_use_id on tool_result blocks,
                // not a tool name — so name will be the call ID.
                assert_eq!(
                    name, "toolu_01",
                    "name should be tool_use_id in real Claude output"
                );
                assert!(output.contains("total 42"), "output: {output}");
                assert!(!is_error);
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_multiple_tool_use_in_one_message() {
        let reader = Cursor::new(FIXTURE_ASSISTANT_TWO_TOOL_USE);
        let events = parse_claude_events(reader);
        // Should produce two ToolCalled events in order.
        assert_eq!(events.len(), 2, "events: {events:?}");
        match &events[0] {
            AgentEvent::ToolCalled { name, .. } => assert_eq!(name, "read"),
            other => panic!("expected ToolCalled(read), got {other:?}"),
        }
        match &events[1] {
            AgentEvent::ToolCalled { name, .. } => assert_eq!(name, "edit"),
            other => panic!("expected ToolCalled(edit), got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_error_subtype() {
        let reader = Cursor::new(FIXTURE_RESULT_ERROR);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::SessionStopped {
                reason, num_turns, ..
            } => {
                assert_eq!(reason, "error");
                assert_eq!(*num_turns, 1);
            }
            other => panic!("expected SessionStopped, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_result_missing_num_turns_defaults_to_zero() {
        let reader = Cursor::new(FIXTURE_RESULT_NO_NUM_TURNS);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::SessionStopped { num_turns, .. } => {
                assert_eq!(*num_turns, 0, "missing num_turns should default to 0");
            }
            other => panic!("expected SessionStopped, got {other:?}"),
        }
    }

    #[test]
    fn test_text_only_assistant_produces_no_tool_events() {
        // A plain text assistant response should produce no events on its own
        // (it's the first assistant, so no TurnEnded either).
        let reader = Cursor::new(FIXTURE_ASSISTANT_TEXT_ONLY);
        let events = parse_claude_events(reader);
        assert!(
            events.is_empty(),
            "text-only assistant should produce no events"
        );
    }

    #[test]
    fn test_text_only_assistant_does_emit_turn_ended_as_second_event() {
        // When a text-only assistant is the second assistant event in a stream,
        // it should still emit TurnEnded for the turn that just completed.
        let stream = format!(
            "{}
{}
",
            FIXTURE_ASSISTANT_TOOL_USE, FIXTURE_ASSISTANT_TEXT_ONLY
        );
        let reader = Cursor::new(stream);
        let events = parse_claude_events(reader);
        // ToolCalled from first assistant + TurnEnded before second assistant
        assert_eq!(events.len(), 2, "events: {events:?}");
        assert!(matches!(&events[0], AgentEvent::ToolCalled { .. }));
        assert!(matches!(
            &events[1],
            AgentEvent::TurnEnded { turn_index: 0 }
        ));
    }

    #[test]
    fn test_malformed_utf8_snippet_does_not_panic() {
        // A line longer than 80 bytes containing multi-byte UTF-8 characters
        // must not panic when generating the log snippet.
        let long_emoji_line = "🦀".repeat(30); // 30 * 4 bytes = 120 bytes, 30 chars
        let reader = Cursor::new(long_emoji_line);
        let events = parse_claude_events(reader);
        // The line is not valid JSON, so it's skipped.
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_session_stopped() {
        let reader = Cursor::new(FIXTURE_RESULT_SUCCESS);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::SessionStopped {
                reason,
                cost_usd,
                num_turns,
            } => {
                assert_eq!(reason, "success");
                assert_eq!(*cost_usd, Some(0.0042));
                assert_eq!(*num_turns, 3);
            }
            other => panic!("expected SessionStopped, got {other:?}"),
        }
    }

    #[test]
    fn test_unknown_event_skipped() {
        let reader = Cursor::new(FIXTURE_SYSTEM_INIT);
        let events = parse_claude_events(reader);
        assert!(events.is_empty(), "unknown events should be skipped");
    }

    #[test]
    fn test_malformed_line_skipped() {
        let reader = Cursor::new(FIXTURE_MALFORMED);
        let events = parse_claude_events(reader);
        assert!(events.is_empty(), "malformed lines should be skipped");
    }

    #[test]
    fn test_empty_input() {
        let reader = Cursor::new("");
        let events = parse_claude_events(reader);
        assert!(events.is_empty());
    }

    #[test]
    fn test_mixed_stream() {
        // Concatenate all fixtures to simulate a real stream.
        let stream = format!(
            "{}\n{}\n{}\n{}\n{}\n",
            FIXTURE_SYSTEM_INIT,
            FIXTURE_ASSISTANT_TOOL_USE,
            FIXTURE_ASSISTANT_TOOL_RESULT,
            FIXTURE_RESULT_SUCCESS,
            FIXTURE_MALFORMED,
        );
        let reader = Cursor::new(stream);
        let events = parse_claude_events(reader);

        // Expected: ToolCalled, TurnEnded (before second assistant), ToolResult, SessionStopped
        // system/init → skipped
        // assistant (tool_use) → ToolCalled
        // assistant (tool_result) → TurnEnded { 0 } + ToolResult
        // result → SessionStopped
        // malformed → skipped
        assert_eq!(events.len(), 4, "events: {events:?}");

        assert!(
            matches!(&events[0], AgentEvent::ToolCalled { name, .. } if name == "bash"),
            "first event should be ToolCalled"
        );
        assert!(
            matches!(&events[1], AgentEvent::TurnEnded { turn_index: 0 }),
            "second event should be TurnEnded(0)"
        );
        assert!(
            // In real Claude NDJSON, tool_result uses tool_use_id (not name)
            matches!(&events[2], AgentEvent::ToolResult { name, .. } if name == "toolu_01"),
            "third event should be ToolResult with tool_use_id as name"
        );
        assert!(
            matches!(
                &events[3],
                AgentEvent::SessionStopped {
                    reason,
                    num_turns: 3,
                    ..
                } if reason == "success"
            ),
            "fourth event should be SessionStopped"
        );
    }
}
