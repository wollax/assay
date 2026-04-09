//! Claude streaming NDJSON parser.
//!
//! Transforms raw Claude `--output-format stream-json --verbose --print` NDJSON
//! output into typed [`AgentEvent`] values. Unknown event types and malformed
//! lines are logged at warn level and skipped — the function always succeeds.

use std::io::BufRead;

use assay_types::AgentEvent;

/// Parse Claude streaming NDJSON into typed agent events (batch).
///
/// Thin wrapper around [`parse_claude_events_streaming`] that collects all
/// emitted events into a `Vec`. Preserved for callers that want the full
/// event log up front (gates, summaries, tests). For real-time / per-token
/// delivery, use [`parse_claude_events_streaming`] directly.
pub fn parse_claude_events(reader: impl BufRead) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    parse_claude_events_streaming(reader, |e| events.push(e));
    events
}

/// Parse Claude streaming NDJSON into typed agent events via a callback.
///
/// Reads lines from `reader`, parses each as JSON, maps recognized event
/// types to [`AgentEvent`] variants, and invokes `callback` once per emitted
/// event in stream order. Unknown event types and malformed lines are logged
/// at warn level and skipped. The function never errors — zero callback
/// invocations is a valid outcome (agent ran but produced nothing
/// recognizable).
///
/// This is the canonical parser implementation. It enables real-time delivery
/// of events as they arrive on `stdout`, which is what the pipeline relay
/// thread (S03) needs to forward `TextDelta` tokens to the TUI live.
///
/// # Double-emit behaviour with `--include-partial-messages`
///
/// When Claude is invoked with `--include-partial-messages`, the same text
/// content appears **twice** in the event stream:
///
/// * [`AgentEvent::TextDelta`] — emitted incrementally from `stream_event`
///   `content_block_delta` lines as tokens arrive.
/// * [`AgentEvent::TextBlock`] — emitted once from the complete `assistant`
///   message when the block is finished.
///
/// Consumers must decide which granularity to use. The recommended split:
/// - TUI live rendering → consume `TextDelta` for per-token display.
/// - Batch consumers (gates, summaries) → consume `TextBlock` for the
///   complete, authoritative text.
/// - Do **not** concatenate both — that doubles the text content.
pub fn parse_claude_events_streaming<R, F>(reader: R, mut callback: F)
where
    R: BufRead,
    F: FnMut(AgentEvent),
{
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
                    callback(AgentEvent::TurnEnded { turn_index });
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
                                callback(AgentEvent::ToolCalled { name, input_json });
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
                                callback(AgentEvent::ToolResult {
                                    name,
                                    output,
                                    is_error,
                                });
                            }
                            Some("text") => {
                                let text = block["text"].as_str().unwrap_or("").to_string();
                                if text.is_empty() {
                                    // Skip empty text blocks — they carry no
                                    // information and add noise to the event
                                    // stream (e.g. the opening content_block_start
                                    // in assistant messages sometimes precedes an
                                    // empty text block).
                                    continue;
                                }
                                tracing::debug!(text_len = text.len(), "parsed TextBlock event");
                                callback(AgentEvent::TextBlock { text });
                            }
                            // Other content types are not relevant.
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
                callback(AgentEvent::SessionStopped {
                    reason,
                    cost_usd,
                    num_turns,
                });
            }
            "stream_event" => {
                let inner = &value["event"];
                match inner["type"].as_str() {
                    Some("content_block_delta") => {
                        if inner["delta"]["type"].as_str() == Some("text_delta") {
                            let raw = inner["delta"]["text"].as_str().unwrap_or("");
                            // Cap individual TextDelta tokens to 64 KiB to prevent
                            // unbounded per-token allocations from oversized payloads.
                            const MAX_TEXT_DELTA_BYTES: usize = 64 * 1024;
                            let text = if raw.len() > MAX_TEXT_DELTA_BYTES {
                                tracing::warn!(
                                    len = raw.len(),
                                    max = MAX_TEXT_DELTA_BYTES,
                                    "truncating oversized TextDelta"
                                );
                                raw[..MAX_TEXT_DELTA_BYTES].to_string()
                            } else {
                                raw.to_string()
                            };
                            // Saturating cast: block indices > u32::MAX are
                            // not realistic in practice but we never want a
                            // silent wraparound.
                            let raw_index = inner["index"].as_u64().unwrap_or(0);
                            let block_index = u32::try_from(raw_index).unwrap_or(u32::MAX);
                            tracing::debug!(
                                block_index,
                                text_len = text.len(),
                                "parsed TextDelta event"
                            );
                            callback(AgentEvent::TextDelta { text, block_index });
                        }
                        // Non-text_delta deltas (e.g. input_json_delta for tool
                        // streaming) are silently skipped — not relevant for
                        // text event consumers.
                    }
                    Some("content_block_start")
                    | Some("content_block_stop")
                    | Some("message_start")
                    | Some("message_delta")
                    | Some("message_stop") => {
                        // Expected noise — skip silently.
                    }
                    Some(other) => {
                        tracing::debug!(
                            stream_event_subtype = other,
                            "skipping unknown stream_event subtype"
                        );
                    }
                    None => {
                        // Include a snippet for debuggability — mirrors the
                        // malformed NDJSON handler's 80-char truncation.
                        let snippet: String = trimmed.chars().take(80).collect();
                        tracing::warn!(line = %snippet, "stream_event missing inner event.type");
                    }
                }
            }
            other => {
                tracing::warn!(event_type = other, "skipping unknown claude event type");
            }
        }
    }
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

    /// A `stream_event` with a `content_block_delta` text_delta payload.
    const FIXTURE_STREAM_EVENT_TEXT_DELTA: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello "}}}"#;

    /// A `stream_event` text_delta with non-zero block index.
    const FIXTURE_STREAM_EVENT_TEXT_DELTA_INDEX_2: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":2,"delta":{"type":"text_delta","text":"chunk"}}}"#;

    /// A `stream_event` content_block_start (should be skipped).
    const FIXTURE_STREAM_EVENT_BLOCK_START: &str = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}}"#;

    /// A `stream_event` content_block_stop (should be skipped).
    const FIXTURE_STREAM_EVENT_BLOCK_STOP: &str =
        r#"{"type":"stream_event","event":{"type":"content_block_stop","index":0}}"#;

    /// A `stream_event` message_start (should be skipped).
    const FIXTURE_STREAM_EVENT_MESSAGE_START: &str = r#"{"type":"stream_event","event":{"type":"message_start","message":{"id":"msg_x","role":"assistant"}}}"#;

    /// An assistant message with both a text block and a tool_use block.
    const FIXTURE_ASSISTANT_TEXT_AND_TOOL: &str = r#"{"type":"assistant","message":{"id":"msg_05","type":"message","role":"assistant","content":[{"type":"text","text":"I will run ls."},{"type":"tool_use","id":"toolu_05","name":"bash","input":{"command":"ls"}}],"model":"claude-sonnet-4-20250514","stop_reason":"tool_use","usage":{"input_tokens":10,"output_tokens":10}},"session_id":"sess_abc"}"#;

    /// A malformed `stream_event` line (missing inner `event` object).
    const FIXTURE_STREAM_EVENT_MALFORMED: &str = r#"{"type":"stream_event"}"#;

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
    fn test_text_only_assistant_emits_one_text_block() {
        // After M023/S02, a plain text assistant response emits exactly one
        // TextBlock event from the assistant handler (no TurnEnded — it's
        // the first assistant message in this stream).
        let reader = Cursor::new(FIXTURE_ASSISTANT_TEXT_ONLY);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TextBlock { text } => {
                assert_eq!(text, "I will now read the file.")
            }
            other => panic!("expected TextBlock, got {other:?}"),
        }
    }

    #[test]
    fn test_text_only_assistant_does_emit_turn_ended_as_second_event() {
        // When a text-only assistant is the second assistant event in a stream,
        // it should still emit TurnEnded for the turn that just completed,
        // followed by the TextBlock from its text content.
        let stream = format!(
            "{}
{}
",
            FIXTURE_ASSISTANT_TOOL_USE, FIXTURE_ASSISTANT_TEXT_ONLY
        );
        let reader = Cursor::new(stream);
        let events = parse_claude_events(reader);
        // ToolCalled from first assistant + TurnEnded before second assistant
        // + TextBlock from second assistant's text content.
        assert_eq!(events.len(), 3, "events: {events:?}");
        assert!(matches!(&events[0], AgentEvent::ToolCalled { .. }));
        assert!(matches!(
            &events[1],
            AgentEvent::TurnEnded { turn_index: 0 }
        ));
        match &events[2] {
            AgentEvent::TextBlock { text } => {
                assert_eq!(text, "I will now read the file.")
            }
            other => panic!("expected TextBlock, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_text_delta_event() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_TEXT_DELTA);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TextDelta { text, block_index } => {
                assert_eq!(text, "Hello ");
                assert_eq!(*block_index, 0);
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_text_delta_with_nonzero_index() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_TEXT_DELTA_INDEX_2);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TextDelta { text, block_index } => {
                assert_eq!(text, "chunk");
                assert_eq!(*block_index, 2);
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    #[test]
    fn test_stream_event_block_start_skipped() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_BLOCK_START);
        let events = parse_claude_events(reader);
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_block_stop_skipped() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_BLOCK_STOP);
        let events = parse_claude_events(reader);
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_message_start_skipped() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_MESSAGE_START);
        let events = parse_claude_events(reader);
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_event_malformed_skipped_without_panic() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_MALFORMED);
        let events = parse_claude_events(reader);
        assert!(events.is_empty());
    }

    #[test]
    fn test_assistant_text_and_tool_preserves_order() {
        let reader = Cursor::new(FIXTURE_ASSISTANT_TEXT_AND_TOOL);
        let events = parse_claude_events(reader);
        assert_eq!(events.len(), 2, "events: {events:?}");
        match &events[0] {
            AgentEvent::TextBlock { text } => assert_eq!(text, "I will run ls."),
            other => panic!("expected TextBlock first, got {other:?}"),
        }
        match &events[1] {
            AgentEvent::ToolCalled { name, .. } => assert_eq!(name, "bash"),
            other => panic!("expected ToolCalled second, got {other:?}"),
        }
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
    fn test_streaming_callback_invokes_per_event() {
        // Two assistant tool_use messages — the streaming parser must invoke
        // the callback once per emitted event in stream order, not all at
        // once at the end. We capture each invocation timestamp via a counter
        // bumped on every call so we can assert ordering and per-event
        // delivery without relying on real time.
        let stream = format!(
            "{}\n{}\n",
            FIXTURE_ASSISTANT_TOOL_USE, FIXTURE_ASSISTANT_TWO_TOOL_USE,
        );
        let reader = Cursor::new(stream);

        let mut received: Vec<AgentEvent> = Vec::new();
        let mut call_count: usize = 0;
        parse_claude_events_streaming(reader, |event| {
            call_count += 1;
            received.push(event);
        });

        // First assistant: ToolCalled(bash)
        // Second assistant: TurnEnded(0) before, then ToolCalled(read), ToolCalled(edit)
        // = 4 callback invocations in this exact order.
        assert_eq!(
            call_count, 4,
            "callback invoked per event, got {call_count}"
        );
        assert_eq!(received.len(), 4);
        match &received[0] {
            AgentEvent::ToolCalled { name, .. } => assert_eq!(name, "bash"),
            other => panic!("expected ToolCalled(bash), got {other:?}"),
        }
        assert!(matches!(
            &received[1],
            AgentEvent::TurnEnded { turn_index: 0 }
        ));
        match &received[2] {
            AgentEvent::ToolCalled { name, .. } => assert_eq!(name, "read"),
            other => panic!("expected ToolCalled(read), got {other:?}"),
        }
        match &received[3] {
            AgentEvent::ToolCalled { name, .. } => assert_eq!(name, "edit"),
            other => panic!("expected ToolCalled(edit), got {other:?}"),
        }
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

    // ── WOL-345: non-text_delta content_block_delta ───────────────────

    /// A `stream_event` with `input_json_delta` (tool input streaming).
    const FIXTURE_STREAM_EVENT_INPUT_JSON_DELTA: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\""}}}"#;

    #[test]
    fn input_json_delta_silently_skipped() {
        let reader = Cursor::new(FIXTURE_STREAM_EVENT_INPUT_JSON_DELTA);
        let events = parse_claude_events(reader);
        assert!(
            events.is_empty(),
            "input_json_delta should produce no events, got {events:?}"
        );
    }

    // ── WOL-346: mixed TextDelta + TextBlock double-emit ─────────────

    #[test]
    fn mixed_text_delta_and_text_block_both_emitted() {
        // When --include-partial-messages is active, the same text appears
        // as incremental TextDelta tokens AND as a final TextBlock.
        let ndjson = [
            FIXTURE_STREAM_EVENT_TEXT_DELTA,         // TextDelta "Hello "
            FIXTURE_STREAM_EVENT_TEXT_DELTA_INDEX_2, // TextDelta "chunk" (different block)
            FIXTURE_ASSISTANT_TEXT_ONLY,             // TextBlock "I will now read the file."
        ]
        .join("\n");
        let reader = Cursor::new(ndjson);
        let events = parse_claude_events(reader);

        let text_deltas: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TextDelta { .. }))
            .collect();
        let text_blocks: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TextBlock { .. }))
            .collect();

        assert_eq!(text_deltas.len(), 2, "expected 2 TextDelta events");
        assert_eq!(text_blocks.len(), 1, "expected 1 TextBlock event");
    }

    // ── WOL-347: TextDelta text length cap ───────────────────────────

    #[test]
    fn oversized_text_delta_is_truncated() {
        // Build a stream_event with a text_delta payload exceeding 64 KiB.
        let big_text = "x".repeat(70_000);
        let json = format!(
            r#"{{"type":"stream_event","event":{{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":"{big_text}"}}}}}}"#,
        );
        let reader = Cursor::new(json);
        let events = parse_claude_events(reader);

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TextDelta { text, .. } => {
                assert_eq!(text.len(), 64 * 1024, "text should be truncated to 64 KiB");
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }
}
