use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use super::{assay_dir, project_root};

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
    assay run manifest.toml --base-branch develop")]
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
    eprintln!("Loading manifest: {}", cmd.manifest.display());
    let manifest = assay_core::manifest::load(&cmd.manifest).map_err(|e| anyhow::anyhow!("{e}"))?;
    eprintln!("Manifest loaded: {} session(s)", manifest.sessions.len());

    // Build pipeline config
    let pipeline_config = assay_core::pipeline::PipelineConfig {
        project_root: root.clone(),
        assay_dir: ad,
        specs_dir,
        worktree_base,
        timeout_secs: cmd.timeout,
        base_branch: cmd.base_branch.clone(),
    };

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
    let results = assay_core::pipeline::run_manifest(&manifest, &pipeline_config, &harness_writer);

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
                    eprintln!("  [{}] {} — {}", pr.session_id, spec_name, outcome_str);
                    for t in &timings {
                        eprintln!("    {}: {:.1}s", t.stage, t.duration_secs);
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
                    eprintln!("  [ERROR] {} — [{}] {}", spec_name, pe.stage, pe.message);
                    eprintln!("    Recovery: {}", pe.recovery);
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
        eprintln!();
        eprintln!(
            "Summary: {} total, {} succeeded, {} gate failures, {} merge conflicts, {} errors",
            total, succeeded, gate_failed, merge_conflict, errored
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
        ]);
        assert_eq!(cmd.manifest, PathBuf::from("path/to/manifest.toml"));
        assert_eq!(cmd.timeout, 900);
        assert!(cmd.json);
        assert_eq!(cmd.base_branch.as_deref(), Some("develop"));
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
}
