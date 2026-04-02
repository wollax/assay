//! Cross-job PeerUpdate routing — evaluates `[[notify]]` rules on session-completion events.
//!
//! After `post_event` calls `ingest_event()` for an event whose
//! `payload["phase"] == "complete"`, it calls `evaluate_notify_rules()` to
//! determine which target jobs should receive a `PeerUpdate` signal. The caller
//! then calls `deliver_peer_update()` for each result.
//!
//! Design decisions: D179 (declarative routing, silent skip for absent/terminal targets).

use std::path::{Path, PathBuf};

use smelt_core::manifest::{JobManifest, NotifyRule};

use crate::serve::events::AssayEvent;
use crate::serve::signals::{GateSummary, PeerUpdate};
use crate::serve::types::JobId;

/// Detect whether an event represents an Assay session completion.
///
/// Returns `true` when `payload["phase"] == "complete"`.
pub(crate) fn is_session_complete(event: &AssayEvent) -> bool {
    event
        .payload
        .get("phase")
        .and_then(|v| v.as_str())
        .is_some_and(|p| p == "complete")
}

/// Extract the session name from an event payload.
///
/// Looks for `payload["sessions"][0]["name"]` (the first session in the array).
/// Falls back to the job name if absent.
pub(crate) fn extract_session_name(event: &AssayEvent) -> String {
    event
        .payload
        .get("sessions")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(&event.job_id)
        .to_string()
}

/// Extract a structured gate summary from the event payload.
///
/// Counts sessions by terminal `state` field (Assay's `SessionRunState`):
/// - `"completed"` → passed
/// - `"failed"` → failed
/// - `"skipped"` → skipped
///
/// Falls back to legacy `passed: bool` field for backward compatibility
/// with existing test events that use the old format.
///
/// Returns `GateSummary { 0, 0, 0 }` when session data is absent.
pub(crate) fn extract_gate_summary(event: &AssayEvent) -> GateSummary {
    let sessions = event.payload.get("sessions").and_then(|v| v.as_array());

    match sessions {
        None => GateSummary {
            passed: 0,
            failed: 0,
            skipped: 0,
        },
        Some(sessions) if sessions.is_empty() => GateSummary {
            passed: 0,
            failed: 0,
            skipped: 0,
        },
        Some(sessions) => {
            let mut passed = 0u32;
            let mut failed = 0u32;
            let mut skipped = 0u32;

            for s in sessions {
                // Primary: count by Assay's SessionRunState `state` field.
                if let Some(state) = s.get("state").and_then(|v| v.as_str()) {
                    match state.to_lowercase().as_str() {
                        "completed" => passed += 1,
                        "failed" => failed += 1,
                        "skipped" => skipped += 1,
                        _ => {} // Pending, Running — not terminal
                    }
                } else if let Some(p) = s.get("passed").and_then(|v| v.as_bool()) {
                    // Legacy fallback: `passed: bool` from existing test events.
                    if p {
                        passed += 1;
                    } else {
                        failed += 1;
                    }
                }
            }

            GateSummary {
                passed,
                failed,
                skipped,
            }
        }
    }
}

/// A routing decision: deliver a PeerUpdate to `job_id`.
pub(crate) struct NotifyTarget {
    /// Target job that should receive the PeerUpdate.
    pub job_id: JobId,
    /// Target job's manifest path — carried here so the caller avoids a second queue scan
    /// and can resolve the repo path + target session names in one manifest read.
    pub manifest_path: PathBuf,
    pub peer_update: PeerUpdate,
}

/// Combined routing info read from a source job's manifest in a single pass.
struct ManifestRoutingInfo {
    notify_rules: Vec<NotifyRule>,
    /// The `merge.target` branch — the result branch peers should pull from.
    merge_target: String,
}

/// Read notify rules and base_ref from a manifest file in one pass.
///
/// Returns `None` on read/parse error (logs at warn). Returns `Some` with an
/// empty `notify_rules` vec when the manifest has no `[[notify]]` entries —
/// distinct from an error.
fn read_manifest_routing_info(manifest_path: &Path) -> Option<ManifestRoutingInfo> {
    let content = std::fs::read_to_string(manifest_path)
        .map_err(|e| {
            tracing::warn!(
                path = %manifest_path.display(),
                error = %e,
                "notify: cannot read manifest for rule evaluation"
            );
        })
        .ok()?;

    let dir = manifest_path.parent().unwrap_or(Path::new("."));
    let manifest = JobManifest::from_str(&content, dir)
        .map_err(|e| {
            tracing::warn!(
                path = %manifest_path.display(),
                error = %e,
                "notify: cannot parse manifest for rule evaluation"
            );
        })
        .ok()?;

    Some(ManifestRoutingInfo {
        notify_rules: manifest.notify,
        merge_target: manifest.merge.target,
    })
}

/// Resolve the repo path and session names from a target job's manifest.
///
/// Returns `None` on any error (logged per step). This function is the canonical
/// way to resolve a target job's delivery context from its manifest path.
pub(crate) fn resolve_target_delivery_context(
    manifest_path: &Path,
) -> Option<(PathBuf, Vec<String>)> {
    let content = std::fs::read_to_string(manifest_path)
        .map_err(|e| {
            tracing::warn!(
                path = %manifest_path.display(),
                error = %e,
                "notify: cannot read target manifest"
            );
        })
        .ok()?;

    let dir = manifest_path.parent().unwrap_or(Path::new("."));
    let manifest = JobManifest::from_str(&content, dir)
        .map_err(|e| {
            tracing::warn!(
                path = %manifest_path.display(),
                error = %e,
                "notify: cannot parse target manifest"
            );
        })
        .ok()?;

    let repo_path = smelt_core::manifest::resolve_repo_path(&manifest.job.repo)
        .map_err(|e| {
            tracing::warn!(
                path = %manifest_path.display(),
                repo = %manifest.job.repo,
                error = %e,
                "notify: cannot resolve repo path in target manifest"
            );
        })
        .ok()?;

    let session_names = manifest
        .session
        .iter()
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();

    Some((repo_path, session_names))
}

/// Evaluate `[[notify]]` rules for a session-completion event.
///
/// **Call this after dropping the `ServerState` lock** — it reads the source
/// job's manifest from disk.
///
/// Returns the list of notify targets to deliver. Empty when:
/// - The event is not a session completion (`phase != "complete"`)
/// - The source job's manifest cannot be read or parsed (logged at warn)
/// - The source job has no `[[notify]]` rules
/// - All matching targets are absent or in a terminal state (silent skip per D179)
///
/// `source_manifest_path` is the manifest path for the source job, which must be
/// extracted from `ServerState` before the lock is dropped.
pub(crate) fn evaluate_notify_rules(
    event: &AssayEvent,
    source_manifest_path: &Path,
    // The subset of queue state needed — extracted under the lock before calling this.
    queued_jobs: &[QueuedJobSnapshot],
) -> Vec<NotifyTarget> {
    // Only fire on session-completion events.
    if !is_session_complete(event) {
        return Vec::new();
    }

    // Parse the source manifest (filesystem I/O — must be called outside the lock).
    let routing_info = match read_manifest_routing_info(source_manifest_path) {
        None => {
            tracing::warn!(
                job_id = %event.job_id,
                path = %source_manifest_path.display(),
                "evaluate_notify_rules: manifest unreadable/unparseable — notify rules cannot be evaluated"
            );
            return Vec::new();
        }
        Some(info) if info.notify_rules.is_empty() => return Vec::new(), // normal: no rules
        Some(info) => info,
    };

    let session_name = extract_session_name(event);
    let gate_summary = extract_gate_summary(event);
    // Use merge.target as the branch — that's the result branch peers care about.
    let branch = if routing_info.merge_target.is_empty() {
        tracing::warn!(
            job_id = %event.job_id,
            path = %source_manifest_path.display(),
            "notify: merge.target is empty in source manifest — falling back to 'main'; \
             PeerUpdate branch may be incorrect"
        );
        "main".to_string()
    } else {
        routing_info.merge_target.clone()
    };

    let mut targets = Vec::new();

    for rule in &routing_info.notify_rules {
        if !rule.on_session_complete {
            continue;
        }

        // Find the target job by manifest job.name (not server-assigned JobId).
        let target = match queued_jobs.iter().find(|j| j.job_name == rule.target_job) {
            Some(j) => j,
            None => {
                tracing::debug!(
                    source_job = %event.job_id,
                    target_job = %rule.target_job,
                    "notify: target job not found — skipping"
                );
                continue;
            }
        };

        // Skip terminal targets (D179): Complete or Failed jobs cannot receive new signals.
        if target.is_terminal {
            tracing::debug!(
                source_job = %event.job_id,
                target_job = %rule.target_job,
                "notify: target job is in terminal state — skipping"
            );
            continue;
        }

        targets.push(NotifyTarget {
            job_id: target.job_id.clone(),
            manifest_path: target.manifest_path.clone(),
            peer_update: PeerUpdate {
                source_job: event.job_id.clone(),
                source_session: session_name.clone(),
                changed_files: Vec::new(), // TODO: populate with per-job changed-file tracking
                gate_summary: gate_summary.clone(),
                branch: branch.clone(),
            },
        });
    }

    targets
}

/// Snapshot of a queued job's routing-relevant fields.
/// Extracted from `ServerState` under the lock, then passed to `evaluate_notify_rules`
/// so filesystem I/O can happen outside the lock.
pub(crate) struct QueuedJobSnapshot {
    pub job_id: JobId,
    /// The `[job].name` field from the manifest, used to match `[[notify]] target_job`.
    pub job_name: String,
    pub manifest_path: PathBuf,
    pub is_terminal: bool,
}
