use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use smelt_core::JobManifest;

use crate::serve::queue::ServerState;
use crate::serve::types::JobSource;

/// Watches a directory for `.toml` manifest files, atomically moves them
/// to `dispatched/`, parses each manifest, and enqueues the resulting job.
///
/// The file-move-before-enqueue semantics (D100) prevent double-pickup on
/// restart: once a file has been renamed into `dispatched/`, it will never
/// be picked up again even if the process crashes before `enqueue()`.
pub(crate) struct DirectoryWatcher {
    queue_dir: PathBuf,
    state: Arc<Mutex<ServerState>>,
}

impl DirectoryWatcher {
    pub(crate) fn new(queue_dir: PathBuf, state: Arc<Mutex<ServerState>>) -> Self {
        Self { queue_dir, state }
    }

    /// Poll `queue_dir/` every 2 seconds for `.toml` files.
    ///
    /// For each discovered file:
    /// 1. `create_dir_all(queue_dir/dispatched/)`
    /// 2. Rename to `dispatched/<unix_ms>-<filename>`
    /// 3. Parse the renamed file as a `JobManifest` + `validate()`
    /// 4. Enqueue via `ServerState::enqueue()`
    ///
    /// Parse/rename errors are logged as warnings and skipped — polling
    /// continues regardless.
    pub(crate) async fn watch(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            let entries = match std::fs::read_dir(&self.queue_dir) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(dir = %self.queue_dir.display(), error = %e, "failed to read queue directory");
                    continue;
                }
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                if !path.is_file() {
                    continue;
                }

                // 1. Ensure dispatched/ exists.
                let dispatched_dir = self.queue_dir.join("dispatched");
                if let Err(e) = std::fs::create_dir_all(&dispatched_dir) {
                    tracing::warn!(dir = %dispatched_dir.display(), error = %e, "failed to create dispatched directory");
                    continue;
                }

                // 2. Build dest path with unix_ms prefix.
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let dest = dispatched_dir.join(format!("{ts}-{filename}"));

                // 3. Atomic move (rename) — skip with warning if it fails
                //    (file may already have been moved by a concurrent watcher).
                if let Err(e) = std::fs::rename(&path, &dest) {
                    tracing::warn!(
                        src = %path.display(),
                        dest = %dest.display(),
                        error = %e,
                        "failed to rename manifest to dispatched — skipping"
                    );
                    continue;
                }

                // 4. Parse the renamed file as a JobManifest.
                let content = match std::fs::read_to_string(&dest) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!(path = %dest.display(), error = %e, "failed to read dispatched manifest");
                        continue;
                    }
                };

                let manifest = match JobManifest::from_str(&content, &dest) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(path = %dest.display(), error = %e, "failed to parse manifest TOML — skipping");
                        continue;
                    }
                };

                if let Err(e) = manifest.validate() {
                    tracing::warn!(path = %dest.display(), error = %e, "manifest validation failed — skipping");
                    continue;
                }

                // 5. Enqueue the job.
                let job_id = self.state.lock().unwrap().enqueue(dest.clone(), JobSource::DirectoryWatch);
                tracing::info!(job_id = %job_id, manifest = %dest.display(), "manifest enqueued via directory watch");
            }
        }
    }
}
