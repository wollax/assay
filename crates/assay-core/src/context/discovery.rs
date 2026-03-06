//! Session file discovery via project slug and history.jsonl.

use std::path::{Path, PathBuf};

use assay_types::context::ClaudeHistoryEntry;

use crate::AssayError;

/// Convert an absolute path to Claude Code's project slug format.
///
/// Claude Code uses the absolute path with `/` replaced by `-` as the
/// directory name under `~/.claude/projects/`.
pub fn path_to_project_slug(path: &Path) -> String {
    let s = path.to_string_lossy();
    // Claude strips the leading slash, then replaces remaining slashes with hyphens
    let stripped = s.strip_prefix('/').unwrap_or(&s);
    stripped.replace('/', "-")
}

/// Find the Claude Code projects directory (`~/.claude/projects/`).
pub fn claude_projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("projects"))
}

/// Find the session directory for a specific project.
///
/// Maps the project's absolute path to a slug and looks for the corresponding
/// directory under `~/.claude/projects/`.
pub fn find_session_dir(project_path: &Path) -> crate::Result<PathBuf> {
    let projects_dir = claude_projects_dir().ok_or_else(|| AssayError::SessionDirNotFound {
        message: "home directory not found".into(),
    })?;
    let slug = path_to_project_slug(project_path);
    let session_dir = projects_dir.join(&slug);
    if session_dir.is_dir() {
        Ok(session_dir)
    } else {
        Err(AssayError::SessionDirNotFound {
            message: format!("no session directory for project slug '{slug}'"),
        })
    }
}

/// Discover all JSONL session files in a directory, sorted by modification time (newest first).
pub fn discover_sessions(session_dir: &Path) -> crate::Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(session_dir).map_err(|source| AssayError::Io {
        operation: "reading session directory".into(),
        path: session_dir.to_path_buf(),
        source,
    })?;

    let mut files: Vec<(PathBuf, std::time::SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|e| {
            let path = e.path();
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((path, mtime))
        })
        .collect();

    // Sort by modification time, newest first
    files.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(files.into_iter().map(|(p, _)| p).collect())
}

/// Resolve a specific session by ID, or return the most recent session.
///
/// If `session_id` is `Some`, looks for `{session_id}.jsonl` in the directory.
/// Otherwise, returns the most recently modified `.jsonl` file.
pub fn resolve_session(session_dir: &Path, session_id: Option<&str>) -> crate::Result<PathBuf> {
    match session_id {
        Some(id) => {
            let path = session_dir.join(format!("{id}.jsonl"));
            if path.is_file() {
                Ok(path)
            } else {
                Err(AssayError::SessionFileNotFound { path })
            }
        }
        None => {
            let sessions = discover_sessions(session_dir)?;
            sessions
                .into_iter()
                .next()
                .ok_or_else(|| AssayError::SessionDirNotFound {
                    message: format!("no JSONL session files found in {}", session_dir.display()),
                })
        }
    }
}

/// Parse `~/.claude/history.jsonl` to find sessions for a project.
///
/// Best-effort: returns an empty vec on any error.
#[allow(dead_code)]
pub fn sessions_from_history(project_path: &Path) -> Vec<ClaudeHistoryEntry> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let history_path = home.join(".claude").join("history.jsonl");
    let Ok(content) = std::fs::read_to_string(&history_path) else {
        return Vec::new();
    };

    let project_str = project_path.to_string_lossy();

    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<ClaudeHistoryEntry>(l).ok())
        .filter(|e| {
            e.project
                .as_deref()
                .is_some_and(|p| p == project_str.as_ref())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_absolute_path() {
        let slug = path_to_project_slug(Path::new("/Users/dev/projects/myapp"));
        assert_eq!(slug, "Users-dev-projects-myapp");
    }

    #[test]
    fn slug_from_root_path() {
        let slug = path_to_project_slug(Path::new("/"));
        assert_eq!(slug, "");
    }

    #[test]
    fn slug_from_relative_path() {
        // Relative paths have no leading slash to strip
        let slug = path_to_project_slug(Path::new("relative/path"));
        assert_eq!(slug, "relative-path");
    }

    #[test]
    fn discover_sessions_returns_jsonl_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("abc.jsonl"), "{}").unwrap();
        std::fs::write(dir.path().join("xyz.jsonl"), "{}").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "hi").unwrap();

        let found = discover_sessions(dir.path()).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|p| p.extension().unwrap() == "jsonl"));
    }

    #[test]
    fn resolve_session_by_id() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sess-123.jsonl"), "{}").unwrap();

        let path = resolve_session(dir.path(), Some("sess-123")).unwrap();
        assert!(path.ends_with("sess-123.jsonl"));
    }

    #[test]
    fn resolve_session_missing_id_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_session(dir.path(), Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_session_latest_when_no_id() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.jsonl"), "{}").unwrap();
        // Small sleep to ensure different mtimes
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(dir.path().join("new.jsonl"), "{}").unwrap();

        let path = resolve_session(dir.path(), None).unwrap();
        assert!(path.ends_with("new.jsonl"));
    }
}
