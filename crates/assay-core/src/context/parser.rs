//! Line-by-line JSONL parser with per-line error tolerance.

use std::fs::File;
use std::io::BufRead;
use std::path::Path;

use assay_types::context::SessionEntry;

/// A parsed session entry with its line number and raw byte size.
#[derive(Debug)]
pub struct ParsedEntry {
    /// The deserialized session entry.
    pub entry: SessionEntry,
    /// 1-based line number in the source file.
    pub line_number: usize,
    /// Raw byte length of the source line.
    pub raw_bytes: usize,
    /// The original JSON line, preserved for lossless write-back.
    pub raw_line: String,
}

impl ParsedEntry {
    /// Re-serialize the entry after modification, updating raw_line and raw_bytes.
    pub fn update_content(&mut self, new_entry: SessionEntry) {
        self.raw_line = serde_json::to_string(&new_entry).unwrap_or_default();
        self.raw_bytes = self.raw_line.len();
        self.entry = new_entry;
    }
}

/// Parse a session JSONL file, tolerating per-line errors.
///
/// Returns successfully parsed entries and a count of skipped (unparseable) lines.
pub fn parse_session(path: &Path) -> crate::Result<(Vec<ParsedEntry>, usize)> {
    let file = File::open(path).map_err(|source| crate::AssayError::Io {
        operation: "opening session file".into(),
        path: path.to_path_buf(),
        source,
    })?;
    let reader = std::io::BufReader::new(file);
    let mut entries = Vec::new();
    let mut skipped = 0;

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|source| crate::AssayError::Io {
            operation: "reading session file".into(),
            path: path.to_path_buf(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEntry>(&line) {
            Ok(entry) => entries.push(ParsedEntry {
                entry,
                line_number: line_num + 1,
                raw_bytes: line.len(),
                raw_line: line,
            }),
            Err(_) => skipped += 1,
        }
    }
    Ok((entries, skipped))
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

    #[test]
    fn parse_user_and_assistant_entries() {
        let f = write_jsonl(&[
            r#"{"type":"user","uuid":"u1","timestamp":"2026-01-01T00:00:00Z","sessionId":"s1"}"#,
            r#"{"type":"assistant","uuid":"a1","timestamp":"2026-01-01T00:01:00Z","sessionId":"s1"}"#,
        ]);
        let (entries, skipped) = parse_session(f.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(skipped, 0);
        assert!(matches!(entries[0].entry, SessionEntry::User(_)));
        assert!(matches!(entries[1].entry, SessionEntry::Assistant(_)));
        assert_eq!(entries[0].line_number, 1);
        assert_eq!(entries[1].line_number, 2);
    }

    #[test]
    fn skips_empty_and_malformed_lines() {
        let f = write_jsonl(&[
            r#"{"type":"user","uuid":"u1","timestamp":"2026-01-01T00:00:00Z","sessionId":"s1"}"#,
            "",
            "not json at all",
            r#"{"type":"assistant","uuid":"a1","timestamp":"2026-01-01T00:01:00Z","sessionId":"s1"}"#,
        ]);
        let (entries, skipped) = parse_session(f.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(skipped, 1); // "not json at all" is skipped; empty line is ignored
    }

    #[test]
    fn unknown_entry_types_parsed_as_unknown() {
        let f = write_jsonl(&[
            r#"{"type":"some-future-type","uuid":"x1","timestamp":"2026-01-01T00:00:00Z","sessionId":"s1"}"#,
        ]);
        let (entries, skipped) = parse_session(f.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(skipped, 0);
        assert!(matches!(entries[0].entry, SessionEntry::Unknown));
    }

    #[test]
    fn file_not_found_returns_error() {
        let result = parse_session(Path::new("/nonexistent/session.jsonl"));
        assert!(result.is_err());
    }
}
