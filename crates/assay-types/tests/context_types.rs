//! Tests for context types: session JSONL parsing, token diagnostics, and
//! bloat categorization types.

use assay_types::context::*;

// ---------------------------------------------------------------------------
// SessionEntry deserialization
// ---------------------------------------------------------------------------

#[test]
fn deserialize_user_entry() {
    let json = r#"{
        "type": "user",
        "uuid": "abc-123",
        "timestamp": "2026-03-06T10:00:00Z",
        "sessionId": "sess-001",
        "parentUuid": null,
        "isSidechain": false,
        "cwd": "/home/user/project",
        "version": "1.0.0",
        "message": { "role": "user", "content": "Hello" }
    }"#;
    let entry: SessionEntry = serde_json::from_str(json).expect("should deserialize user entry");
    match &entry {
        SessionEntry::User(u) => {
            assert_eq!(u.meta.uuid, "abc-123");
            assert_eq!(u.meta.session_id, "sess-001");
            assert!(!u.meta.is_sidechain);
            assert!(u.message.is_some());
        }
        other => panic!("expected User, got {:?}", other),
    }
}

#[test]
fn deserialize_assistant_entry_with_usage() {
    let json = r#"{
        "type": "assistant",
        "uuid": "def-456",
        "timestamp": "2026-03-06T10:01:00Z",
        "sessionId": "sess-001",
        "isSidechain": false,
        "message": {
            "model": "claude-sonnet-4-5-20250514",
            "content": [
                { "type": "text", "text": "Here is the answer." }
            ],
            "usage": {
                "input_tokens": 3,
                "output_tokens": 13,
                "cache_creation_input_tokens": 3502,
                "cache_read_input_tokens": 26514
            },
            "stop_reason": "end_turn"
        }
    }"#;
    let entry: SessionEntry =
        serde_json::from_str(json).expect("should deserialize assistant entry");
    match &entry {
        SessionEntry::Assistant(a) => {
            assert_eq!(a.meta.uuid, "def-456");
            let msg = a.message.as_ref().expect("should have message");
            assert_eq!(msg.model.as_deref(), Some("claude-sonnet-4-5-20250514"));
            assert_eq!(msg.content.len(), 1);
            let usage = msg.usage.as_ref().expect("should have usage");
            assert_eq!(usage.input_tokens, 3);
            assert_eq!(usage.output_tokens, 13);
            assert_eq!(usage.cache_creation_input_tokens, 3502);
            assert_eq!(usage.cache_read_input_tokens, 26514);
            assert_eq!(usage.context_tokens(), 3 + 3502 + 26514);
        }
        other => panic!("expected Assistant, got {:?}", other),
    }
}

#[test]
fn deserialize_progress_entry() {
    let json = r#"{
        "type": "progress",
        "uuid": "prog-001",
        "timestamp": "2026-03-06T10:02:00Z",
        "sessionId": "sess-001",
        "isSidechain": false,
        "data": { "type": "hook_progress", "status": "running" }
    }"#;
    let entry: SessionEntry =
        serde_json::from_str(json).expect("should deserialize progress entry");
    match &entry {
        SessionEntry::Progress(p) => {
            assert_eq!(p.meta.uuid, "prog-001");
            assert!(p.data.is_some());
        }
        other => panic!("expected Progress, got {:?}", other),
    }
}

#[test]
fn deserialize_system_entry() {
    let json = r#"{
        "type": "system",
        "uuid": "sys-001",
        "timestamp": "2026-03-06T10:03:00Z",
        "sessionId": "sess-001",
        "isSidechain": false,
        "data": { "type": "compact_boundary" }
    }"#;
    let entry: SessionEntry = serde_json::from_str(json).expect("should deserialize system entry");
    match &entry {
        SessionEntry::System(s) => {
            assert_eq!(s.meta.uuid, "sys-001");
        }
        other => panic!("expected System, got {:?}", other),
    }
}

#[test]
fn deserialize_file_history_snapshot() {
    let json = r#"{
        "type": "file-history-snapshot",
        "uuid": "fhs-001",
        "timestamp": "2026-03-06T10:04:00Z",
        "sessionId": "sess-001",
        "isSidechain": false,
        "files": [{"path": "src/main.rs", "content": "fn main() {}"}]
    }"#;
    let entry: SessionEntry =
        serde_json::from_str(json).expect("should deserialize file-history-snapshot");
    assert!(matches!(entry, SessionEntry::FileHistorySnapshot(_)));
}

#[test]
fn deserialize_queue_operation() {
    let json = r#"{
        "type": "queue-operation",
        "uuid": "qo-001",
        "timestamp": "2026-03-06T10:05:00Z",
        "sessionId": "sess-001",
        "isSidechain": false,
        "operation": "enqueue"
    }"#;
    let entry: SessionEntry =
        serde_json::from_str(json).expect("should deserialize queue-operation");
    assert!(matches!(entry, SessionEntry::QueueOperation(_)));
}

#[test]
fn deserialize_pr_link() {
    let json = r#"{
        "type": "pr-link",
        "uuid": "pr-001",
        "timestamp": "2026-03-06T10:06:00Z",
        "sessionId": "sess-001",
        "isSidechain": false,
        "url": "https://github.com/org/repo/pull/42"
    }"#;
    let entry: SessionEntry = serde_json::from_str(json).expect("should deserialize pr-link");
    assert!(matches!(entry, SessionEntry::PrLink(_)));
}

#[test]
fn deserialize_unknown_entry_type() {
    let json = r#"{
        "type": "future-new-type",
        "uuid": "unk-001",
        "timestamp": "2026-03-06T10:07:00Z",
        "sessionId": "sess-001"
    }"#;
    let entry: SessionEntry =
        serde_json::from_str(json).expect("unknown type should deserialize gracefully");
    assert!(matches!(entry, SessionEntry::Unknown));
}

// ---------------------------------------------------------------------------
// UsageData
// ---------------------------------------------------------------------------

#[test]
fn usage_data_context_tokens() {
    let usage = UsageData {
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 200,
        cache_read_input_tokens: 300,
    };
    assert_eq!(usage.context_tokens(), 600);
}

#[test]
fn usage_data_default_is_zero() {
    let usage = UsageData::default();
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.cache_creation_input_tokens, 0);
    assert_eq!(usage.cache_read_input_tokens, 0);
    assert_eq!(usage.context_tokens(), 0);
}

// ---------------------------------------------------------------------------
// BloatCategory
// ---------------------------------------------------------------------------

#[test]
fn bloat_category_all_returns_six_variants() {
    let all = BloatCategory::all();
    assert_eq!(all.len(), 6);

    // Verify all expected variants are present.
    assert!(all.contains(&BloatCategory::ProgressTicks));
    assert!(all.contains(&BloatCategory::ThinkingBlocks));
    assert!(all.contains(&BloatCategory::StaleReads));
    assert!(all.contains(&BloatCategory::ToolOutput));
    assert!(all.contains(&BloatCategory::Metadata));
    assert!(all.contains(&BloatCategory::SystemReminders));
}

#[test]
fn bloat_category_labels_are_nonempty() {
    for cat in BloatCategory::all() {
        assert!(!cat.label().is_empty(), "{:?} has empty label", cat);
    }
}

#[test]
fn bloat_category_serde_roundtrip() {
    for cat in BloatCategory::all() {
        let json = serde_json::to_string(cat).expect("should serialize");
        let back: BloatCategory = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(*cat, back);
    }
}

// ---------------------------------------------------------------------------
// ContentBlock deserialization
// ---------------------------------------------------------------------------

#[test]
fn content_block_text() {
    let json = r#"{ "type": "text", "text": "Hello world" }"#;
    let block: ContentBlock = serde_json::from_str(json).expect("should parse text block");
    match block {
        ContentBlock::Text { text } => assert_eq!(text, "Hello world"),
        other => panic!("expected Text, got {:?}", other),
    }
}

#[test]
fn content_block_thinking() {
    let json = r#"{ "type": "thinking", "thinking": "Let me consider..." }"#;
    let block: ContentBlock = serde_json::from_str(json).expect("should parse thinking block");
    match block {
        ContentBlock::Thinking { thinking } => assert_eq!(thinking, "Let me consider..."),
        other => panic!("expected Thinking, got {:?}", other),
    }
}

#[test]
fn content_block_tool_use() {
    let json = r#"{
        "type": "tool_use",
        "id": "tu-001",
        "name": "Read",
        "input": { "file_path": "/tmp/test.rs" }
    }"#;
    let block: ContentBlock = serde_json::from_str(json).expect("should parse tool_use block");
    match block {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "tu-001");
            assert_eq!(name, "Read");
            assert!(input.is_object());
        }
        other => panic!("expected ToolUse, got {:?}", other),
    }
}

#[test]
fn content_block_tool_result() {
    let json = r#"{
        "type": "tool_result",
        "tool_use_id": "tu-001",
        "content": "file contents here"
    }"#;
    let block: ContentBlock = serde_json::from_str(json).expect("should parse tool_result block");
    match block {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
        } => {
            assert_eq!(tool_use_id, "tu-001");
            assert!(content.is_string());
        }
        other => panic!("expected ToolResult, got {:?}", other),
    }
}

#[test]
fn content_block_unknown_type() {
    let json = r#"{ "type": "future_block_type", "data": 42 }"#;
    let block: ContentBlock = serde_json::from_str(json).expect("unknown block should parse");
    assert!(matches!(block, ContentBlock::Unknown));
}

// ---------------------------------------------------------------------------
// DiagnosticsReport and TokenEstimate round-trip
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_report_roundtrip() {
    let report = DiagnosticsReport {
        session_id: "sess-001".to_string(),
        file_path: "/home/user/.claude/projects/test/sess.jsonl".to_string(),
        file_size_bytes: 1_048_576,
        total_entries: 830,
        message_count: 175,
        model: Some("claude-sonnet-4-5-20250514".to_string()),
        context_window: 200_000,
        system_overhead: 21_000,
        usage: Some(UsageData {
            input_tokens: 3,
            output_tokens: 13,
            cache_creation_input_tokens: 3502,
            cache_read_input_tokens: 26514,
        }),
        context_utilization_pct: Some(15.01),
        bloat: BloatBreakdown {
            entries: vec![BloatEntry {
                category: BloatCategory::ProgressTicks,
                bytes: 512_000,
                count: 640,
                percentage: 48.8,
            }],
        },
    };

    let json = serde_json::to_string_pretty(&report).expect("should serialize");
    let back: DiagnosticsReport = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(back.session_id, "sess-001");
    assert_eq!(back.total_entries, 830);
    assert_eq!(back.bloat.entries.len(), 1);
    assert_eq!(back.bloat.entries[0].category, BloatCategory::ProgressTicks);
}

#[test]
fn token_estimate_roundtrip() {
    let estimate = TokenEstimate {
        session_id: "sess-002".to_string(),
        context_tokens: 30_019,
        output_tokens: 13,
        context_window: 200_000,
        context_utilization_pct: 15.01,
        health: ContextHealth::Healthy,
    };

    let json = serde_json::to_string_pretty(&estimate).expect("should serialize");
    let back: TokenEstimate = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(back.session_id, "sess-002");
    assert_eq!(back.context_tokens, 30_019);
    assert_eq!(back.health, ContextHealth::Healthy);
}

// ---------------------------------------------------------------------------
// ContextHealth deserialization
// ---------------------------------------------------------------------------

#[test]
fn context_health_all_variants_deserialize() {
    let cases = [
        (r#""healthy""#, ContextHealth::Healthy),
        (r#""warning""#, ContextHealth::Warning),
        (r#""critical""#, ContextHealth::Critical),
    ];
    for (json, expected) in cases {
        let health: ContextHealth = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(health, expected);
    }
}

// ---------------------------------------------------------------------------
// SessionInfo round-trip
// ---------------------------------------------------------------------------

#[test]
fn session_info_roundtrip() {
    let info = SessionInfo {
        session_id: "sess-003".to_string(),
        project: Some("/home/user/project".to_string()),
        file_path: "/home/user/.claude/projects/test/sess.jsonl".to_string(),
        file_size_bytes: 2_097_152,
        entry_count: 1200,
        last_modified: Some("2026-03-06T10:00:00Z".to_string()),
        token_count: Some(45_000),
    };

    let json = serde_json::to_string(&info).expect("should serialize");
    let back: SessionInfo = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(back.session_id, "sess-003");
    assert_eq!(back.token_count, Some(45_000));
}

// ---------------------------------------------------------------------------
// ClaudeHistoryEntry
// ---------------------------------------------------------------------------

#[test]
fn claude_history_entry_deserialize() {
    let json = r#"{
        "display": "/plugin marketplace add ...",
        "project": "/Users/wollax/Git/personal/assay",
        "sessionId": "3201041c-df85-4c91-a485-7b8c189f7636",
        "timestamp": 1766983638489
    }"#;
    let entry: ClaudeHistoryEntry =
        serde_json::from_str(json).expect("should deserialize history entry");
    assert_eq!(entry.session_id, "3201041c-df85-4c91-a485-7b8c189f7636");
    assert_eq!(
        entry.project.as_deref(),
        Some("/Users/wollax/Git/personal/assay")
    );
    assert_eq!(entry.timestamp, Some(1766983638489));
}

// ---------------------------------------------------------------------------
// Sidechain entry metadata
// ---------------------------------------------------------------------------

#[test]
fn deserialize_sidechain_entry() {
    let json = r#"{
        "type": "assistant",
        "uuid": "side-001",
        "timestamp": "2026-03-06T10:01:00Z",
        "sessionId": "sess-001",
        "isSidechain": true,
        "message": null
    }"#;
    let entry: SessionEntry = serde_json::from_str(json).expect("should deserialize");
    match &entry {
        SessionEntry::Assistant(a) => {
            assert!(a.meta.is_sidechain);
        }
        other => panic!("expected Assistant, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn snapshot_diagnostics_report() {
    let report = DiagnosticsReport {
        session_id: "sess-snap".to_string(),
        file_path: "/test/session.jsonl".to_string(),
        file_size_bytes: 500_000,
        total_entries: 400,
        message_count: 100,
        model: Some("claude-sonnet-4-5".to_string()),
        context_window: 200_000,
        system_overhead: 21_000,
        usage: Some(UsageData {
            input_tokens: 10,
            output_tokens: 25,
            cache_creation_input_tokens: 5000,
            cache_read_input_tokens: 20000,
        }),
        context_utilization_pct: Some(12.505),
        bloat: BloatBreakdown {
            entries: vec![
                BloatEntry {
                    category: BloatCategory::ProgressTicks,
                    bytes: 250_000,
                    count: 300,
                    percentage: 50.0,
                },
                BloatEntry {
                    category: BloatCategory::ThinkingBlocks,
                    bytes: 50_000,
                    count: 10,
                    percentage: 10.0,
                },
            ],
        },
    };
    insta::assert_json_snapshot!("diagnostics_report", report);
}

#[test]
fn snapshot_token_estimate() {
    let estimate = TokenEstimate {
        session_id: "sess-snap".to_string(),
        context_tokens: 25_010,
        output_tokens: 25,
        context_window: 200_000,
        context_utilization_pct: 12.505,
        health: ContextHealth::Healthy,
    };
    insta::assert_json_snapshot!("token_estimate", estimate);
}
