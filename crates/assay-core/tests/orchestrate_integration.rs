//! End-to-end integration tests for the orchestrated multi-session pipeline.
//!
//! These tests exercise the full path: DAG validation → parallel execution →
//! sequential merge → status persistence. Uses mock session runners (not real
//! agents) with **real git repos** (tempfile + `git init`).
//!
//! Proves R020 (multi-session orchestration) and R021 (sequential merge) at
//! the integration level.

#![cfg(feature = "orchestrate")]

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use assay_core::orchestrate::conflict_resolver::ConflictResolutionResult;
use assay_core::orchestrate::executor::{
    OrchestratorConfig, OrchestratorResult, SessionOutcome, run_orchestrated,
};
use assay_core::orchestrate::merge_runner::{
    MergeRunnerConfig, default_conflict_handler, extract_completed_sessions,
    merge_completed_sessions,
};
use assay_core::pipeline::{PipelineConfig, PipelineError, PipelineResult, PipelineStage};
use assay_types::orchestrate::{
    FailurePolicy, OrchestratorPhase, OrchestratorStatus, SessionRunState,
};
use assay_types::{
    ConflictAction, ConflictFileContent, ConflictResolution, ManifestSession, MergeSessionStatus,
    MergeStrategy, RunManifest,
};

// ── Helpers ──────────────────────────────────────────────────────────

/// Create a temp git repo with an initial commit on `main`, a `.assay` dir,
/// and a minimal `config.toml`.
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

    // Create .assay directory structure
    let assay_dir = p.join(".assay");
    std::fs::create_dir_all(assay_dir.join("specs")).unwrap();
    std::fs::write(
        assay_dir.join("config.toml"),
        r#"project_name = "integration-test""#,
    )
    .unwrap();

    // Ignore orchestrator state so the merge runner sees a clean worktree.
    std::fs::write(assay_dir.join(".gitignore"), "orchestrator/\n").unwrap();

    // Initial commit
    std::fs::write(p.join("readme.md"), "# integration test\n").unwrap();
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

/// Build a `PipelineConfig` pointing at the temp repo.
fn make_pipeline_config(tmp: &Path) -> PipelineConfig {
    PipelineConfig {
        project_root: tmp.to_path_buf(),
        assay_dir: tmp.join(".assay"),
        specs_dir: tmp.join(".assay/specs"),
        worktree_base: tmp.to_path_buf(),
        timeout_secs: 60,
        base_branch: Some("main".to_string()),
    }
}

/// Build a `RunManifest` from a list of (spec, name, depends_on) tuples.
fn make_manifest(sessions: Vec<(&str, Option<&str>, Vec<&str>)>) -> RunManifest {
    RunManifest {
        sessions: sessions
            .into_iter()
            .map(|(spec, name, deps)| ManifestSession {
                spec: spec.to_string(),
                name: name.map(|n| n.to_string()),
                settings: None,
                hooks: vec![],
                prompt_layers: vec![],
                file_scope: vec![],
                shared_files: vec![],
                depends_on: deps.into_iter().map(|d| d.to_string()).collect(),
            })
            .collect(),
        mode: assay_types::OrchestratorMode::Dag,
        mesh_config: None,
        gossip_config: None,
        state_backend: None,
    }
}

/// Mock session runner that creates a git branch in the real repo, writes a
/// unique file, commits, and returns a success `PipelineResult`.
///
/// The branch is named `assay/<session_name>` and the file is `<session_name>.txt`.
fn mock_success_runner(
    session: &ManifestSession,
    config: &PipelineConfig,
) -> Result<PipelineResult, PipelineError> {
    let name = session.name.clone().unwrap_or_else(|| session.spec.clone());
    let repo = &config.project_root;
    let branch = format!("assay/{name}");

    // Create branch from main
    let out = Command::new("git")
        .args(["checkout", "-b", &branch, "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    if !out.status.success() {
        return Err(PipelineError {
            stage: PipelineStage::AgentLaunch,
            message: format!(
                "git checkout -b failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
            recovery: "check branch name".to_string(),
            elapsed: Duration::from_millis(1),
        });
    }

    // Write a unique file
    let filename = format!("{name}.txt");
    std::fs::write(
        repo.join(&filename),
        format!("content from session {name}\n"),
    )
    .unwrap();

    // Only stage the specific file — `git add .` would stage untracked
    // .assay/orchestrator/ files and `git checkout main` would then delete them.
    Command::new("git")
        .args(["add", &filename])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", &format!("work from session {name}")])
        .current_dir(repo)
        .output()
        .unwrap();

    // Switch back to main so other sessions can branch from it
    Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();

    Ok(PipelineResult {
        session_id: format!("sess-{name}"),
        spec_name: session.spec.clone(),
        gate_summary: None,
        merge_check: None,
        stage_timings: vec![],
        outcome: assay_core::pipeline::PipelineOutcome::Success,
    })
}

// ── Test 1: 3-session manifest with mixed deps — full DAG → execute → merge ──

#[test]
fn three_session_dag_execute_merge_end_to_end() {
    let dir = setup_git_repo();
    let repo = dir.path();
    let pipeline_config = make_pipeline_config(repo);

    // Manifest: A (no deps), B (depends on A), C (no deps)
    let manifest = make_manifest(vec![
        ("spec-a", Some("A"), vec![]),
        ("spec-b", Some("B"), vec!["A"]),
        ("spec-c", Some("C"), vec![]),
    ]);

    let config = OrchestratorConfig {
        max_concurrency: 1, // serialize for deterministic git operations
        failure_policy: FailurePolicy::SkipDependents,
        ..Default::default()
    };

    // Use a mutex to serialize git operations (mock runner does real git)
    let git_mutex = std::sync::Mutex::new(());
    let runner = |session: &ManifestSession, pconfig: &PipelineConfig| {
        let _lock = git_mutex.lock().unwrap();
        mock_success_runner(session, pconfig)
    };

    // Phase 1: Execute
    let result: OrchestratorResult =
        run_orchestrated(&manifest, config, &pipeline_config, &runner).unwrap();

    // All 3 should complete
    assert_eq!(result.outcomes.len(), 3);
    assert!(
        result
            .outcomes
            .iter()
            .all(|(_, o)| matches!(o, SessionOutcome::Completed { .. })),
        "all sessions should complete"
    );

    // Verify execution order: B must have run after A
    // (We used max_concurrency: 1, so the ordering is deterministic)

    // Phase 2: Checkout base branch
    let out = Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success(), "checkout main should succeed");

    // Phase 3: Extract completed sessions and merge
    let completed = extract_completed_sessions(&result.outcomes);
    assert_eq!(completed.len(), 3, "all 3 should be extractable");

    let merge_config = MergeRunnerConfig {
        strategy: MergeStrategy::CompletionTime,
        project_root: repo.to_path_buf(),
        base_branch: "main".to_string(),
        conflict_resolution_enabled: false,
    };

    let merge_report =
        merge_completed_sessions(completed, &merge_config, default_conflict_handler()).unwrap();

    // Assert merge results
    assert_eq!(merge_report.sessions_merged, 3, "all 3 should merge");
    assert_eq!(merge_report.conflict_skipped, 0, "no conflicts expected");
    assert_eq!(merge_report.aborted, 0, "no aborts expected");
    assert_eq!(merge_report.results.len(), 3);

    for r in &merge_report.results {
        assert_eq!(
            r.status,
            MergeSessionStatus::Merged,
            "session {} should be merged",
            r.session_name
        );
        assert!(
            r.merge_sha.is_some(),
            "session {} should have a merge SHA",
            r.session_name
        );
    }

    // Verify all files present on main after merge
    assert!(
        repo.join("A.txt").exists(),
        "A.txt should exist on main after merge"
    );
    assert!(
        repo.join("B.txt").exists(),
        "B.txt should exist on main after merge"
    );
    assert!(
        repo.join("C.txt").exists(),
        "C.txt should exist on main after merge"
    );

    // Verify file contents
    let a_content = std::fs::read_to_string(repo.join("A.txt")).unwrap();
    assert!(a_content.contains("content from session A"));
    let b_content = std::fs::read_to_string(repo.join("B.txt")).unwrap();
    assert!(b_content.contains("content from session B"));
    let c_content = std::fs::read_to_string(repo.join("C.txt")).unwrap();
    assert!(c_content.contains("content from session C"));

    // Verify state.json persisted under .assay/orchestrator/<run_id>/
    let state_path = repo
        .join(".assay/orchestrator")
        .join(&result.run_id)
        .join("state.json");
    assert!(state_path.exists(), "state.json should exist");
    let status: OrchestratorStatus =
        serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(status.phase, OrchestratorPhase::Completed);
    assert_eq!(status.sessions.len(), 3);
    assert!(
        status
            .sessions
            .iter()
            .all(|s| s.state == SessionRunState::Completed)
    );
}

// ── Test 2: Failure propagation — A fails, B skipped, C succeeds ─────

#[test]
fn failure_propagation_a_fails_b_skipped_c_succeeds() {
    let dir = setup_git_repo();
    let repo = dir.path();
    let pipeline_config = make_pipeline_config(repo);

    // Same manifest: A (no deps), B (depends on A), C (no deps)
    let manifest = make_manifest(vec![
        ("spec-a", Some("A"), vec![]),
        ("spec-b", Some("B"), vec!["A"]),
        ("spec-c", Some("C"), vec![]),
    ]);

    let config = OrchestratorConfig {
        max_concurrency: 1,
        failure_policy: FailurePolicy::SkipDependents,
        ..Default::default()
    };

    let git_mutex = std::sync::Mutex::new(());
    let runner = |session: &ManifestSession, pconfig: &PipelineConfig| {
        let name = session.name.clone().unwrap_or_else(|| session.spec.clone());
        if name == "A" {
            // A fails
            Err(PipelineError {
                stage: PipelineStage::AgentLaunch,
                message: "session A crashed".to_string(),
                recovery: "retry".to_string(),
                elapsed: Duration::from_millis(5),
            })
        } else {
            let _lock = git_mutex.lock().unwrap();
            mock_success_runner(session, pconfig)
        }
    };

    let result = run_orchestrated(&manifest, config, &pipeline_config, &runner).unwrap();

    // Find outcomes by name
    let find = |name: &str| {
        result
            .outcomes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, o)| o)
    };

    // A should be Failed
    assert!(
        matches!(find("A"), Some(SessionOutcome::Failed { .. })),
        "A should be Failed"
    );

    // B should be Skipped (upstream A failed)
    match find("B") {
        Some(SessionOutcome::Skipped { reason }) => {
            assert!(
                reason.contains("A"),
                "skip reason should mention A, got: {reason}"
            );
        }
        other => panic!("B should be Skipped, got: {other:?}"),
    }

    // C should be Completed (independent)
    assert!(
        matches!(find("C"), Some(SessionOutcome::Completed { .. })),
        "C should be Completed"
    );

    // Merge phase: only C's branch should exist and merge
    let out = Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(out.status.success());

    let completed = extract_completed_sessions(&result.outcomes);
    assert_eq!(completed.len(), 1, "only C should be extractable");
    assert_eq!(completed[0].session_name, "C");

    let merge_config = MergeRunnerConfig {
        strategy: MergeStrategy::CompletionTime,
        project_root: repo.to_path_buf(),
        base_branch: "main".to_string(),
        conflict_resolution_enabled: false,
    };

    let merge_report =
        merge_completed_sessions(completed, &merge_config, default_conflict_handler()).unwrap();

    assert_eq!(merge_report.sessions_merged, 1, "only C should merge");
    assert_eq!(merge_report.results.len(), 1);
    assert_eq!(merge_report.results[0].session_name, "C");
    assert_eq!(merge_report.results[0].status, MergeSessionStatus::Merged);

    // C.txt should exist, A.txt and B.txt should not
    assert!(
        repo.join("C.txt").exists(),
        "C.txt should exist after merge"
    );
    assert!(
        !repo.join("A.txt").exists(),
        "A.txt should NOT exist (A failed)"
    );
    assert!(
        !repo.join("B.txt").exists(),
        "B.txt should NOT exist (B skipped)"
    );

    // Verify state.json records correct outcomes
    let state_path = repo
        .join(".assay/orchestrator")
        .join(&result.run_id)
        .join("state.json");
    let status: OrchestratorStatus =
        serde_json::from_str(&std::fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(status.phase, OrchestratorPhase::PartialFailure);

    let a_status = status.sessions.iter().find(|s| s.name == "A").unwrap();
    assert_eq!(a_status.state, SessionRunState::Failed);
    assert!(a_status.error.as_ref().unwrap().contains("crashed"));

    let b_status = status.sessions.iter().find(|s| s.name == "B").unwrap();
    assert_eq!(b_status.state, SessionRunState::Skipped);
    assert!(b_status.skip_reason.as_ref().unwrap().contains("A"));

    let c_status = status.sessions.iter().find(|s| s.name == "C").unwrap();
    assert_eq!(c_status.state, SessionRunState::Completed);
}

// ── Test 3: Status file persistence and readability ──────────────────

#[test]
fn status_persistence_round_trip() {
    let dir = setup_git_repo();
    let repo = dir.path();
    let pipeline_config = make_pipeline_config(repo);

    let manifest = make_manifest(vec![
        ("spec-a", Some("A"), vec![]),
        ("spec-b", Some("B"), vec![]),
    ]);

    let config = OrchestratorConfig {
        max_concurrency: 1,
        failure_policy: FailurePolicy::SkipDependents,
        ..Default::default()
    };

    let git_mutex = std::sync::Mutex::new(());
    let runner = |session: &ManifestSession, pconfig: &PipelineConfig| {
        let _lock = git_mutex.lock().unwrap();
        mock_success_runner(session, pconfig)
    };

    let result = run_orchestrated(&manifest, config, &pipeline_config, &runner).unwrap();

    // Read the persisted state.json
    let state_path = repo
        .join(".assay/orchestrator")
        .join(&result.run_id)
        .join("state.json");
    assert!(state_path.exists(), "state.json must exist");

    let raw_json = std::fs::read_to_string(&state_path).unwrap();
    let status: OrchestratorStatus = serde_json::from_str(&raw_json).unwrap();

    // Verify all top-level fields
    assert_eq!(status.run_id, result.run_id);
    assert_eq!(status.phase, OrchestratorPhase::Completed);
    assert_eq!(status.failure_policy, FailurePolicy::SkipDependents);
    assert!(status.completed_at.is_some());
    assert_eq!(status.sessions.len(), 2);

    // Verify per-session status fields
    for s in &status.sessions {
        assert_eq!(s.state, SessionRunState::Completed);
        assert!(s.started_at.is_some(), "started_at should be set");
        assert!(s.completed_at.is_some(), "completed_at should be set");
        assert!(s.duration_secs.is_some(), "duration_secs should be set");
        assert!(s.error.is_none(), "error should be None for success");
        assert!(s.skip_reason.is_none(), "skip_reason should be None");
    }

    // Verify the JSON is re-serializable (round-trip)
    let re_serialized = serde_json::to_string_pretty(&status).unwrap();
    let re_parsed: OrchestratorStatus = serde_json::from_str(&re_serialized).unwrap();
    assert_eq!(re_parsed.run_id, status.run_id);
    assert_eq!(re_parsed.phase, status.phase);
    assert_eq!(re_parsed.sessions.len(), status.sessions.len());
}

// ── Helpers for conflict tests ────────────────────────────────────────

/// Create a branch that modifies an existing file differently from main.
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

// ── Test 4: Audit trail — resolutions populated when handler returns audit ──

#[test]
fn test_merge_resolutions_audit_trail() {
    let dir = setup_git_repo();
    let repo = dir.path();

    // Create shared.rs on main so both sessions conflict
    std::fs::write(repo.join("shared.rs"), "fn shared() { /* original */ }\n").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add shared.rs"])
        .current_dir(repo)
        .output()
        .unwrap();

    create_branch_modifying_file(
        repo,
        "assay/session-a",
        "shared.rs",
        "fn shared() { /* version A */ }\n",
    );
    create_branch_modifying_file(
        repo,
        "assay/session-b",
        "shared.rs",
        "fn shared() { /* version B */ }\n",
    );

    let t0 = chrono::DateTime::from_timestamp(0, 0).unwrap();
    let sessions = vec![
        assay_core::orchestrate::ordering::CompletedSession {
            session_name: "session-a".to_string(),
            branch_name: "assay/session-a".to_string(),
            changed_files: vec!["shared.rs".to_string()],
            completed_at: t0,
            topo_order: 0,
        },
        assay_core::orchestrate::ordering::CompletedSession {
            session_name: "session-b".to_string(),
            branch_name: "assay/session-b".to_string(),
            changed_files: vec!["shared.rs".to_string()],
            completed_at: t0 + chrono::Duration::seconds(1),
            topo_order: 1,
        },
    ];

    let merge_config = MergeRunnerConfig {
        strategy: MergeStrategy::CompletionTime,
        project_root: repo.to_path_buf(),
        base_branch: "main".to_string(),
        conflict_resolution_enabled: true,
    };

    // Scripted handler: captures original, strips markers, returns full audit record
    let resolver_handler = |name: &str,
                            files: &[String],
                            _scan: &assay_types::ConflictScan,
                            work_dir: &Path|
     -> ConflictResolutionResult {
        let shared_path = work_dir.join("shared.rs");
        let original = std::fs::read_to_string(&shared_path).unwrap();

        // Strip conflict markers
        let mut resolved = String::new();
        for line in original.lines() {
            if line.starts_with("<<<<<<<")
                || line.starts_with("=======")
                || line.starts_with(">>>>>>>")
            {
                continue;
            }
            resolved.push_str(line);
            resolved.push('\n');
        }

        std::fs::write(&shared_path, &resolved).unwrap();

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
            audit: Some(ConflictResolution {
                session_name: name.to_string(),
                conflicting_files: files.to_vec(),
                original_contents: vec![ConflictFileContent {
                    path: "shared.rs".into(),
                    content: original.clone(),
                }],
                resolved_contents: vec![ConflictFileContent {
                    path: "shared.rs".into(),
                    content: resolved.clone(),
                }],
                resolver_stdout: "scripted".to_string(),
                validation_passed: None,
            }),
            repo_clean: false,
        }
    };

    let report = merge_completed_sessions(sessions, &merge_config, resolver_handler).unwrap();

    // Check merge counts
    assert_eq!(report.sessions_merged, 2, "both sessions should merge");
    assert_eq!(report.conflict_skipped, 0, "no skips");

    // The conflict occurs on session-b (session-a merges cleanly, session-b conflicts)
    assert_eq!(report.resolutions.len(), 1, "one resolution audit record");
    let res = &report.resolutions[0];
    assert_eq!(res.session_name, "session-b");
    assert_eq!(res.conflicting_files, vec!["shared.rs".to_string()]);
    assert!(
        res.original_contents[0].content.contains("<<<<<<<"),
        "original content should contain conflict markers, got: {}",
        res.original_contents[0].content
    );
    assert!(
        !res.resolved_contents[0].content.contains("<<<<<<<"),
        "resolved content should not contain conflict markers"
    );
    assert_eq!(res.resolver_stdout, "scripted");
}

// ── Test 5: Skip handler leaves resolutions empty ─────────────────────

#[test]
fn test_merge_skip_leaves_empty_resolutions() {
    let dir = setup_git_repo();
    let repo = dir.path();

    // Three sessions, each writes its own unique file — no conflicts
    for (branch, file) in [
        ("assay/session-x", "x.txt"),
        ("assay/session-y", "y.txt"),
        ("assay/session-z", "z.txt"),
    ] {
        Command::new("git")
            .args(["checkout", "-b", branch, "main"])
            .current_dir(repo)
            .output()
            .unwrap();
        std::fs::write(repo.join(file), format!("content of {file}\n")).unwrap();
        Command::new("git")
            .args(["add", file])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", &format!("add {file}")])
            .current_dir(repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(repo)
            .output()
            .unwrap();
    }

    let t0 = chrono::DateTime::from_timestamp(0, 0).unwrap();
    let sessions = vec![
        assay_core::orchestrate::ordering::CompletedSession {
            session_name: "session-x".to_string(),
            branch_name: "assay/session-x".to_string(),
            changed_files: vec!["x.txt".to_string()],
            completed_at: t0,
            topo_order: 0,
        },
        assay_core::orchestrate::ordering::CompletedSession {
            session_name: "session-y".to_string(),
            branch_name: "assay/session-y".to_string(),
            changed_files: vec!["y.txt".to_string()],
            completed_at: t0 + chrono::Duration::seconds(1),
            topo_order: 0,
        },
        assay_core::orchestrate::ordering::CompletedSession {
            session_name: "session-z".to_string(),
            branch_name: "assay/session-z".to_string(),
            changed_files: vec!["z.txt".to_string()],
            completed_at: t0 + chrono::Duration::seconds(2),
            topo_order: 0,
        },
    ];

    let merge_config = MergeRunnerConfig {
        strategy: MergeStrategy::CompletionTime,
        project_root: repo.to_path_buf(),
        base_branch: "main".to_string(),
        conflict_resolution_enabled: false,
    };

    let report =
        merge_completed_sessions(sessions, &merge_config, default_conflict_handler()).unwrap();

    assert_eq!(report.sessions_merged, 3, "all 3 should merge");
    assert_eq!(report.conflict_skipped, 0, "no conflicts expected");
    assert!(
        report.resolutions.is_empty(),
        "no resolutions for clean merges"
    );
}
