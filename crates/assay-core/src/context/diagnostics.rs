//! Bloat categorization, stale read detection, and DiagnosticsReport assembly.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use regex_lite::Regex;

use assay_types::context::{
    BloatBreakdown, BloatCategory, BloatEntry, ContentBlock, DiagnosticsReport, SessionEntry,
};

use super::parser::{ParsedEntry, parse_session};
use super::tokens::{
    SYSTEM_OVERHEAD_TOKENS, context_window_for_model, extract_model, extract_usage, is_sidechain,
};

/// Analyze a parsed session and produce a `DiagnosticsReport`.
pub fn diagnose(path: &Path, session_id: &str) -> crate::Result<DiagnosticsReport> {
    let (entries, _skipped) = parse_session(path)?;
    let file_size = std::fs::metadata(path)
        .map_err(|source| crate::AssayError::Io {
            operation: "reading session file metadata".into(),
            path: path.to_path_buf(),
            source,
        })?
        .len();

    let usage = extract_usage(&entries);
    let model = extract_model(&entries);
    let context_window = context_window_for_model(model.as_deref());
    let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
    let context_pct = usage
        .as_ref()
        .map(|u| (u.context_tokens() as f64 / available as f64) * 100.0);

    let bloat = categorize_bloat(&entries, file_size);

    let message_count = entries
        .iter()
        .filter(|e| matches!(&e.entry, SessionEntry::User(_) | SessionEntry::Assistant(_)))
        .filter(|e| !is_sidechain(&e.entry))
        .count() as u64;

    Ok(DiagnosticsReport {
        session_id: session_id.to_string(),
        file_path: path.to_string_lossy().to_string(),
        file_size_bytes: file_size,
        total_entries: entries.len() as u64,
        message_count,
        model,
        context_window,
        system_overhead: SYSTEM_OVERHEAD_TOKENS,
        usage,
        context_utilization_pct: context_pct,
        bloat,
    })
}

/// Categorize bloat across all 6 categories.
fn categorize_bloat(entries: &[ParsedEntry], file_size: u64) -> BloatBreakdown {
    let system_reminder_re = Regex::new(r"<system-reminder>").expect("valid regex");

    let mut counts: HashMap<BloatCategory, (u64, u64)> = HashMap::new(); // (bytes, count)
    let mut read_paths: HashSet<String> = HashSet::new();

    for parsed in entries {
        let bytes = parsed.raw_bytes as u64;

        match &parsed.entry {
            // Progress entries -> ProgressTicks
            SessionEntry::Progress(_) => {
                let e = counts.entry(BloatCategory::ProgressTicks).or_default();
                e.0 += bytes;
                e.1 += 1;
            }

            // Metadata entries (file-history-snapshot, queue-operation, pr-link)
            SessionEntry::FileHistorySnapshot(_)
            | SessionEntry::QueueOperation(_)
            | SessionEntry::PrLink(_) => {
                let e = counts.entry(BloatCategory::Metadata).or_default();
                e.0 += bytes;
                e.1 += 1;
            }

            // System entries -> Metadata
            SessionEntry::System(_) => {
                let e = counts.entry(BloatCategory::Metadata).or_default();
                e.0 += bytes;
                e.1 += 1;
            }

            // Assistant entries: check for thinking blocks and system reminders
            SessionEntry::Assistant(a) => {
                if let Some(msg) = &a.message {
                    for block in &msg.content {
                        match block {
                            ContentBlock::Thinking { thinking } => {
                                let e = counts.entry(BloatCategory::ThinkingBlocks).or_default();
                                e.0 += thinking.len() as u64;
                                e.1 += 1;
                            }
                            ContentBlock::Text { text } => {
                                if system_reminder_re.is_match(text) {
                                    let e =
                                        counts.entry(BloatCategory::SystemReminders).or_default();
                                    e.0 += text.len() as u64;
                                    e.1 += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // User entries: check for tool results, stale reads, and system reminders
            SessionEntry::User(u) => {
                if let Some(msg) = &u.message {
                    // Check if message is an array of content blocks (tool results)
                    if let Some(blocks) = msg.as_array() {
                        for block in blocks {
                            let block_type = block.get("type").and_then(|t| t.as_str());

                            // Tool use blocks: check for Read tool (stale read detection)
                            if block_type == Some("tool_use") {
                                let tool_name =
                                    block.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                if (tool_name == "Read" || tool_name == "read")
                                    && let Some(input) = block.get("input")
                                    && let Some(file_path) =
                                        input.get("file_path").and_then(|p| p.as_str())
                                    && !read_paths.insert(file_path.to_string())
                                {
                                    // Already read this file -> stale read
                                    let e = counts.entry(BloatCategory::StaleReads).or_default();
                                    e.0 += block.to_string().len() as u64;
                                    e.1 += 1;
                                }
                            }

                            // Tool result blocks -> ToolOutput
                            if block_type == Some("tool_result") {
                                let content_size = block
                                    .get("content")
                                    .map(|c| c.to_string().len() as u64)
                                    .unwrap_or(0);
                                let e = counts.entry(BloatCategory::ToolOutput).or_default();
                                e.0 += content_size;
                                e.1 += 1;
                            }

                            // Check text content for system reminders
                            if block_type == Some("text")
                                && let Some(text) = block.get("text").and_then(|t| t.as_str())
                                && system_reminder_re.is_match(text)
                            {
                                let e = counts.entry(BloatCategory::SystemReminders).or_default();
                                e.0 += text.len() as u64;
                                e.1 += 1;
                            }
                        }
                    }
                }
            }

            // Unknown entries are ignored for bloat purposes
            SessionEntry::Unknown => {}
        }
    }

    // Build entries for all 6 categories (including zero-count ones)
    let bloat_entries: Vec<BloatEntry> = BloatCategory::all()
        .iter()
        .map(|cat| {
            let (cat_bytes, count) = counts.get(cat).copied().unwrap_or((0, 0));
            let percentage = if file_size > 0 {
                (cat_bytes as f64 / file_size as f64) * 100.0
            } else {
                0.0
            };
            BloatEntry {
                category: *cat,
                bytes: cat_bytes,
                count,
                percentage,
            }
        })
        .collect();

    BloatBreakdown {
        entries: bloat_entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_jsonl(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    fn meta_json(uuid: &str, session_id: &str) -> String {
        format!(r#""uuid":"{uuid}","timestamp":"2026-01-01T00:00:00Z","sessionId":"{session_id}""#)
    }

    #[test]
    fn diagnose_produces_complete_report() {
        let meta = meta_json("u1", "s1");
        let f = write_jsonl(&[
            &format!(r#"{{"type":"user",{meta}}}"#),
            &format!(
                r#"{{"type":"assistant",{meta},"message":{{"model":"claude-sonnet-4-5-20250514","content":[{{"type":"text","text":"hello"}}],"usage":{{"input_tokens":1000,"output_tokens":200,"cache_creation_input_tokens":500,"cache_read_input_tokens":300}}}}}}"#
            ),
        ]);

        let report = diagnose(f.path(), "s1").unwrap();
        assert_eq!(report.session_id, "s1");
        assert_eq!(report.total_entries, 2);
        assert_eq!(report.message_count, 2);
        assert_eq!(report.model.as_deref(), Some("claude-sonnet-4-5-20250514"));
        assert_eq!(report.context_window, 200_000);
        assert!(report.usage.is_some());
        let usage = report.usage.unwrap();
        assert_eq!(usage.context_tokens(), 1800); // 1000 + 500 + 300
        assert!(report.context_utilization_pct.is_some());
        // 6 bloat categories always present
        assert_eq!(report.bloat.entries.len(), 6);
    }

    #[test]
    fn categorize_bloat_detects_progress_ticks() {
        let meta = meta_json("p1", "s1");
        let f = write_jsonl(&[
            &format!(r#"{{"type":"progress",{meta}}}"#),
            &format!(r#"{{"type":"progress",{meta}}}"#),
        ]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let file_size = std::fs::metadata(f.path()).unwrap().len();
        let bloat = categorize_bloat(&entries, file_size);

        let ticks = bloat
            .entries
            .iter()
            .find(|e| e.category == BloatCategory::ProgressTicks)
            .unwrap();
        assert_eq!(ticks.count, 2);
        assert!(ticks.bytes > 0);
    }

    #[test]
    fn categorize_bloat_detects_thinking_blocks() {
        let meta = meta_json("a1", "s1");
        let f = write_jsonl(&[&format!(
            r#"{{"type":"assistant",{meta},"message":{{"content":[{{"type":"thinking","thinking":"deep thoughts here"}}]}}}}"#
        )]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let file_size = std::fs::metadata(f.path()).unwrap().len();
        let bloat = categorize_bloat(&entries, file_size);

        let thinking = bloat
            .entries
            .iter()
            .find(|e| e.category == BloatCategory::ThinkingBlocks)
            .unwrap();
        assert_eq!(thinking.count, 1);
        assert!(thinking.bytes > 0);
    }

    #[test]
    fn categorize_bloat_detects_metadata_entries() {
        let meta = meta_json("m1", "s1");
        let f = write_jsonl(&[
            &format!(r#"{{"type":"file-history-snapshot",{meta}}}"#),
            &format!(r#"{{"type":"queue-operation",{meta}}}"#),
            &format!(r#"{{"type":"system",{meta}}}"#),
        ]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let file_size = std::fs::metadata(f.path()).unwrap().len();
        let bloat = categorize_bloat(&entries, file_size);

        let metadata = bloat
            .entries
            .iter()
            .find(|e| e.category == BloatCategory::Metadata)
            .unwrap();
        assert_eq!(metadata.count, 3);
    }

    #[test]
    fn categorize_bloat_detects_system_reminders() {
        let meta = meta_json("a1", "s1");
        let f = write_jsonl(&[&format!(
            r#"{{"type":"assistant",{meta},"message":{{"content":[{{"type":"text","text":"Here is some <system-reminder> injected content"}}]}}}}"#
        )]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let file_size = std::fs::metadata(f.path()).unwrap().len();
        let bloat = categorize_bloat(&entries, file_size);

        let reminders = bloat
            .entries
            .iter()
            .find(|e| e.category == BloatCategory::SystemReminders)
            .unwrap();
        assert_eq!(reminders.count, 1);
        assert!(reminders.bytes > 0);
    }

    #[test]
    fn categorize_bloat_detects_tool_output() {
        let meta = meta_json("u1", "s1");
        let f = write_jsonl(&[&format!(
            r#"{{"type":"user",{meta},"message":[{{"type":"tool_result","tool_use_id":"t1","content":"lots of output here"}}]}}"#
        )]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let file_size = std::fs::metadata(f.path()).unwrap().len();
        let bloat = categorize_bloat(&entries, file_size);

        let tool_output = bloat
            .entries
            .iter()
            .find(|e| e.category == BloatCategory::ToolOutput)
            .unwrap();
        assert_eq!(tool_output.count, 1);
        assert!(tool_output.bytes > 0);
    }

    #[test]
    fn categorize_bloat_detects_stale_reads() {
        let meta = meta_json("u1", "s1");
        let f = write_jsonl(&[
            // First read of a file
            &format!(
                r#"{{"type":"user",{meta},"message":[{{"type":"tool_use","id":"t1","name":"Read","input":{{"file_path":"/src/main.rs"}}}}]}}"#
            ),
            // Second read of the same file (stale)
            &format!(
                r#"{{"type":"user",{meta},"message":[{{"type":"tool_use","id":"t2","name":"Read","input":{{"file_path":"/src/main.rs"}}}}]}}"#
            ),
        ]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let file_size = std::fs::metadata(f.path()).unwrap().len();
        let bloat = categorize_bloat(&entries, file_size);

        let stale = bloat
            .entries
            .iter()
            .find(|e| e.category == BloatCategory::StaleReads)
            .unwrap();
        assert_eq!(stale.count, 1); // Only the second read is stale
        assert!(stale.bytes > 0);
    }

    #[test]
    fn all_six_categories_always_present() {
        let f = write_jsonl(&[]);
        let (entries, _) = parse_session(f.path()).unwrap();
        let bloat = categorize_bloat(&entries, 0);
        assert_eq!(bloat.entries.len(), 6);
        for entry in &bloat.entries {
            assert_eq!(entry.count, 0);
            assert_eq!(entry.bytes, 0);
        }
    }
}
