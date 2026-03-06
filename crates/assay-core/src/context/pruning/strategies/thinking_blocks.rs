//! Thinking-blocks strategy: removes Thinking content blocks from assistant entries.
//!
//! Extended thinking blocks are ephemeral and not counted in the context window.
//! Removing them reduces session file size without losing conversational content.

use std::collections::HashSet;

use assay_types::context::{PrescriptionTier, PruneSample};

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Remove all Thinking content blocks from unprotected assistant entries.
///
/// For each assistant entry with thinking blocks:
/// - Filters out `ContentBlock::Thinking` variants
/// - Re-serializes the entry via `ParsedEntry::update_content()`
/// - Tracks bytes saved as the difference in raw_bytes
///
/// Protected entries are preserved unchanged and counted in `protected_skipped`.
pub fn thinking_blocks(
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    protected: &HashSet<usize>,
) -> StrategyResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{
        AssistantEntry, AssistantMessage, ContentBlock, EntryMetadata, SessionEntry, UserEntry,
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

    fn make_assistant_with_thinking(line: usize) -> ParsedEntry {
        let entry = SessionEntry::Assistant(AssistantEntry {
            meta: make_meta(),
            message: Some(AssistantMessage {
                model: None,
                content: vec![
                    ContentBlock::Thinking {
                        thinking: "deep internal reasoning here".into(),
                    },
                    ContentBlock::Text {
                        text: "visible response".into(),
                    },
                ],
                usage: None,
                stop_reason: None,
            }),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    fn make_assistant_thinking_only(line: usize) -> ParsedEntry {
        let entry = SessionEntry::Assistant(AssistantEntry {
            meta: make_meta(),
            message: Some(AssistantMessage {
                model: None,
                content: vec![ContentBlock::Thinking {
                    thinking: "only thinking here".into(),
                }],
                usage: None,
                stop_reason: None,
            }),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    fn make_assistant_text_only(line: usize) -> ParsedEntry {
        let entry = SessionEntry::Assistant(AssistantEntry {
            meta: make_meta(),
            message: Some(AssistantMessage {
                model: None,
                content: vec![ContentBlock::Text {
                    text: "just text".into(),
                }],
                usage: None,
                stop_reason: None,
            }),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    fn make_user(line: usize) -> ParsedEntry {
        let entry = SessionEntry::User(UserEntry {
            meta: make_meta(),
            message: Some(serde_json::json!("hello")),
        });
        let raw_line = serde_json::to_string(&entry).unwrap();
        let raw_bytes = raw_line.len();
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes,
            raw_line,
        }
    }

    #[test]
    fn thinking_block_removed_from_assistant() {
        let entries = vec![make_assistant_thinking_only(1)];
        let original_bytes = entries[0].raw_bytes;
        let result = thinking_blocks(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert!(result.entries[0].raw_bytes < original_bytes);
        assert_eq!(result.lines_modified, 1);
        assert!(result.bytes_saved > 0);
        // Verify no thinking blocks remain
        if let SessionEntry::Assistant(a) = &result.entries[0].entry {
            let msg = a.message.as_ref().unwrap();
            assert!(msg
                .content
                .iter()
                .all(|b| !matches!(b, ContentBlock::Thinking { .. })));
        } else {
            panic!("expected assistant entry");
        }
    }

    #[test]
    fn text_preserved_when_thinking_removed() {
        let entries = vec![make_assistant_with_thinking(1)];
        let result = thinking_blocks(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        if let SessionEntry::Assistant(a) = &result.entries[0].entry {
            let msg = a.message.as_ref().unwrap();
            assert_eq!(msg.content.len(), 1);
            assert!(matches!(&msg.content[0], ContentBlock::Text { text } if text == "visible response"));
        } else {
            panic!("expected assistant entry");
        }
    }

    #[test]
    fn assistant_without_thinking_unchanged() {
        let entries = vec![make_assistant_text_only(1)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = thinking_blocks(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.lines_modified, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn non_assistant_entries_unchanged() {
        let entries = vec![make_user(1)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = thinking_blocks(entries, PrescriptionTier::Aggressive, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.lines_modified, 0);
    }

    #[test]
    fn protected_assistant_with_thinking_not_modified() {
        let mut protected = HashSet::new();
        protected.insert(1);
        let entries = vec![make_assistant_with_thinking(1)];
        let original_raw_line = entries[0].raw_line.clone();
        let result = thinking_blocks(entries, PrescriptionTier::Aggressive, &protected);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].raw_line, original_raw_line);
        assert_eq!(result.protected_skipped, 1);
        assert_eq!(result.lines_modified, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn bytes_saved_is_accurate() {
        let entries = vec![
            make_assistant_with_thinking(1),
            make_assistant_with_thinking(2),
        ];
        let total_original: usize = entries.iter().map(|e| e.raw_bytes).sum();
        let result = thinking_blocks(entries, PrescriptionTier::Aggressive, &HashSet::new());
        let total_new: usize = result.entries.iter().map(|e| e.raw_bytes).sum();
        assert_eq!(result.bytes_saved, (total_original - total_new) as u64);
        assert_eq!(result.lines_modified, 2);
    }
}
