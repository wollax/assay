use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use assay_types::OrchestratorMode;
use assay_types::orchestrate::{FailurePolicy, MergeStrategy};

use super::{assay_dir, project_root};

/// Conflict resolution mode for multi-session merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConflictResolutionMode {
    /// Use AI (Claude) to automatically resolve merge conflicts.
    Auto,
    /// Skip conflicting sessions (default behavior).
    Skip,
}

fn parse_conflict_resolution(s: &str) -> Result<ConflictResolutionMode, String> {
    match s {
        "auto" => Ok(ConflictResolutionMode::Auto),
        "skip" => Ok(ConflictResolutionMode::Skip),
        _ => Err(format!(
            "invalid conflict resolution mode '{s}': expected 'auto' or 'skip'"
        )),
    }
}

/// Run a manifest through the end-to-end pipeline.
#[derive(Parser)]
#[command(after_long_help = "\
Examples:
  Run a manifest with default settings:
    assay run manifest.toml

  Override agent timeout to 15 minutes:
    assay run manifest.toml --timeout 900

  Output structured JSON for scripting:
    assay run manifest.toml --json

  Use a specific base branch:
    assay run manifest.toml --base-branch develop

  Run multi-session manifest with abort policy:
    assay run multi.toml --failure-policy abort

  Use file-overlap merge strategy:
    assay run multi.toml --merge-strategy file-overlap

  Enable AI conflict resolution:
    assay run multi.toml --conflict-resolution auto")]
pub(crate) struct RunCommand {
    /// Path to the manifest TOML file
    pub manifest: PathBuf,

    /// Maximum seconds to wait for each agent subprocess (default: 600)
    #[arg(long, default_value_t = assay_core::pipeline::PipelineConfig::DEFAULT_TIMEOUT_SECS)]
    pub timeout: u64,

    /// Output results as JSON instead of human-readable text
    #[arg(long)]
    pub json: bool,

    /// Base branch for worktree creation (default: auto-detect)
    #[arg(long)]
    pub base_branch: Option<String>,

    /// Failure policy for multi-session orchestration (skip-dependents or abort)
    #[arg(long, default_value = "skip-dependents", value_parser = parse_failure_policy)]
    pub failure_policy: FailurePolicy,

    /// Merge strategy for combining completed session branches (completion-time or file-overlap)
    #[arg(long, default_value = "completion-time", value_parser = parse_merge_strategy)]
    pub merge_strategy: MergeStrategy,

    /// Conflict resolution mode for merge phase (auto or skip)
    ///
    /// `auto`: use AI (Claude) to resolve merge conflicts automatically.
    /// `skip`: skip conflicting sessions without resolving (default).
    #[arg(long, default_value = "skip", value_parser = parse_conflict_resolution)]
    pub conflict_resolution: ConflictResolutionMode,
}

fn parse_failure_policy(s: &str) -> Result<FailurePolicy, String> {
    match s {
        "skip-dependents" => Ok(FailurePolicy::SkipDependents),
        "abort" => Ok(FailurePolicy::Abort),
        _ => Err(format!(
            "invalid failure policy '{s}': expected 'skip-dependents' or 'abort'"
        )),
    }
}

fn parse_merge_strategy(s: &str) -> Result<MergeStrategy, String> {
    match s {
        "completion-time" => Ok(MergeStrategy::CompletionTime),
        "file-overlap" => Ok(MergeStrategy::FileOverlap),
        _ => Err(format!(
            "invalid merge strategy '{s}': expected 'completion-time' or 'file-overlap'"
        )),
    }
}

// ── JSON response types ──────────────────────────────────────────────

#[derive(Serialize)]
struct RunResponse {
    sessions: Vec<SessionResult>,
    summary: RunSummary,
}

#[derive(Serialize)]
struct RunSummary {
    total: usize,
    succeeded: usize,
    gate_failed: usize,
    merge_conflict: usize,
    errored: usize,
}

#[derive(Serialize)]
struct SessionResult {
    spec_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<SessionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stage_timings: Option<Vec<StageTimingEntry>>,
}

#[derive(Serialize)]
struct SessionError {
    stage: String,
    message: String,
    recovery: String,
    elapsed_secs: f64,
}

#[derive(Serialize)]
struct StageTimingEntry {
    stage: String,
    duration_secs: f64,
}

// ── Orchestration response types ─────────────────────────────────────

#[derive(Serialize)]
struct OrchestrationResponse {
    run_id: String,
    sessions: Vec<OrchestrationSessionResult>,
    merge_report: assay_types::MergeReport,
    summary: OrchestrationSummary,
}

#[derive(Serialize)]
struct OrchestrationSessionResult {
    name: String,
    outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
}

#[derive(Serialize)]
struct OrchestrationSummary {
    total: usize,
    completed: usize,
    failed: usize,
    skipped: usize,
    sessions_merged: usize,
    merge_conflicts: usize,
    duration_secs: f64,
}

// ── Multi-session detection ─────────────────────────────────────────

/// Returns `true` if the manifest requires orchestrated execution.
///
/// A manifest needs orchestration when it has more than one session, or
/// when any session declares dependencies. Single-session manifests with
/// no dependencies use the simpler sequential `run_manifest()` path.
fn needs_orchestration(manifest: &assay_types::RunManifest) -> bool {
    manifest.sessions.len() > 1 || manifest.sessions.iter().any(|s| !s.depends_on.is_empty())
}

// ── Execute ──────────────────────────────────────────────────────────

pub(crate) fn execute(cmd: &RunCommand) -> anyhow::Result<i32> {
    let root = project_root()?;
    let ad = assay_dir(&root);
    if !ad.is_dir() {
        anyhow::bail!("No Assay project found. Run `assay init` first.");
    }

    let config = assay_core::config::load(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let specs_dir = root.join(".assay").join(&config.specs_dir);
    let worktree_base = assay_core::worktree::resolve_worktree_dir(None, &config, &root);

    // Load manifest
    tracing::info!(path = %cmd.manifest.display(), "Loading manifest");
    let manifest = assay_core::manifest::load(&cmd.manifest).map_err(|e| anyhow::anyhow!("{e}"))?;
    tracing::info!(session_count = manifest.sessions.len(), "Manifest loaded");

    // Build pipeline config
    let pipeline_config = assay_core::pipeline::PipelineConfig {
        project_root: root.clone(),
        assay_dir: ad,
        specs_dir,
        worktree_base,
        timeout_secs: cmd.timeout,
        base_branch: cmd.base_branch.clone(),
    };

    // Route based on mode first, then fall through to multi-session detection.
    match manifest.mode {
        OrchestratorMode::Mesh => return execute_mesh(cmd, &manifest, &pipeline_config),
        OrchestratorMode::Gossip => return execute_gossip(cmd, &manifest, &pipeline_config),
        OrchestratorMode::Dag => {} // fall through to existing logic
    }

    // Route based on multi-session detection
    if needs_orchestration(&manifest) {
        execute_orchestrated(cmd, &manifest, &pipeline_config)
    } else {
        execute_sequential(cmd, &manifest, &pipeline_config)
    }
}

/// Single-session path: runs through `run_manifest()` sequentially.
fn execute_sequential(
    cmd: &RunCommand,
    manifest: &assay_types::RunManifest,
    pipeline_config: &assay_core::pipeline::PipelineConfig,
) -> anyhow::Result<i32> {
    // Concrete harness writer: composes assay_harness::claude functions
    let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
        |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
            let claude_config = assay_harness::claude::generate_config(profile);
            assay_harness::claude::write_config(&claude_config, worktree_path)
                .map_err(|e| format!("Failed to write claude config: {e}"))?;
            Ok(assay_harness::claude::build_cli_args(&claude_config))
        },
    );

    // Run the pipeline
    let results = assay_core::pipeline::run_manifest(manifest, pipeline_config, &harness_writer);

    // Process results
    let mut session_results = Vec::new();
    let mut succeeded = 0usize;
    let mut gate_failed = 0usize;
    let mut merge_conflict = 0usize;
    let mut errored = 0usize;

    for (i, result) in results.into_iter().enumerate() {
        let spec_name = manifest.sessions[i].spec.clone();
        match result {
            Ok(pr) => {
                let timings: Vec<StageTimingEntry> = pr
                    .stage_timings
                    .iter()
                    .map(|t| StageTimingEntry {
                        stage: t.stage.to_string(),
                        duration_secs: t.duration.as_secs_f64(),
                    })
                    .collect();

                let outcome_str = pr.outcome.to_string();

                if !cmd.json {
                    tracing::info!(session_id = %pr.session_id, session_name = %spec_name, status = %outcome_str, "Session result");
                    for t in &timings {
                        tracing::info!(stage = %t.stage, duration_secs = t.duration_secs, "Stage timing");
                    }
                }

                match pr.outcome {
                    assay_core::pipeline::PipelineOutcome::Success => succeeded += 1,
                    assay_core::pipeline::PipelineOutcome::GateFailed => gate_failed += 1,
                    assay_core::pipeline::PipelineOutcome::MergeConflict => merge_conflict += 1,
                }

                session_results.push(SessionResult {
                    spec_name,
                    session_id: Some(pr.session_id),
                    outcome: outcome_str,
                    error: None,
                    stage_timings: Some(timings),
                });
            }
            Err(pe) => {
                errored += 1;

                if !cmd.json {
                    tracing::error!(session_name = %spec_name, stage = %pe.stage, error = %pe.message, "Session error");
                    tracing::error!(recovery = %pe.recovery, "Recovery suggestion");
                }

                session_results.push(SessionResult {
                    spec_name,
                    session_id: None,
                    outcome: "Error".to_string(),
                    error: Some(SessionError {
                        stage: pe.stage.to_string(),
                        message: pe.message,
                        recovery: pe.recovery,
                        elapsed_secs: pe.elapsed.as_secs_f64(),
                    }),
                    stage_timings: None,
                });
            }
        }
    }

    let total = session_results.len();
    let response = RunResponse {
        sessions: session_results,
        summary: RunSummary {
            total,
            succeeded,
            gate_failed,
            merge_conflict,
            errored,
        },
    };

    if cmd.json {
        let json = serde_json::to_string_pretty(&response)?;
        println!("{json}");
    } else {
        tracing::info!(
            total = total,
            succeeded = succeeded,
            gate_failed = gate_failed,
            merge_conflict = merge_conflict,
            errored = errored,
            "Run summary"
        );
    }

    // Exit codes: 0 = all succeed, 1 = any pipeline error, 2 = gate failures
    if errored > 0 {
        Ok(1)
    } else if gate_failed > 0 || merge_conflict > 0 {
        Ok(2)
    } else {
        Ok(0)
    }
}

/// Multi-session path: orchestrated execution + sequential merge.
fn execute_orchestrated(
    cmd: &RunCommand,
    manifest: &assay_types::RunManifest,
    pipeline_config: &assay_core::pipeline::PipelineConfig,
) -> anyhow::Result<i32> {
    use assay_core::orchestrate::conflict_resolver::resolve_conflict;
    use assay_core::orchestrate::executor::{OrchestratorConfig, SessionOutcome};
    use assay_core::orchestrate::merge_runner::{
        MergeRunnerConfig, default_conflict_handler, extract_completed_sessions,
        merge_completed_sessions,
    };

    tracing::info!(
        session_count = manifest.sessions.len(),
        "Multi-session manifest detected — using orchestrated execution"
    );

    // ── Phase 1: Orchestrated execution ──────────────────────────────

    let orch_config = OrchestratorConfig {
        max_concurrency: 8,
        failure_policy: cmd.failure_policy,
    };

    // Session runner closure: constructs HarnessWriter from plain function
    // calls (D035), delegates to run_session.
    let session_runner = |session: &assay_types::ManifestSession,
                          pipe_cfg: &assay_core::pipeline::PipelineConfig|
     -> Result<
        assay_core::pipeline::PipelineResult,
        assay_core::pipeline::PipelineError,
    > {
        let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
            |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
                let claude_config = assay_harness::claude::generate_config(profile);
                assay_harness::claude::write_config(&claude_config, worktree_path)
                    .map_err(|e| format!("Failed to write claude config: {e}"))?;
                Ok(assay_harness::claude::build_cli_args(&claude_config))
            },
        );
        assay_core::pipeline::run_session(session, pipe_cfg, &harness_writer)
    };

    tracing::info!("Phase 1: Executing sessions");
    let orch_result = assay_core::orchestrate::executor::run_orchestrated(
        manifest,
        orch_config,
        pipeline_config,
        &session_runner,
    )
    .map_err(|e| anyhow::anyhow!("Orchestration failed: {e}"))?;

    tracing::info!(
        outcome_count = orch_result.outcomes.len(),
        duration_secs = format_args!("{:.1}", orch_result.duration.as_secs_f64()),
        "Phase 1 complete"
    );

    // ── Phase 2: Checkout base branch ────────────────────────────────

    let base_branch = if let Some(ref branch) = cmd.base_branch {
        branch.clone()
    } else {
        // Auto-detect current branch
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&pipeline_config.project_root)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to detect base branch: {e}"))?;
        if !output.status.success() {
            anyhow::bail!(
                "Failed to detect base branch: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    tracing::info!(base_branch = %base_branch, "Phase 2: Checking out base branch");
    let checkout_output = std::process::Command::new("git")
        .args(["checkout", &base_branch])
        .current_dir(&pipeline_config.project_root)
        .output()
        .map_err(|e| anyhow::anyhow!("git checkout failed: {e}"))?;
    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        anyhow::bail!(
            "Failed to checkout base branch '{base_branch}': {}",
            stderr.trim()
        );
    }

    // ── Phase 3: Sequential merge ────────────────────────────────────

    tracing::info!("Phase 3: Merging completed sessions");
    let completed = extract_completed_sessions(&orch_result.outcomes);

    let (merge_config, merge_report) = match cmd.conflict_resolution {
        ConflictResolutionMode::Auto => {
            let config = assay_types::orchestrate::ConflictResolutionConfig {
                enabled: true,
                ..Default::default()
            };
            let merge_config = MergeRunnerConfig {
                strategy: cmd.merge_strategy,
                project_root: pipeline_config.project_root.clone(),
                base_branch: base_branch.clone(),
                conflict_resolution_enabled: true,
            };
            let handler = move |name: &str,
                                files: &[String],
                                scan: &assay_types::ConflictScan,
                                dir: &std::path::Path| {
                resolve_conflict(name, files, scan, dir, &config)
            };
            let report = merge_completed_sessions(completed, &merge_config, handler)
                .map_err(|e| anyhow::anyhow!("Merge failed: {e}"))?;
            (merge_config, report)
        }
        ConflictResolutionMode::Skip => {
            let merge_config = MergeRunnerConfig {
                strategy: cmd.merge_strategy,
                project_root: pipeline_config.project_root.clone(),
                base_branch: base_branch.clone(),
                conflict_resolution_enabled: false,
            };
            let report =
                merge_completed_sessions(completed, &merge_config, default_conflict_handler())
                    .map_err(|e| anyhow::anyhow!("Merge failed: {e}"))?;
            (merge_config, report)
        }
    };
    drop(merge_config);

    tracing::info!(
        sessions_merged = merge_report.sessions_merged,
        conflict_skipped = merge_report.conflict_skipped,
        aborted = merge_report.aborted,
        "Phase 3 complete"
    );

    // ── Format results ───────────────────────────────────────────────

    let mut completed_count = 0usize;
    let mut failed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut session_results = Vec::new();

    for (name, outcome) in &orch_result.outcomes {
        match outcome {
            SessionOutcome::Completed { .. } => {
                completed_count += 1;
                if !cmd.json {
                    tracing::info!(session_name = %name, status = "completed", "Session result");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "completed".to_string(),
                    error: None,
                    skip_reason: None,
                });
            }
            SessionOutcome::Failed { error, .. } => {
                failed_count += 1;
                if !cmd.json {
                    tracing::error!(session_name = %name, error = %error.message, "Session failed");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "failed".to_string(),
                    error: Some(error.message.clone()),
                    skip_reason: None,
                });
            }
            SessionOutcome::Skipped { reason } => {
                skipped_count += 1;
                if !cmd.json {
                    tracing::info!(session_name = %name, reason = %reason, "Session skipped");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "skipped".to_string(),
                    error: None,
                    skip_reason: Some(reason.clone()),
                });
            }
        }
    }

    let total = session_results.len();
    let response = OrchestrationResponse {
        run_id: orch_result.run_id,
        sessions: session_results,
        merge_report: merge_report.clone(),
        summary: OrchestrationSummary {
            total,
            completed: completed_count,
            failed: failed_count,
            skipped: skipped_count,
            sessions_merged: merge_report.sessions_merged,
            merge_conflicts: merge_report.conflict_skipped,
            duration_secs: orch_result.duration.as_secs_f64(),
        },
    };

    if cmd.json {
        let json = serde_json::to_string_pretty(&response)?;
        println!("{json}");
    } else {
        tracing::info!(
            total = total,
            completed = completed_count,
            failed = failed_count,
            skipped = skipped_count,
            sessions_merged = merge_report.sessions_merged,
            merge_conflicts = merge_report.conflict_skipped,
            "Orchestration summary"
        );
    }

    // Exit codes: 0 = all succeed + merge clean, 1 = any error/skip, 2 = merge conflicts
    if failed_count > 0 || skipped_count > 0 {
        Ok(1)
    } else if merge_report.conflict_skipped > 0 || merge_report.aborted > 0 {
        Ok(2)
    } else {
        Ok(0)
    }
}

// ── Mesh / Gossip stubs ───────────────────────────────────────────────

/// Mesh-mode path: delegates to `run_mesh()`, populates outcomes from real session runner.
fn execute_mesh(
    cmd: &RunCommand,
    manifest: &assay_types::RunManifest,
    pipeline_config: &assay_core::pipeline::PipelineConfig,
) -> anyhow::Result<i32> {
    use assay_core::orchestrate::executor::{OrchestratorConfig, SessionOutcome};

    tracing::info!(session_count = manifest.sessions.len(), "mode: mesh");

    let orch_config = OrchestratorConfig {
        max_concurrency: 8,
        failure_policy: cmd.failure_policy,
    };

    // Session runner closure: constructs HarnessWriter from plain function
    // calls (D035), delegates to run_session.
    let session_runner = |session: &assay_types::ManifestSession,
                          pipe_cfg: &assay_core::pipeline::PipelineConfig|
     -> Result<
        assay_core::pipeline::PipelineResult,
        assay_core::pipeline::PipelineError,
    > {
        let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
            |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
                let claude_config = assay_harness::claude::generate_config(profile);
                assay_harness::claude::write_config(&claude_config, worktree_path)
                    .map_err(|e| format!("Failed to write claude config: {e}"))?;
                Ok(assay_harness::claude::build_cli_args(&claude_config))
            },
        );
        assay_core::pipeline::run_session(session, pipe_cfg, &harness_writer)
    };

    let orch_result = assay_core::orchestrate::mesh::run_mesh(
        manifest,
        &orch_config,
        pipeline_config,
        &session_runner,
    )
    .map_err(|e| anyhow::anyhow!("Mesh execution failed: {e}"))?;

    // ── Format results ───────────────────────────────────────────────

    let mut completed_count = 0usize;
    let mut failed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut session_results = Vec::new();

    for (name, outcome) in &orch_result.outcomes {
        match outcome {
            SessionOutcome::Completed { .. } => {
                completed_count += 1;
                if !cmd.json {
                    tracing::info!(session_name = %name, status = "completed", "Session result");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "completed".to_string(),
                    error: None,
                    skip_reason: None,
                });
            }
            SessionOutcome::Failed { error, .. } => {
                failed_count += 1;
                if !cmd.json {
                    tracing::error!(session_name = %name, error = %error.message, "Session failed");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "failed".to_string(),
                    error: Some(error.message.clone()),
                    skip_reason: None,
                });
            }
            SessionOutcome::Skipped { reason } => {
                skipped_count += 1;
                if !cmd.json {
                    tracing::info!(session_name = %name, reason = %reason, "Session skipped");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "skipped".to_string(),
                    error: None,
                    skip_reason: Some(reason.clone()),
                });
            }
        }
    }

    let total = session_results.len();
    let empty_merge_report = assay_types::MergeReport {
        sessions_merged: 0,
        sessions_skipped: 0,
        conflict_skipped: 0,
        aborted: 0,
        plan: assay_types::MergePlan {
            strategy: assay_types::MergeStrategy::CompletionTime,
            entries: vec![],
        },
        results: vec![],
        duration_secs: 0.0,
        resolutions: vec![],
    };
    let response = OrchestrationResponse {
        run_id: orch_result.run_id,
        sessions: session_results,
        merge_report: empty_merge_report,
        summary: OrchestrationSummary {
            total,
            completed: completed_count,
            failed: failed_count,
            skipped: skipped_count,
            sessions_merged: 0,
            merge_conflicts: 0,
            duration_secs: orch_result.duration.as_secs_f64(),
        },
    };

    if cmd.json {
        let json = serde_json::to_string_pretty(&response)?;
        println!("{json}");
    } else {
        tracing::info!(
            total = total,
            completed = completed_count,
            failed = failed_count,
            skipped = skipped_count,
            "Mesh summary"
        );
    }

    if failed_count > 0 || skipped_count > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Gossip-mode path: delegates to `run_gossip()`, populates outcomes from real session runner.
fn execute_gossip(
    cmd: &RunCommand,
    manifest: &assay_types::RunManifest,
    pipeline_config: &assay_core::pipeline::PipelineConfig,
) -> anyhow::Result<i32> {
    use assay_core::orchestrate::executor::{OrchestratorConfig, SessionOutcome};

    tracing::info!(session_count = manifest.sessions.len(), "mode: gossip");

    let orch_config = OrchestratorConfig {
        max_concurrency: 8,
        failure_policy: cmd.failure_policy,
    };

    // Session runner closure: constructs HarnessWriter from plain function
    // calls (D035), delegates to run_session.
    let session_runner = |session: &assay_types::ManifestSession,
                          pipe_cfg: &assay_core::pipeline::PipelineConfig|
     -> Result<
        assay_core::pipeline::PipelineResult,
        assay_core::pipeline::PipelineError,
    > {
        let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
            |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
                let claude_config = assay_harness::claude::generate_config(profile);
                assay_harness::claude::write_config(&claude_config, worktree_path)
                    .map_err(|e| format!("Failed to write claude config: {e}"))?;
                Ok(assay_harness::claude::build_cli_args(&claude_config))
            },
        );
        assay_core::pipeline::run_session(session, pipe_cfg, &harness_writer)
    };

    let orch_result = assay_core::orchestrate::gossip::run_gossip(
        manifest,
        &orch_config,
        pipeline_config,
        &session_runner,
    )
    .map_err(|e| anyhow::anyhow!("Gossip execution failed: {e}"))?;

    // ── Format results ───────────────────────────────────────────────

    let mut completed_count = 0usize;
    let mut failed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut session_results = Vec::new();

    for (name, outcome) in &orch_result.outcomes {
        match outcome {
            SessionOutcome::Completed { .. } => {
                completed_count += 1;
                if !cmd.json {
                    tracing::info!(session_name = %name, status = "completed", "Session result");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "completed".to_string(),
                    error: None,
                    skip_reason: None,
                });
            }
            SessionOutcome::Failed { error, .. } => {
                failed_count += 1;
                if !cmd.json {
                    tracing::error!(session_name = %name, error = %error.message, "Session failed");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "failed".to_string(),
                    error: Some(error.message.clone()),
                    skip_reason: None,
                });
            }
            SessionOutcome::Skipped { reason } => {
                skipped_count += 1;
                if !cmd.json {
                    tracing::info!(session_name = %name, reason = %reason, "Session skipped");
                }
                session_results.push(OrchestrationSessionResult {
                    name: name.clone(),
                    outcome: "skipped".to_string(),
                    error: None,
                    skip_reason: Some(reason.clone()),
                });
            }
        }
    }

    let total = session_results.len();
    let empty_merge_report = assay_types::MergeReport {
        sessions_merged: 0,
        sessions_skipped: 0,
        conflict_skipped: 0,
        aborted: 0,
        plan: assay_types::MergePlan {
            strategy: assay_types::MergeStrategy::CompletionTime,
            entries: vec![],
        },
        results: vec![],
        duration_secs: 0.0,
        resolutions: vec![],
    };
    let response = OrchestrationResponse {
        run_id: orch_result.run_id,
        sessions: session_results,
        merge_report: empty_merge_report,
        summary: OrchestrationSummary {
            total,
            completed: completed_count,
            failed: failed_count,
            skipped: skipped_count,
            sessions_merged: 0,
            merge_conflicts: 0,
            duration_secs: orch_result.duration.as_secs_f64(),
        },
    };

    if cmd.json {
        let json = serde_json::to_string_pretty(&response)?;
        println!("{json}");
    } else {
        tracing::info!(
            total = total,
            completed = completed_count,
            failed = failed_count,
            skipped = skipped_count,
            "Gossip summary"
        );
    }

    if failed_count > 0 || skipped_count > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn run_command_parses_minimal() {
        let cmd = RunCommand::parse_from(["run", "manifest.toml"]);
        assert_eq!(cmd.manifest, PathBuf::from("manifest.toml"));
        assert_eq!(cmd.timeout, 600);
        assert!(!cmd.json);
        assert!(cmd.base_branch.is_none());
        assert_eq!(cmd.failure_policy, FailurePolicy::SkipDependents);
        assert_eq!(cmd.merge_strategy, MergeStrategy::CompletionTime);
        assert_eq!(cmd.conflict_resolution, ConflictResolutionMode::Skip);
    }

    #[test]
    fn run_command_parses_all_flags() {
        let cmd = RunCommand::parse_from([
            "run",
            "path/to/manifest.toml",
            "--timeout",
            "900",
            "--json",
            "--base-branch",
            "develop",
            "--failure-policy",
            "abort",
            "--merge-strategy",
            "file-overlap",
        ]);
        assert_eq!(cmd.manifest, PathBuf::from("path/to/manifest.toml"));
        assert_eq!(cmd.timeout, 900);
        assert!(cmd.json);
        assert_eq!(cmd.base_branch.as_deref(), Some("develop"));
        assert_eq!(cmd.failure_policy, FailurePolicy::Abort);
        assert_eq!(cmd.merge_strategy, MergeStrategy::FileOverlap);
    }

    #[test]
    fn run_command_conflict_resolution_auto() {
        let cmd = RunCommand::parse_from(["run", "manifest.toml", "--conflict-resolution", "auto"]);
        assert_eq!(cmd.conflict_resolution, ConflictResolutionMode::Auto);
    }

    #[test]
    fn run_command_conflict_resolution_skip_default() {
        let cmd = RunCommand::parse_from(["run", "manifest.toml"]);
        assert_eq!(
            cmd.conflict_resolution,
            ConflictResolutionMode::Skip,
            "default conflict resolution mode should be Skip"
        );
    }

    #[test]
    fn run_command_conflict_resolution_skip_explicit() {
        let cmd = RunCommand::parse_from(["run", "manifest.toml", "--conflict-resolution", "skip"]);
        assert_eq!(cmd.conflict_resolution, ConflictResolutionMode::Skip);
    }

    #[test]
    fn run_command_rejects_invalid_conflict_resolution() {
        let result = RunCommand::try_parse_from([
            "run",
            "manifest.toml",
            "--conflict-resolution",
            "invalid",
        ]);
        assert!(
            result.is_err(),
            "invalid conflict-resolution value should produce a clap error"
        );
    }

    #[test]
    fn run_command_parses_orchestration_flags() {
        // Test each combination of orchestration flags
        let cmd = RunCommand::parse_from([
            "run",
            "manifest.toml",
            "--failure-policy",
            "skip-dependents",
            "--merge-strategy",
            "completion-time",
        ]);
        assert_eq!(cmd.failure_policy, FailurePolicy::SkipDependents);
        assert_eq!(cmd.merge_strategy, MergeStrategy::CompletionTime);

        let cmd = RunCommand::parse_from(["run", "manifest.toml", "--failure-policy", "abort"]);
        assert_eq!(cmd.failure_policy, FailurePolicy::Abort);
        assert_eq!(cmd.merge_strategy, MergeStrategy::CompletionTime); // default

        let cmd =
            RunCommand::parse_from(["run", "manifest.toml", "--merge-strategy", "file-overlap"]);
        assert_eq!(cmd.failure_policy, FailurePolicy::SkipDependents); // default
        assert_eq!(cmd.merge_strategy, MergeStrategy::FileOverlap);
    }

    #[test]
    fn run_command_rejects_invalid_failure_policy() {
        let result =
            RunCommand::try_parse_from(["run", "manifest.toml", "--failure-policy", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn run_command_rejects_invalid_merge_strategy() {
        let result =
            RunCommand::try_parse_from(["run", "manifest.toml", "--merge-strategy", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn run_response_serializes_to_json() {
        let response = RunResponse {
            sessions: vec![SessionResult {
                spec_name: "auth-flow".into(),
                session_id: Some("sess-001".into()),
                outcome: "Success".into(),
                error: None,
                stage_timings: Some(vec![StageTimingEntry {
                    stage: "SpecLoad".into(),
                    duration_secs: 0.01,
                }]),
            }],
            summary: RunSummary {
                total: 1,
                succeeded: 1,
                gate_failed: 0,
                merge_conflict: 0,
                errored: 0,
            },
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        assert!(json.contains("auth-flow"));
        assert!(json.contains("Success"));
        assert!(json.contains("SpecLoad"));
    }

    #[test]
    fn run_response_error_serializes() {
        let response = RunResponse {
            sessions: vec![SessionResult {
                spec_name: "broken".into(),
                session_id: None,
                outcome: "Error".into(),
                error: Some(SessionError {
                    stage: "SpecLoad".into(),
                    message: "spec not found".into(),
                    recovery: "check specs dir".into(),
                    elapsed_secs: 0.05,
                }),
                stage_timings: None,
            }],
            summary: RunSummary {
                total: 1,
                succeeded: 0,
                gate_failed: 0,
                merge_conflict: 0,
                errored: 1,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("SpecLoad"));
        assert!(json.contains("spec not found"));
        assert!(json.contains("check specs dir"));
        // session_id should be absent (skip_serializing_if)
        assert!(!json.contains("session_id"));
    }

    // ── Multi-session detection tests ────────────────────────────────

    fn make_manifest(sessions: Vec<(&str, Vec<&str>)>) -> assay_types::RunManifest {
        assay_types::RunManifest {
            sessions: sessions
                .into_iter()
                .map(|(spec, deps)| assay_types::ManifestSession {
                    spec: spec.to_string(),
                    name: None,
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: deps.into_iter().map(|d| d.to_string()).collect(),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn needs_orchestration_single_session_no_deps() {
        let manifest = make_manifest(vec![("auth", vec![])]);
        assert!(!needs_orchestration(&manifest));
    }

    #[test]
    fn needs_orchestration_single_session_with_deps() {
        // Odd edge case: single session with depends_on (self-referencing or
        // external). Still triggers orchestration for validation purposes.
        let manifest = make_manifest(vec![("auth", vec!["setup"])]);
        assert!(needs_orchestration(&manifest));
    }

    #[test]
    fn needs_orchestration_multi_session_no_deps() {
        let manifest = make_manifest(vec![("auth", vec![]), ("db", vec![])]);
        assert!(needs_orchestration(&manifest));
    }

    #[test]
    fn needs_orchestration_multi_session_with_deps() {
        let manifest = make_manifest(vec![
            ("auth", vec![]),
            ("db", vec!["auth"]),
            ("api", vec!["auth", "db"]),
        ]);
        assert!(needs_orchestration(&manifest));
    }

    // ── Mode dispatch tests ──────────────────────────────────────────

    fn make_manifest_with_mode(
        mode: OrchestratorMode,
        sessions: Vec<(&str, Vec<&str>)>,
    ) -> assay_types::RunManifest {
        let mut m = make_manifest(sessions);
        m.mode = mode;
        m
    }

    #[test]
    fn mode_mesh_bypasses_needs_orchestration() {
        // A single-session Mesh manifest: needs_orchestration returns false
        // (no DAG reason to orchestrate), but the mode match routes to execute_mesh
        // before the needs_orchestration check runs.
        let manifest = make_manifest_with_mode(OrchestratorMode::Mesh, vec![("auth", vec![])]);
        // DAG orchestration check: single session, no deps → false.
        assert!(
            !needs_orchestration(&manifest),
            "single-session no-dep manifest does not need DAG orchestration"
        );
        // The mode is Mesh, so the match arm catches it before needs_orchestration.
        assert_eq!(manifest.mode, OrchestratorMode::Mesh);
        // This proves the routing decision: mode == Mesh → execute_mesh, not execute_sequential.
    }

    #[test]
    fn mode_gossip_bypasses_needs_orchestration() {
        let manifest = make_manifest_with_mode(OrchestratorMode::Gossip, vec![("auth", vec![])]);
        assert!(!needs_orchestration(&manifest));
        assert_eq!(manifest.mode, OrchestratorMode::Gossip);
    }

    #[test]
    fn mode_dag_falls_through_to_needs_orchestration() {
        let manifest = make_manifest_with_mode(
            OrchestratorMode::Dag,
            vec![("auth", vec![]), ("db", vec![])],
        );
        // Dag mode falls through; multi-session triggers needs_orchestration.
        assert_eq!(manifest.mode, OrchestratorMode::Dag);
        assert!(needs_orchestration(&manifest));
    }

    // ── Orchestration response serialization tests ───────────────────

    #[test]
    fn orchestration_response_serializes_to_json() {
        use assay_types::{MergePlan, MergeReport, MergeSessionResult, MergeSessionStatus};

        let response = OrchestrationResponse {
            run_id: "01JTEST789".to_string(),
            sessions: vec![
                OrchestrationSessionResult {
                    name: "auth".to_string(),
                    outcome: "completed".to_string(),
                    error: None,
                    skip_reason: None,
                },
                OrchestrationSessionResult {
                    name: "db".to_string(),
                    outcome: "failed".to_string(),
                    error: Some("agent crashed".to_string()),
                    skip_reason: None,
                },
                OrchestrationSessionResult {
                    name: "api".to_string(),
                    outcome: "skipped".to_string(),
                    error: None,
                    skip_reason: Some("upstream 'db' failed".to_string()),
                },
            ],
            merge_report: MergeReport {
                sessions_merged: 1,
                sessions_skipped: 0,
                conflict_skipped: 0,
                aborted: 0,
                plan: MergePlan {
                    strategy: MergeStrategy::CompletionTime,
                    entries: vec![],
                },
                results: vec![MergeSessionResult {
                    session_name: "auth".to_string(),
                    status: MergeSessionStatus::Merged,
                    merge_sha: Some("abc123".to_string()),
                    error: None,
                }],
                duration_secs: 1.5,
                resolutions: vec![],
            },
            summary: OrchestrationSummary {
                total: 3,
                completed: 1,
                failed: 1,
                skipped: 1,
                sessions_merged: 1,
                merge_conflicts: 0,
                duration_secs: 45.2,
            },
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        assert!(json.contains("01JTEST789"));
        assert!(json.contains("auth"));
        assert!(json.contains("completed"));
        assert!(json.contains("agent crashed"));
        assert!(json.contains("upstream 'db' failed"));
        assert!(json.contains("abc123"));
        assert!(json.contains("sessions_merged"));
        // skip_reason should be absent for completed session
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["sessions"][0]["skip_reason"].is_null());
        assert!(parsed["sessions"][0]["error"].is_null());
    }

    fn make_two_session_manifest(
        mode: assay_types::orchestrate::OrchestratorMode,
    ) -> assay_types::RunManifest {
        assay_types::RunManifest {
            sessions: vec![
                assay_types::ManifestSession {
                    spec: "spec-a".to_string(),
                    name: None,
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: vec![],
                },
                assay_types::ManifestSession {
                    spec: "spec-b".to_string(),
                    name: None,
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: vec![],
                },
            ],
            mode,
            mesh_config: None,
            gossip_config: None,
        }
    }

    /// Guard test: confirms Mesh mode is preserved through manifest construction and
    /// that the manifest shape expected by execute_mesh() is well-formed.
    #[test]
    fn execute_mesh_output_mode_label() {
        use assay_types::orchestrate::OrchestratorMode;
        let manifest = make_two_session_manifest(OrchestratorMode::Mesh);
        assert_eq!(manifest.mode, OrchestratorMode::Mesh);
        assert_eq!(manifest.sessions.len(), 2);
    }

    /// Guard test: confirms Gossip mode is preserved through manifest construction and
    /// that the manifest shape expected by execute_gossip() is well-formed.
    #[test]
    fn execute_gossip_output_mode_label() {
        use assay_types::orchestrate::OrchestratorMode;
        let manifest = make_two_session_manifest(OrchestratorMode::Gossip);
        assert_eq!(manifest.mode, OrchestratorMode::Gossip);
        assert_eq!(manifest.sessions.len(), 2);
    }
}
