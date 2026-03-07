//! System-reminder-dedup strategy: keeps only the last occurrence of each
//! repeated system reminder.
//!
//! System reminders (`<system-reminder>` tags) are injected into message content
//! repeatedly throughout a session. This strategy removes duplicate entries,
//! keeping only the last occurrence of each unique reminder.

use std::collections::{HashMap, HashSet};

use regex_lite::Regex;

use assay_types::context::{ContentBlock, PrescriptionTier, PruneSample, SessionEntry};

use super::super::super::parser::ParsedEntry;
use super::super::strategy::StrategyResult;

/// Extract system reminder text from an entry, if present.
///
/// Checks both assistant content blocks (typed `ContentBlock::Text`) and
/// user message array blocks (untyped JSON with `"type": "text"`).
/// Returns the full text block content as the dedup key.
fn extract_reminder_text(entry: &ParsedEntry, re: &Regex) -> Option<String> {
    match &entry.entry {
        SessionEntry::Assistant(a) => {
            let msg = a.message.as_ref()?;
            for block in &msg.content {
                if let ContentBlock::Text { text } = block
                    && re.is_match(text)
                {
                    return Some(text.clone());
                }
            }
            None
        }
        SessionEntry::User(u) => {
            let msg = u.message.as_ref()?;
            let blocks = msg.as_array()?;
            for block in blocks {
                if block.get("type").and_then(|t| t.as_str()) == Some("text")
                    && let Some(text) = block.get("text").and_then(|t| t.as_str())
                    && re.is_match(text)
                {
                    return Some(text.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

/// Remove entries containing duplicate system reminders, keeping only the last
/// occurrence of each unique reminder.
///
/// Uses a two-pass approach:
/// 1. Reverse scan to identify the last occurrence of each unique reminder
/// 2. Forward scan to remove earlier duplicates
///
/// Protected entries are preserved and counted in `protected_skipped`.
pub fn system_reminder_dedup(
    entries: Vec<ParsedEntry>,
    _tier: PrescriptionTier,
    protected: &HashSet<usize>,
) -> StrategyResult {
    let re = Regex::new(r"<system-reminder>").expect("valid regex");

    // Pass 1: reverse scan to find the last occurrence of each unique reminder.
    let mut last_occurrence: HashMap<String, usize> = HashMap::new();
    for entry in entries.iter().rev() {
        if let Some(text) = extract_reminder_text(entry, &re) {
            last_occurrence.entry(text).or_insert(entry.line_number);
        }
    }

    // Build the set of line numbers that are the "last" for their reminder.
    let keep_lines: HashSet<usize> = last_occurrence.values().copied().collect();

    // Pass 2: forward scan, removing duplicates.
    let mut result_entries = Vec::with_capacity(entries.len());
    let mut lines_removed: usize = 0;
    let mut bytes_saved: u64 = 0;
    let mut protected_skipped: usize = 0;
    let mut samples: Vec<PruneSample> = Vec::new();

    for entry in entries {
        let reminder_text = extract_reminder_text(&entry, &re);

        if reminder_text.is_some() && !keep_lines.contains(&entry.line_number) {
            // This is a duplicate (not the last occurrence).
            if protected.contains(&entry.line_number) {
                protected_skipped += 1;
                result_entries.push(entry);
            } else {
                let entry_bytes = entry.raw_bytes as u64;
                if samples.len() < 3 {
                    samples.push(PruneSample {
                        line_number: entry.line_number,
                        description: "Duplicate system reminder".into(),
                        bytes: entry_bytes,
                    });
                }
                bytes_saved += entry_bytes;
                lines_removed += 1;
            }
        } else {
            result_entries.push(entry);
        }
    }

    StrategyResult {
        entries: result_entries,
        lines_removed,
        lines_modified: 0,
        bytes_saved,
        protected_skipped,
        samples,
    }
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

    fn make_user_plain(line: usize) -> ParsedEntry {
        let entry = SessionEntry::User(UserEntry {
            meta: make_meta(),
            message: Some(serde_json::json!("just a normal message")),
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

    fn make_assistant_with_reminder(line: usize, reminder_text: &str) -> ParsedEntry {
        let text = format!("Some text\n<system-reminder>\n{reminder_text}\n</system-reminder>");
        let entry = SessionEntry::Assistant(AssistantEntry {
            meta: make_meta(),
            message: Some(AssistantMessage {
                model: None,
                content: vec![ContentBlock::Text { text }],
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

    fn make_user_with_reminder(line: usize, reminder_text: &str) -> ParsedEntry {
        let text = format!("Content\n<system-reminder>\n{reminder_text}\n</system-reminder>");
        let entry = SessionEntry::User(UserEntry {
            meta: make_meta(),
            message: Some(serde_json::json!([
                {
                    "type": "text",
                    "text": text
                }
            ])),
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

    fn make_assistant_no_reminder(line: usize) -> ParsedEntry {
        let entry = SessionEntry::Assistant(AssistantEntry {
            meta: make_meta(),
            message: Some(AssistantMessage {
                model: None,
                content: vec![ContentBlock::Text {
                    text: "just regular text".into(),
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

    #[test]
    fn no_system_reminders_all_unchanged() {
        let entries = vec![
            make_user_plain(1),
            make_assistant_no_reminder(2),
            make_user_plain(3),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.lines_removed, 0);
        assert_eq!(result.bytes_saved, 0);
    }

    #[test]
    fn single_reminder_kept() {
        let entries = vec![
            make_user_plain(1),
            make_assistant_with_reminder(2, "Remember this"),
            make_user_plain(3),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.lines_removed, 0);
    }

    #[test]
    fn two_identical_reminders_first_removed() {
        let entries = vec![
            make_assistant_with_reminder(1, "Remember this"),
            make_user_plain(2),
            make_assistant_with_reminder(3, "Remember this"),
        ];
        let original_bytes = entries[0].raw_bytes;
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        // Line 1 should be removed (first occurrence), line 3 kept (last)
        assert_eq!(result.entries[0].line_number, 2); // plain user
        assert_eq!(result.entries[1].line_number, 3); // last reminder
        assert_eq!(result.lines_removed, 1);
        assert_eq!(result.bytes_saved, original_bytes as u64);
    }

    #[test]
    fn three_identical_reminders_first_two_removed() {
        let entries = vec![
            make_assistant_with_reminder(1, "Same reminder"),
            make_assistant_with_reminder(2, "Same reminder"),
            make_assistant_with_reminder(3, "Same reminder"),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].line_number, 3);
        assert_eq!(result.lines_removed, 2);
    }

    #[test]
    fn different_reminders_each_keeps_last() {
        let entries = vec![
            make_assistant_with_reminder(1, "Reminder A"),
            make_assistant_with_reminder(2, "Reminder B"),
            make_assistant_with_reminder(3, "Reminder A"),
            make_assistant_with_reminder(4, "Reminder B"),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].line_number, 3); // last A
        assert_eq!(result.entries[1].line_number, 4); // last B
        assert_eq!(result.lines_removed, 2);
    }

    #[test]
    fn reminder_in_assistant_text_detected() {
        let entries = vec![
            make_assistant_with_reminder(1, "Check this"),
            make_user_plain(2),
            make_assistant_with_reminder(3, "Check this"),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.lines_removed, 1);
    }

    #[test]
    fn reminder_in_user_text_block_detected() {
        let entries = vec![
            make_user_with_reminder(1, "User reminder"),
            make_user_plain(2),
            make_user_with_reminder(3, "User reminder"),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].line_number, 2); // plain
        assert_eq!(result.entries[1].line_number, 3); // last reminder
        assert_eq!(result.lines_removed, 1);
    }

    #[test]
    fn protected_duplicate_kept() {
        let mut protected = HashSet::new();
        protected.insert(1);
        let entries = vec![
            make_assistant_with_reminder(1, "Protected"),
            make_user_plain(2),
            make_assistant_with_reminder(3, "Protected"),
        ];
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &protected);
        // Line 1 is protected so kept even though it's a duplicate
        assert_eq!(result.entries.len(), 3);
        assert_eq!(result.protected_skipped, 1);
        assert_eq!(result.lines_removed, 0);
    }

    #[test]
    fn bytes_saved_reflects_removal() {
        let entries = vec![
            make_assistant_with_reminder(1, "Dedup me"),
            make_assistant_with_reminder(2, "Dedup me"),
        ];
        let first_bytes = entries[0].raw_bytes as u64;
        let result = system_reminder_dedup(entries, PrescriptionTier::Gentle, &HashSet::new());
        assert_eq!(result.bytes_saved, first_bytes);
        assert_eq!(result.lines_removed, 1);
    }
}
