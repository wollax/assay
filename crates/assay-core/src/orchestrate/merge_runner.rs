//! Sequential merge runner for completed session branches.
//!
//! Wires together ordering ([`order_sessions`]), merge execution
//! ([`merge_execute`]), and a closure-based conflict handler into a
//! complete sequential merge loop.

use std::path::{Path, PathBuf};
use std::time::Instant;

use assay_types::{
    ConflictAction, ConflictScan, MergePlan, MergeReport, MergeSessionResult, MergeSessionStatus,
    MergeStrategy,
};

use crate::orchestrate::conflict_resolver::ConflictResolutionResult;

use crate::error::{AssayError, Result};
use crate::merge::{git_raw, merge_execute};
use crate::orchestrate::executor::SessionOutcome;
use crate::orchestrate::ordering::{CompletedSession, order_sessions};

/// Configuration for the merge runner.
#[derive(Debug, Clone)]
pub struct MergeRunnerConfig {
    /// Which ordering strategy to use.
    pub strategy: MergeStrategy,
    /// Path to the project root (the git repo to merge into).
    pub project_root: PathBuf,
    /// The base branch to merge onto (must be checked out).
    pub base_branch: String,
    /// When `true`, the merge runner uses two-phase merge: conflicts leave
    /// the working tree dirty so the conflict handler can resolve in-place.
    /// When `false` (default), conflicts are auto-aborted before the handler
    /// sees them (existing behavior).
    pub conflict_resolution_enabled: bool,
}

/// Execute sequential merges for all completed sessions.
///
/// 1. Validates the working tree is clean and no merge is in progress.
/// 2. Orders sessions using the configured strategy.
/// 3. Iterates in order, calling [`merge_execute`] for each.
/// 4. On conflict, invokes the `conflict_handler` and acts on the returned
///    [`ConflictResolutionResult`].
/// 5. Returns a [`MergeReport`] with per-session results and aggregate counts.
///
/// The conflict handler receives `(session_name, conflicting_files, scan, work_dir)`
/// and must return a [`ConflictResolutionResult`].
pub fn merge_completed_sessions<H>(
    completed_sessions: Vec<CompletedSession>,
    config: &MergeRunnerConfig,
    conflict_handler: H,
) -> Result<MergeReport>
where
    H: Fn(&str, &[String], &ConflictScan, &Path) -> ConflictResolutionResult,
{
    let start = Instant::now();
    let project_root = &config.project_root;

    // ── Pre-flight checks ────────────────────────────────────────────

    // 1. Working tree must be clean.
    let (status_out, _, _) = git_raw(&["status", "--porcelain"], project_root)?;
    if !status_out.trim().is_empty() {
        return Err(AssayError::MergeExecuteError {
            branch: String::new(),
            conflicting_files: Vec::new(),
            message: format!(
                "working tree is not clean — please commit or stash changes before merging. \
                 Dirty files:\n{}",
                status_out.trim()
            ),
        });
    }

    // 2. No in-progress merge.
    let git_dir = project_root.join(".git");
    let merge_head = if git_dir.is_file() {
        let content = std::fs::read_to_string(&git_dir).map_err(|e| AssayError::Io {
            operation: "reading .git file".to_string(),
            path: git_dir.clone(),
            source: e,
        })?;
        let gitdir = content
            .trim()
            .strip_prefix("gitdir: ")
            .unwrap_or(content.trim());
        Path::new(gitdir).join("MERGE_HEAD")
    } else {
        git_dir.join("MERGE_HEAD")
    };

    if merge_head.exists() {
        return Err(AssayError::MergeExecuteError {
            branch: String::new(),
            conflicting_files: Vec::new(),
            message: "a merge is already in progress (MERGE_HEAD exists)".to_string(),
        });
    }

    // ── Handle empty input ───────────────────────────────────────────

    if completed_sessions.is_empty() {
        return Ok(MergeReport {
            sessions_merged: 0,
            sessions_skipped: 0,
            conflict_skipped: 0,
            aborted: 0,
            plan: MergePlan {
                strategy: config.strategy,
                entries: Vec::new(),
            },
            results: Vec::new(),
            duration_secs: start.elapsed().as_secs_f64(),
            resolutions: vec![],
        });
    }

    // ── Order sessions ───────────────────────────────────────────────

    let (ordered, plan) = order_sessions(completed_sessions, config.strategy);

    // ── Sequential merge loop ────────────────────────────────────────

    let mut results: Vec<MergeSessionResult> = Vec::with_capacity(ordered.len());
    let mut resolutions: Vec<assay_types::ConflictResolution> = Vec::new();
    let mut sessions_merged: usize = 0;
    let mut conflict_skipped: usize = 0;
    let mut aborted_count: usize = 0;
    let mut abort_triggered = false;

    for session in &ordered {
        if abort_triggered {
            results.push(MergeSessionResult {
                session_name: session.session_name.clone(),
                status: MergeSessionStatus::Aborted,
                merge_sha: None,
                error: Some("merge sequence aborted by conflict handler".to_string()),
            });
            aborted_count += 1;
            continue;
        }

        let message = format!(
            "Merge branch '{}' into {} (session: {})",
            session.branch_name, config.base_branch, session.session_name
        );

        // Two-phase merge: when conflict_resolution_enabled is true, don't
        // auto-abort on conflict — leave the working tree conflicted so the
        // handler can resolve in-place. When false (default), auto-abort
        // before handing to the handler (existing behavior).
        let abort_on_conflict = !config.conflict_resolution_enabled;

        match merge_execute(
            project_root,
            &session.branch_name,
            &message,
            abort_on_conflict,
        ) {
            Ok(result) if !result.was_conflict => {
                // Successful merge
                results.push(MergeSessionResult {
                    session_name: session.session_name.clone(),
                    status: MergeSessionStatus::Merged,
                    merge_sha: result.merge_sha,
                    error: None,
                });
                sessions_merged += 1;
            }
            Ok(result) => {
                // Conflict — invoke the handler
                let empty_scan = ConflictScan {
                    has_markers: false,
                    markers: Vec::new(),
                    truncated: false,
                };
                let scan = result.conflict_details.as_ref().unwrap_or(&empty_scan);

                // Extract conflicting files from the scan markers
                let conflicting_files: Vec<String> = scan
                    .markers
                    .iter()
                    .map(|m| m.file.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                if config.conflict_resolution_enabled {
                    // Two-phase path: working tree is still conflicted.
                    // Wrap handler in catch_unwind for panic safety.
                    let handler_result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            conflict_handler(
                                &session.session_name,
                                &conflicting_files,
                                scan,
                                project_root,
                            )
                        }));

                    match handler_result {
                        Ok(ConflictResolutionResult {
                            action: ConflictAction::Resolved(sha),
                            audit,
                            repo_clean: _,
                        }) => {
                            // Verify the commit SHA exists
                            match git_raw(&["rev-parse", "--verify", &sha], project_root) {
                                Ok((_, _, Some(0))) => {
                                    // Push audit record if present
                                    if let Some(audit_record) = audit {
                                        resolutions.push(audit_record);
                                    }
                                    results.push(MergeSessionResult {
                                        session_name: session.session_name.clone(),
                                        status: MergeSessionStatus::Merged,
                                        merge_sha: Some(sha),
                                        error: None,
                                    });
                                    sessions_merged += 1;
                                }
                                _ => {
                                    // SHA verification failed — abort the merge
                                    let _ = git_raw(&["merge", "--abort"], project_root);
                                    results.push(MergeSessionResult {
                                        session_name: session.session_name.clone(),
                                        status: MergeSessionStatus::ConflictSkipped,
                                        merge_sha: None,
                                        error: Some(format!(
                                            "conflict handler returned invalid SHA: {sha}"
                                        )),
                                    });
                                    conflict_skipped += 1;
                                }
                            }
                        }
                        Ok(ConflictResolutionResult {
                            action: ConflictAction::Skip,
                            repo_clean,
                            ..
                        }) => {
                            // Handler chose to skip — abort only if repo is not already clean
                            if !repo_clean {
                                let _ = git_raw(&["merge", "--abort"], project_root);
                            }
                            results.push(MergeSessionResult {
                                session_name: session.session_name.clone(),
                                status: MergeSessionStatus::ConflictSkipped,
                                merge_sha: None,
                                error: Some(format!(
                                    "conflict skipped — conflicting files: {}",
                                    conflicting_files.join(", ")
                                )),
                            });
                            conflict_skipped += 1;
                        }
                        Ok(ConflictResolutionResult {
                            action: ConflictAction::Abort,
                            repo_clean,
                            ..
                        }) => {
                            // Handler chose to abort — clean up and stop the loop
                            if !repo_clean {
                                let _ = git_raw(&["merge", "--abort"], project_root);
                            }
                            results.push(MergeSessionResult {
                                session_name: session.session_name.clone(),
                                status: MergeSessionStatus::Aborted,
                                merge_sha: None,
                                error: Some("merge aborted by conflict handler".to_string()),
                            });
                            aborted_count += 1;
                            abort_triggered = true;
                        }
                        Err(_panic) => {
                            // Handler panicked — no commit was made, always abort
                            let _ = git_raw(&["merge", "--abort"], project_root);
                            results.push(MergeSessionResult {
                                session_name: session.session_name.clone(),
                                status: MergeSessionStatus::ConflictSkipped,
                                merge_sha: None,
                                error: Some(
                                    "conflict handler panicked — merge aborted".to_string(),
                                ),
                            });
                            conflict_skipped += 1;
                        }
                    }
                } else {
                    // Default path: merge was already aborted by merge_execute.
                    // Handler receives a post-abort scan (existing behavior).
                    let result = conflict_handler(
                        &session.session_name,
                        &conflicting_files,
                        scan,
                        project_root,
                    );

                    match result.action {
                        ConflictAction::Skip => {
                            results.push(MergeSessionResult {
                                session_name: session.session_name.clone(),
                                status: MergeSessionStatus::ConflictSkipped,
                                merge_sha: None,
                                error: Some(format!(
                                    "conflict skipped — conflicting files: {}",
                                    conflicting_files.join(", ")
                                )),
                            });
                            conflict_skipped += 1;
                        }
                        ConflictAction::Abort => {
                            results.push(MergeSessionResult {
                                session_name: session.session_name.clone(),
                                status: MergeSessionStatus::Aborted,
                                merge_sha: None,
                                error: Some("merge aborted by conflict handler".to_string()),
                            });
                            aborted_count += 1;
                            abort_triggered = true;
                        }
                        ConflictAction::Resolved(sha) => {
                            // The handler resolved the conflict externally and committed.
                            if let Some(audit_record) = result.audit {
                                resolutions.push(audit_record);
                            }
                            results.push(MergeSessionResult {
                                session_name: session.session_name.clone(),
                                status: MergeSessionStatus::Merged,
                                merge_sha: Some(sha),
                                error: None,
                            });
                            sessions_merged += 1;
                        }
                    }
                }
            }
            Err(e) => {
                // Infrastructure failure
                results.push(MergeSessionResult {
                    session_name: session.session_name.clone(),
                    status: MergeSessionStatus::Failed,
                    merge_sha: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(MergeReport {
        sessions_merged,
        sessions_skipped: 0,
        conflict_skipped,
        aborted: aborted_count,
        plan,
        results,
        duration_secs: start.elapsed().as_secs_f64(),
        resolutions,
    })
}

/// Returns a default conflict handler that always skips conflicting sessions.
pub fn default_conflict_handler()
-> impl Fn(&str, &[String], &ConflictScan, &Path) -> ConflictResolutionResult {
    |_session_name, _conflicting_files, _scan, _work_dir| ConflictResolutionResult {
        action: ConflictAction::Skip,
        audit: None,
        repo_clean: false,
    }
}

/// Extract [`CompletedSession`]s from orchestrator outcomes.
///
/// Filters to `SessionOutcome::Completed` variants and derives branch names
/// from session names using the `assay/<slug>` pattern when the `branch_name`
/// field is empty (the executor currently uses placeholder empty strings).
pub fn extract_completed_sessions(outcomes: &[(String, SessionOutcome)]) -> Vec<CompletedSession> {
    outcomes
        .iter()
        .filter_map(|(name, outcome)| match outcome {
            SessionOutcome::Completed {
                worktree_path: _,
                branch_name,
                changed_files,
                result: _,
            } => {
                let effective_branch = if branch_name.is_empty() {
                    format!("assay/{}", name)
                } else {
                    branch_name.clone()
                };

                Some(CompletedSession {
                    session_name: name.clone(),
                    branch_name: effective_branch,
                    changed_files: changed_files.clone(),
                    completed_at: chrono::Utc::now(),
                    topo_order: 0,
                })
            }
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Helper: create a new git repo in a temp dir with an initial commit on `main`.
    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();

        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(p)
            .output()
            .unwrap();

        std::fs::write(p.join("readme.md"), "# hello\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(p)
            .output()
            .unwrap();

        dir
    }

    /// Helper: create a branch with a single file commit and switch back to main.
    fn create_branch_with_file(repo: &Path, branch: &str, file: &str, content: &str) {
        Command::new("git")
            .args(["checkout", "-b", branch])
            .current_dir(repo)
            .output()
            .unwrap();
        std::fs::write(repo.join(file), content).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("add {file} on {branch}")])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();
    }

    fn make_session(name: &str, branch: &str, files: Vec<String>, topo: usize) -> CompletedSession {
        CompletedSession {
            session_name: name.to_string(),
            branch_name: branch.to_string(),
            changed_files: files,
            completed_at: chrono::Utc::now(),
            topo_order: topo,
        }
    }

    // ── Test: 3 sessions, no conflicts ───────────────────────────────

    #[test]
    fn test_merge_three_sessions_no_conflicts() {
        let dir = setup_git_repo();
        let p = dir.path();

        create_branch_with_file(p, "assay/session-a", "a.rs", "fn a() {}\n");
        create_branch_with_file(p, "assay/session-b", "b.rs", "fn b() {}\n");
        create_branch_with_file(p, "assay/session-c", "c.rs", "fn c() {}\n");

        let sessions = vec![
            make_session("session-a", "assay/session-a", vec!["a.rs".into()], 0),
            make_session("session-b", "assay/session-b", vec!["b.rs".into()], 1),
            make_session("session-c", "assay/session-c", vec!["c.rs".into()], 2),
        ];

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: false,
        };

        let report =
            merge_completed_sessions(sessions, &config, default_conflict_handler()).unwrap();

        assert_eq!(report.sessions_merged, 3);
        assert_eq!(report.conflict_skipped, 0);
        assert_eq!(report.aborted, 0);
        assert_eq!(report.results.len(), 3);

        for r in &report.results {
            assert_eq!(r.status, MergeSessionStatus::Merged);
            assert!(r.merge_sha.is_some());
        }

        // Verify all files exist on main
        assert!(p.join("a.rs").exists());
        assert!(p.join("b.rs").exists());
        assert!(p.join("c.rs").exists());
    }

    // ── Test: conflict in middle, skip handler ───────────────────────

    #[test]
    fn test_merge_conflict_skip_continues() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Session A: modify readme.md (no conflict with initial)
        create_branch_with_file(p, "assay/session-a", "a.rs", "fn a() {}\n");

        // Session B: also touches a.rs but on a branch from initial (will conflict after A merges)
        Command::new("git")
            .args(["checkout", "-b", "assay/session-b"])
            .current_dir(p)
            .output()
            .unwrap();
        // Write a DIFFERENT a.rs on this branch
        std::fs::write(p.join("a.rs"), "fn a_conflict() {}\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add a.rs on session-b"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(p)
            .output()
            .unwrap();

        // Session C: touches a different file (no conflict)
        create_branch_with_file(p, "assay/session-c", "c.rs", "fn c() {}\n");

        let sessions = vec![
            make_session("session-a", "assay/session-a", vec!["a.rs".into()], 0),
            make_session("session-b", "assay/session-b", vec!["a.rs".into()], 1),
            make_session("session-c", "assay/session-c", vec!["c.rs".into()], 2),
        ];

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: false,
        };

        let report =
            merge_completed_sessions(sessions, &config, default_conflict_handler()).unwrap();

        assert_eq!(report.sessions_merged, 2, "A and C should merge");
        assert_eq!(report.conflict_skipped, 1, "B should be conflict-skipped");
        assert_eq!(report.aborted, 0);
        assert_eq!(report.results.len(), 3);

        assert_eq!(report.results[0].status, MergeSessionStatus::Merged);
        assert_eq!(report.results[0].session_name, "session-a");

        assert_eq!(
            report.results[1].status,
            MergeSessionStatus::ConflictSkipped
        );
        assert_eq!(report.results[1].session_name, "session-b");

        assert_eq!(report.results[2].status, MergeSessionStatus::Merged);
        assert_eq!(report.results[2].session_name, "session-c");

        // Verify: a.rs has session-a content, c.rs exists, b's content not present
        let a_content = std::fs::read_to_string(p.join("a.rs")).unwrap();
        assert_eq!(a_content, "fn a() {}\n");
        assert!(p.join("c.rs").exists());
    }

    // ── Test: abort handler stops loop ───────────────────────────────

    #[test]
    fn test_merge_abort_stops_loop() {
        let dir = setup_git_repo();
        let p = dir.path();

        create_branch_with_file(p, "assay/session-a", "a.rs", "fn a() {}\n");

        // Session B conflicts with A
        Command::new("git")
            .args(["checkout", "-b", "assay/session-b"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("a.rs"), "fn a_different() {}\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add a.rs on session-b"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(p)
            .output()
            .unwrap();

        create_branch_with_file(p, "assay/session-c", "c.rs", "fn c() {}\n");

        let sessions = vec![
            make_session("session-a", "assay/session-a", vec!["a.rs".into()], 0),
            make_session("session-b", "assay/session-b", vec!["a.rs".into()], 1),
            make_session("session-c", "assay/session-c", vec!["c.rs".into()], 2),
        ];

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: false,
        };

        // Abort handler
        let abort_handler = |_name: &str, _files: &[String], _scan: &ConflictScan, _dir: &Path| {
            ConflictResolutionResult {
                action: ConflictAction::Abort,
                audit: None,
                repo_clean: false,
            }
        };

        let report = merge_completed_sessions(sessions, &config, abort_handler).unwrap();

        assert_eq!(report.sessions_merged, 1, "only A should merge");
        assert_eq!(report.conflict_skipped, 0);
        // B triggers abort, C gets marked aborted
        assert_eq!(report.aborted, 2, "B (abort trigger) + C (remaining)");
        assert_eq!(report.results.len(), 3);

        assert_eq!(report.results[0].status, MergeSessionStatus::Merged);
        assert_eq!(report.results[1].status, MergeSessionStatus::Aborted);
        assert_eq!(report.results[2].status, MergeSessionStatus::Aborted);

        // C should NOT have been merged
        assert!(!p.join("c.rs").exists());
    }

    // ── Test: empty sessions ─────────────────────────────────────────

    #[test]
    fn test_merge_empty_sessions() {
        let dir = setup_git_repo();
        let p = dir.path();

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: false,
        };

        let report =
            merge_completed_sessions(Vec::new(), &config, default_conflict_handler()).unwrap();

        assert_eq!(report.sessions_merged, 0);
        assert_eq!(report.sessions_skipped, 0);
        assert_eq!(report.conflict_skipped, 0);
        assert_eq!(report.aborted, 0);
        assert!(report.results.is_empty());
    }

    // ── Test: dirty working tree ─────────────────────────────────────

    #[test]
    fn test_merge_dirty_working_tree_error() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Create an uncommitted file
        std::fs::write(p.join("dirty.txt"), "uncommitted\n").unwrap();

        let sessions = vec![make_session(
            "session-a",
            "assay/session-a",
            vec!["a.rs".into()],
            0,
        )];

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: false,
        };

        let err =
            merge_completed_sessions(sessions, &config, default_conflict_handler()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("not clean"),
            "error should mention dirty tree: {msg}"
        );
    }

    // ── Test: extract_completed_sessions ─────────────────────────────

    #[test]
    fn test_extract_completed_sessions_derives_branch_names() {
        use std::time::Duration;

        use crate::pipeline::{PipelineError, PipelineOutcome, PipelineResult, PipelineStage};

        // Create a minimal completed outcome with empty branch_name
        let result = PipelineResult {
            session_id: "sid-1".to_string(),
            spec_name: "test".to_string(),
            gate_summary: None,
            merge_check: None,
            stage_timings: Vec::new(),
            outcome: PipelineOutcome::Success,
        };

        let outcomes: Vec<(String, SessionOutcome)> = vec![
            (
                "auth".to_string(),
                SessionOutcome::Completed {
                    result: Box::new(result.clone()),
                    worktree_path: PathBuf::from("/tmp/wt"),
                    branch_name: String::new(), // empty — should derive
                    changed_files: vec!["auth.rs".to_string()],
                },
            ),
            (
                "db".to_string(),
                SessionOutcome::Completed {
                    result: Box::new(result),
                    worktree_path: PathBuf::from("/tmp/wt2"),
                    branch_name: "custom-branch".to_string(), // explicit — keep
                    changed_files: vec!["db.rs".to_string()],
                },
            ),
            (
                "failed".to_string(),
                SessionOutcome::Failed {
                    error: PipelineError {
                        stage: PipelineStage::SpecLoad,
                        message: "fail".to_string(),
                        recovery: "retry".to_string(),
                        elapsed: Duration::from_secs(0),
                    },
                    stage: PipelineStage::SpecLoad,
                },
            ),
        ];

        let completed = extract_completed_sessions(&outcomes);
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].session_name, "auth");
        assert_eq!(completed[0].branch_name, "assay/auth");
        assert_eq!(completed[1].session_name, "db");
        assert_eq!(completed[1].branch_name, "custom-branch");
    }

    // ── Test: conflict resolution with live tree ─────────────────────

    /// Helper: create a branch that modifies an existing file (creates a real
    /// merge conflict when two branches modify the same file differently).
    fn create_branch_modifying_file(repo: &Path, branch: &str, file: &str, content: &str) {
        Command::new("git")
            .args(["checkout", "-b", branch])
            .current_dir(repo)
            .output()
            .unwrap();
        std::fs::write(repo.join(file), content).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("modify {file} on {branch}")])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();
    }

    #[test]
    fn test_merge_runner_conflict_resolution_with_live_tree() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Create a shared file on main so both branches modify it
        std::fs::write(p.join("shared.rs"), "fn shared() { /* original */ }\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add shared.rs"])
            .current_dir(p)
            .output()
            .unwrap();

        // Branch A modifies shared.rs one way
        create_branch_modifying_file(
            p,
            "assay/session-a",
            "shared.rs",
            "fn shared() { /* version A */ }\n",
        );

        // Branch B modifies shared.rs differently (will conflict after A merges)
        create_branch_modifying_file(
            p,
            "assay/session-b",
            "shared.rs",
            "fn shared() { /* version B */ }\n",
        );

        let sessions = vec![
            make_session("session-a", "assay/session-a", vec!["shared.rs".into()], 0),
            make_session("session-b", "assay/session-b", vec!["shared.rs".into()], 1),
        ];

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: true,
        };

        // Scripted handler: strips conflict markers, keeps both sides, stages, commits
        let resolver_handler =
            |_name: &str, _files: &[String], _scan: &ConflictScan, work_dir: &Path| {
                let shared_path = work_dir.join("shared.rs");
                let content = std::fs::read_to_string(&shared_path).unwrap();

                // Strip conflict markers, keeping both sides
                let mut resolved = String::new();
                for line in content.lines() {
                    if line.starts_with("<<<<<<<")
                        || line.starts_with("=======")
                        || line.starts_with(">>>>>>>")
                    {
                        continue;
                    }
                    resolved.push_str(line);
                    resolved.push('\n');
                }

                // Write the resolved content
                std::fs::write(&shared_path, &resolved).unwrap();

                // Stage and commit
                Command::new("git")
                    .args(["add", "shared.rs"])
                    .current_dir(work_dir)
                    .output()
                    .unwrap();
                let output = Command::new("git")
                    .args(["commit", "--no-edit"])
                    .current_dir(work_dir)
                    .output()
                    .unwrap();
                assert!(output.status.success(), "commit should succeed");

                // Get the commit SHA
                let sha_output = Command::new("git")
                    .args(["rev-parse", "HEAD"])
                    .current_dir(work_dir)
                    .output()
                    .unwrap();
                let sha = String::from_utf8_lossy(&sha_output.stdout)
                    .trim()
                    .to_string();
                ConflictResolutionResult {
                    action: ConflictAction::Resolved(sha),
                    audit: None,
                    repo_clean: false,
                }
            };

        let report = merge_completed_sessions(sessions, &config, resolver_handler).unwrap();

        assert_eq!(report.sessions_merged, 2, "both A and B should merge");
        assert_eq!(report.conflict_skipped, 0, "no skips");
        assert_eq!(report.aborted, 0, "no aborts");
        assert_eq!(report.results.len(), 2);

        // Session A: clean merge (no conflict)
        assert_eq!(report.results[0].status, MergeSessionStatus::Merged);
        assert_eq!(report.results[0].session_name, "session-a");
        assert!(report.results[0].merge_sha.is_some());

        // Session B: conflict resolved by handler
        assert_eq!(report.results[1].status, MergeSessionStatus::Merged);
        assert_eq!(report.results[1].session_name, "session-b");
        let merge_sha = report.results[1].merge_sha.as_ref().unwrap();
        assert!(!merge_sha.is_empty());

        // Verify the merge commit has two parents (it's a real merge)
        let parents_output = Command::new("git")
            .args(["rev-list", "--parents", "-n", "1", merge_sha])
            .current_dir(p)
            .output()
            .unwrap();
        let parents_line = String::from_utf8_lossy(&parents_output.stdout)
            .trim()
            .to_string();
        let parent_count = parents_line.split_whitespace().count() - 1; // subtract the commit itself
        assert_eq!(parent_count, 2, "merge commit should have 2 parents");

        // Verify the resolved content contains both versions
        let final_content = std::fs::read_to_string(p.join("shared.rs")).unwrap();
        assert!(
            final_content.contains("version A") && final_content.contains("version B"),
            "resolved file should contain both versions, got: {final_content}"
        );

        // Verify repo is clean (no MERGE_HEAD)
        assert!(
            !p.join(".git").join("MERGE_HEAD").exists(),
            "MERGE_HEAD should not exist after resolution"
        );
    }

    // ── Test: conflict resolution handler failure + panic ────────────

    #[test]
    fn test_merge_runner_conflict_resolution_handler_failure() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Create a shared file on main
        std::fs::write(p.join("shared.rs"), "fn shared() { /* original */ }\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add shared.rs"])
            .current_dir(p)
            .output()
            .unwrap();

        // Two branches that will conflict
        create_branch_modifying_file(
            p,
            "assay/session-a",
            "shared.rs",
            "fn shared() { /* A */ }\n",
        );
        create_branch_modifying_file(
            p,
            "assay/session-b",
            "shared.rs",
            "fn shared() { /* B */ }\n",
        );

        // ── Test 1: Handler returns Skip ──

        let sessions = vec![
            make_session("session-a", "assay/session-a", vec!["shared.rs".into()], 0),
            make_session("session-b", "assay/session-b", vec!["shared.rs".into()], 1),
        ];

        let config = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: true,
        };

        let skip_handler = |_name: &str, _files: &[String], _scan: &ConflictScan, _dir: &Path| {
            ConflictResolutionResult {
                action: ConflictAction::Skip,
                audit: None,
                repo_clean: false,
            }
        };

        let report = merge_completed_sessions(sessions, &config, skip_handler).unwrap();

        assert_eq!(report.sessions_merged, 1, "only A should merge cleanly");
        assert_eq!(report.conflict_skipped, 1, "B should be conflict-skipped");
        assert_eq!(
            report.results[1].status,
            MergeSessionStatus::ConflictSkipped
        );

        // Verify repo is clean — no MERGE_HEAD
        assert!(
            !p.join(".git").join("MERGE_HEAD").exists(),
            "MERGE_HEAD should not exist after skip"
        );

        // ── Reset for Test 2: Handler panics ──
        // Reset main to before session-a was merged
        Command::new("git")
            .args(["reset", "--hard", "HEAD~1"])
            .current_dir(p)
            .output()
            .unwrap();

        let sessions2 = vec![
            make_session("session-a", "assay/session-a", vec!["shared.rs".into()], 0),
            make_session("session-b", "assay/session-b", vec!["shared.rs".into()], 1),
        ];

        let config2 = MergeRunnerConfig {
            strategy: MergeStrategy::CompletionTime,
            project_root: p.to_path_buf(),
            base_branch: "main".to_string(),
            conflict_resolution_enabled: true,
        };

        let panic_handler = |_name: &str,
                             _files: &[String],
                             _scan: &ConflictScan,
                             _dir: &Path|
         -> ConflictResolutionResult {
            panic!("handler exploded!");
        };

        let report2 = merge_completed_sessions(sessions2, &config2, panic_handler).unwrap();

        assert_eq!(report2.sessions_merged, 1, "only A should merge cleanly");
        assert_eq!(
            report2.conflict_skipped, 1,
            "B should be conflict-skipped (panic caught)"
        );
        assert_eq!(
            report2.results[1].status,
            MergeSessionStatus::ConflictSkipped
        );
        assert!(
            report2.results[1]
                .error
                .as_ref()
                .unwrap()
                .contains("panicked"),
            "error should mention panic: {:?}",
            report2.results[1].error
        );

        // Verify repo is still clean after panic
        assert!(
            !p.join(".git").join("MERGE_HEAD").exists(),
            "MERGE_HEAD should not exist after panic recovery"
        );

        // Verify working tree is clean
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(p)
            .output()
            .unwrap();
        let status = String::from_utf8_lossy(&status_output.stdout)
            .trim()
            .to_string();
        assert!(
            status.is_empty(),
            "working tree should be clean after panic recovery, got: {status}"
        );
    }
}
