//! Claude Code session parsing, discovery, and token diagnostics.
//!
//! Provides the domain logic for reading session JSONL files, discovering
//! sessions by project, extracting token usage, and producing diagnostics
//! reports.

use std::path::Path;

use assay_types::context::SessionInfo;

mod diagnostics;
mod discovery;
mod parser;
mod tokens;

pub use diagnostics::diagnose;
pub use discovery::{discover_sessions, find_session_dir, resolve_session};
pub use parser::{ParsedEntry, parse_session};
pub use tokens::{estimate_tokens, extract_usage, quick_token_estimate};

/// List session files for a project (or all projects), optionally including token counts.
///
/// When `project_dir` is `Some`, only sessions for that project are returned.
/// When `None`, all sessions across all projects are discovered.
/// Results are sorted by modification time (newest first) and limited to `limit`.
pub fn list_sessions(
    project_dir: Option<&Path>,
    limit: usize,
    include_tokens: bool,
) -> crate::Result<Vec<SessionInfo>> {
    let session_dir = match project_dir {
        Some(dir) => find_session_dir(dir)?,
        None => discovery::claude_projects_dir().ok_or_else(|| {
            crate::AssayError::SessionDirNotFound {
                message: "home directory not found".into(),
            }
        })?,
    };

    let paths = discover_sessions(&session_dir)?;
    let mut sessions = Vec::new();

    for path in paths.into_iter().take(limit) {
        let file_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let metadata = std::fs::metadata(&path).ok();
        let file_size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let last_modified = metadata.as_ref().and_then(|m| m.modified().ok()).map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        });

        let entry_count = quick_line_count(&path);

        let token_count = if include_tokens {
            quick_token_estimate(&path)
                .ok()
                .flatten()
                .map(|u| u.context_tokens())
        } else {
            None
        };

        let project = project_dir.map(|p| p.to_string_lossy().to_string());

        sessions.push(SessionInfo {
            session_id: file_name,
            project,
            file_path: path.to_string_lossy().to_string(),
            file_size_bytes,
            entry_count,
            last_modified,
            token_count,
        });
    }

    Ok(sessions)
}

/// Count non-empty lines in a file (cheap entry count estimate).
fn quick_line_count(path: &Path) -> u64 {
    use std::io::BufRead;
    let Ok(file) = std::fs::File::open(path) else {
        return 0;
    };
    let reader = std::io::BufReader::new(file);
    reader
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .count() as u64
}
