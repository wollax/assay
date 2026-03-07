//! Team message protection for the pruning pipeline.
//!
//! Builds a set of line numbers that must be preserved during pruning
//! because they contain team coordination messages (Task*, Team*, SendMessage).

use std::collections::HashSet;

use assay_types::context::SessionEntry;

use super::super::parser::ParsedEntry;

/// Tool names that indicate team coordination messages.
///
/// Any JSONL entry containing a tool_use block with one of these names
/// is protected from pruning (the entire line is preserved).
const PROTECTED_TOOL_NAMES: &[&str] = &[
    "Task",
    "TaskCreate",
    "TaskUpdate",
    "TaskOutput",
    "TaskGet",
    "TaskList",
    "TaskStop",
    "TeamCreate",
    "TeamDelete",
    "SendMessage",
];

/// Build a set of line numbers that are protected from pruning.
///
/// Scans all entries for team coordination tool uses and returns
/// a `HashSet` of their 1-based line numbers.
pub fn build_protection_set(entries: &[ParsedEntry]) -> HashSet<usize> {
    let mut protected = HashSet::new();
    for entry in entries {
        if is_team_message(entry) {
            protected.insert(entry.line_number);
        }
    }
    protected
}

/// Check whether a parsed entry contains a team coordination message.
fn is_team_message(entry: &ParsedEntry) -> bool {
    match &entry.entry {
        SessionEntry::User(u) => check_user_blocks(&u.message),
        SessionEntry::Progress(p) => check_progress_blocks(&p.data),
        SessionEntry::Assistant(a) => check_assistant_blocks(a),
        _ => false,
    }
}

/// Check user entry message blocks for protected tool names.
fn check_user_blocks(message: &Option<serde_json::Value>) -> bool {
    let Some(msg) = message else { return false };
    let Some(blocks) = msg.as_array() else {
        return false;
    };
    blocks.iter().any(|b| {
        b.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| PROTECTED_TOOL_NAMES.contains(&n))
    })
}

/// Check progress entry nested content blocks for protected tool names.
///
/// Progress entries from subagents contain tool_use blocks at:
/// `data.message.message.content[].name`
fn check_progress_blocks(data: &Option<serde_json::Value>) -> bool {
    let Some(data) = data else { return false };
    let Some(blocks) = data
        .pointer("/message/message/content")
        .and_then(|c| c.as_array())
    else {
        return false;
    };
    blocks.iter().any(|b| {
        b.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| PROTECTED_TOOL_NAMES.contains(&n))
    })
}

/// Check assistant entry content blocks for protected tool names.
fn check_assistant_blocks(assistant: &assay_types::context::AssistantEntry) -> bool {
    let Some(msg) = &assistant.message else {
        return false;
    };
    msg.content.iter().any(|block| {
        matches!(block, assay_types::context::ContentBlock::ToolUse { name, .. }
            if PROTECTED_TOOL_NAMES.contains(&name.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{
        AssistantEntry, AssistantMessage, ContentBlock, EntryMetadata, ProgressEntry, UserEntry,
    };

    fn make_meta() -> EntryMetadata {
        EntryMetadata {
            uuid: "test-uuid".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: "s1".into(),
            parent_uuid: None,
            is_sidechain: false,
            cwd: None,
            version: None,
        }
    }

    fn make_parsed(entry: SessionEntry, line: usize) -> ParsedEntry {
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes: 100,
            raw_line: String::new(),
        }
    }

    #[test]
    fn user_entry_with_task_create_is_protected() {
        let entry = make_parsed(
            SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!([
                    {
                        "type": "tool_use",
                        "id": "tu1",
                        "name": "TaskCreate",
                        "input": { "subject": "Do something" }
                    }
                ])),
            }),
            1,
        );
        assert!(is_team_message(&entry));
    }

    #[test]
    fn user_entry_with_send_message_is_protected() {
        let entry = make_parsed(
            SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!([
                    {
                        "type": "tool_use",
                        "id": "tu1",
                        "name": "SendMessage",
                        "input": { "recipient": "agent-1", "message": "hello" }
                    }
                ])),
            }),
            2,
        );
        assert!(is_team_message(&entry));
    }

    #[test]
    fn progress_entry_with_send_message_is_protected() {
        let entry = make_parsed(
            SessionEntry::Progress(ProgressEntry {
                meta: make_meta(),
                data: Some(serde_json::json!({
                    "agentId": "agent-abc",
                    "message": {
                        "message": {
                            "content": [
                                {
                                    "type": "tool_use",
                                    "id": "tu1",
                                    "name": "SendMessage",
                                    "input": { "recipient": "primary", "message": "done" }
                                }
                            ]
                        }
                    }
                })),
            }),
            3,
        );
        assert!(is_team_message(&entry));
    }

    #[test]
    fn progress_entry_with_task_update_is_protected() {
        let entry = make_parsed(
            SessionEntry::Progress(ProgressEntry {
                meta: make_meta(),
                data: Some(serde_json::json!({
                    "agentId": "agent-abc",
                    "message": {
                        "message": {
                            "content": [
                                {
                                    "type": "tool_use",
                                    "id": "tu1",
                                    "name": "TaskUpdate",
                                    "input": { "taskId": "1", "status": "completed" }
                                }
                            ]
                        }
                    }
                })),
            }),
            4,
        );
        assert!(is_team_message(&entry));
    }

    #[test]
    fn assistant_entry_with_team_create_is_protected() {
        let entry = make_parsed(
            SessionEntry::Assistant(AssistantEntry {
                meta: make_meta(),
                message: Some(AssistantMessage {
                    model: None,
                    content: vec![ContentBlock::ToolUse {
                        id: "tu1".into(),
                        name: "TeamCreate".into(),
                        input: serde_json::json!({}),
                    }],
                    usage: None,
                    stop_reason: None,
                }),
            }),
            5,
        );
        assert!(is_team_message(&entry));
    }

    #[test]
    fn regular_user_message_is_not_protected() {
        let entry = make_parsed(
            SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!("Hello, how are you?")),
            }),
            10,
        );
        assert!(!is_team_message(&entry));
    }

    #[test]
    fn user_entry_with_read_tool_is_not_protected() {
        let entry = make_parsed(
            SessionEntry::User(UserEntry {
                meta: make_meta(),
                message: Some(serde_json::json!([
                    {
                        "type": "tool_use",
                        "id": "tu1",
                        "name": "Read",
                        "input": { "file_path": "/src/main.rs" }
                    }
                ])),
            }),
            11,
        );
        assert!(!is_team_message(&entry));
    }

    #[test]
    fn progress_tick_without_tool_use_is_not_protected() {
        let entry = make_parsed(
            SessionEntry::Progress(ProgressEntry {
                meta: make_meta(),
                data: Some(serde_json::json!({
                    "type": "bash_progress",
                    "command": "cargo build"
                })),
            }),
            12,
        );
        assert!(!is_team_message(&entry));
    }

    #[test]
    fn build_protection_set_returns_correct_line_numbers() {
        let entries = vec![
            make_parsed(
                SessionEntry::User(UserEntry {
                    meta: make_meta(),
                    message: Some(serde_json::json!([
                        { "type": "tool_use", "id": "tu1", "name": "TaskCreate", "input": {} }
                    ])),
                }),
                1,
            ),
            make_parsed(
                SessionEntry::User(UserEntry {
                    meta: make_meta(),
                    message: Some(serde_json::json!("regular message")),
                }),
                2,
            ),
            make_parsed(
                SessionEntry::Progress(ProgressEntry {
                    meta: make_meta(),
                    data: Some(serde_json::json!({
                        "agentId": "a1",
                        "message": { "message": { "content": [
                            { "type": "tool_use", "id": "tu2", "name": "SendMessage", "input": {} }
                        ]}}
                    })),
                }),
                3,
            ),
        ];

        let protected = build_protection_set(&entries);
        assert_eq!(protected.len(), 2);
        assert!(protected.contains(&1));
        assert!(!protected.contains(&2));
        assert!(protected.contains(&3));
    }

    #[test]
    fn empty_entries_yields_empty_protection_set() {
        let protected = build_protection_set(&[]);
        assert!(protected.is_empty());
    }
}
