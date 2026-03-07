//! File system watcher for reactive session monitoring.
//!
//! Wraps `notify::RecommendedWatcher` to detect session file growth
//! with sub-second latency (kqueue on macOS, inotify on Linux).
//!
//! On macOS (kqueue), the file itself is watched for modifications
//! and the parent directory is watched for creates (atomic writes).
//! On Linux (inotify), watching the parent directory suffices for both.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

/// Watches a session file for modifications.
///
/// Uses `notify::RecommendedWatcher` (kqueue on macOS, inotify on Linux)
/// to detect changes to the target session file. Events for other files
/// in the same directory (e.g., temp files from atomic writes) are filtered out.
pub struct SessionWatcher {
    /// The watcher must be kept alive for events to flow.
    _watcher: RecommendedWatcher,
    /// Channel receiving filtered events.
    pub rx: mpsc::UnboundedReceiver<()>,
}

impl SessionWatcher {
    /// Create a new `SessionWatcher` for the given session file.
    ///
    /// Watches both the file itself (for content modifications) and the
    /// parent directory (for atomic write detection via create+rename).
    /// Only events matching the target file name are forwarded.
    pub fn new(session_path: &Path) -> crate::Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<()>();

        let target_name = session_path
            .file_name()
            .ok_or_else(|| crate::AssayError::Io {
                operation: "resolving session file name for watcher".into(),
                path: session_path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "session path has no file name",
                ),
            })?
            .to_os_string();

        let target_name = Arc::new(Mutex::new(target_name));

        let watcher_target = Arc::clone(&target_name);
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let Ok(event) = res else {
                    return;
                };

                // Only react to modifications and creates (atomic rename)
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {}
                    _ => return,
                }

                let target = watcher_target.lock().unwrap();

                // Check if any event path matches the session file name.
                // For Modify events on the file itself, paths contains the file path.
                // For Create events on the directory, paths contains the new file path.
                let matches = event.paths.iter().any(|p| {
                    if let Some(name) = p.file_name() {
                        // Filter out temp files (e.g., .tmp, ~ suffixes)
                        let name_str = name.to_string_lossy();
                        if name_str.ends_with(".tmp") || name_str.ends_with('~') {
                            return false;
                        }
                        name == target.as_os_str()
                    } else {
                        false
                    }
                });

                if matches {
                    // Best-effort send; if the receiver is dropped, we stop silently
                    let _ = tx.send(());
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .map_err(|e| crate::AssayError::Io {
            operation: "creating file system watcher".into(),
            path: session_path.to_path_buf(),
            source: std::io::Error::other(e.to_string()),
        })?;

        // Watch the file itself for content modifications (kqueue needs this).
        // This may fail if the file doesn't exist yet -- that's OK, the
        // directory watch will catch creates.
        if session_path.exists() {
            let _ = watcher.watch(session_path, RecursiveMode::NonRecursive);
        }

        // Watch the parent directory for creates (atomic writes = create+rename)
        let parent = session_path.parent().unwrap_or(Path::new("."));
        watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(|e| crate::AssayError::Io {
                operation: "watching session directory".into(),
                path: parent.to_path_buf(),
                source: std::io::Error::other(e.to_string()),
            })?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn watcher_can_be_created_for_valid_path() {
        let dir = TempDir::new().unwrap();
        let session_path = dir.path().join("session.jsonl");
        std::fs::write(&session_path, "{}").unwrap();

        let watcher = SessionWatcher::new(&session_path);
        assert!(watcher.is_ok(), "should create watcher without error");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watcher_detects_target_file_modification() {
        let dir = TempDir::new().unwrap();
        let session_path = dir.path().join("session.jsonl");
        std::fs::write(&session_path, "{}").unwrap();

        // Wait for the file to settle before starting the watcher
        tokio::time::sleep(Duration::from_millis(200)).await;

        let mut watcher = SessionWatcher::new(&session_path).unwrap();

        // Drain any events from initial setup and give the watcher time to register
        tokio::time::sleep(Duration::from_secs(1)).await;
        while watcher.rx.try_recv().is_ok() {}

        // Modify the watched file
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&session_path)
            .unwrap();
        writeln!(f, r#"{{"type":"user"}}"#).unwrap();
        f.sync_all().unwrap();
        drop(f);

        // Wait for the event with a timeout
        let result = tokio::time::timeout(Duration::from_secs(5), watcher.rx.recv()).await;

        assert!(result.is_ok(), "should receive event within timeout");
        assert!(result.unwrap().is_some(), "channel should not be closed");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watcher_ignores_unrelated_files() {
        let dir = TempDir::new().unwrap();
        let session_path = dir.path().join("session.jsonl");

        // Create the session file and a sentinel approach:
        // We use a separate subdirectory so that only events for
        // files in the watched dir trigger, but our target doesn't match.
        let watch_dir = dir.path().join("watched");
        std::fs::create_dir_all(&watch_dir).unwrap();
        let target_path = watch_dir.join("session.jsonl");
        std::fs::write(&target_path, "{}").unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        let mut watcher = SessionWatcher::new(&target_path).unwrap();

        // Drain setup events
        tokio::time::sleep(Duration::from_secs(1)).await;
        while watcher.rx.try_recv().is_ok() {}

        // Create files with non-matching names in the watched directory
        let other_path = watch_dir.join("other.jsonl");
        std::fs::write(&other_path, "different file").unwrap();

        let tmp_path = watch_dir.join("session.jsonl.tmp");
        std::fs::write(&tmp_path, "temp content").unwrap();

        // Brief wait to allow any events to propagate
        let result = tokio::time::timeout(Duration::from_secs(1), watcher.rx.recv()).await;

        assert!(
            result.is_err(),
            "should NOT receive event for non-matching files (timeout expected)"
        );
    }
}
