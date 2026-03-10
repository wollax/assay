//! `smelt session` command handlers.

use std::path::{Path, PathBuf};

use clap::Subcommand;
use smelt_core::session::types::SessionOutcome;
use smelt_core::{GitCli, Manifest, SessionRunner};

/// Session subcommands for managing and running agent sessions.
#[derive(Subcommand)]
pub enum SessionCommands {
    /// Execute sessions defined in a manifest file
    Run {
        /// Path to the session manifest TOML file
        manifest: PathBuf,
    },
}

/// Execute the `session run` subcommand.
///
/// Loads a manifest, runs all sessions via `SessionRunner`, prints results,
/// and returns exit code 0 if all sessions completed, 1 if any failed.
pub async fn execute_run(
    git: GitCli,
    repo_root: PathBuf,
    manifest_path: &Path,
) -> anyhow::Result<i32> {
    let manifest = match Manifest::load(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {e}");
            return Ok(1);
        }
    };

    let n = manifest.sessions.len();
    println!(
        "Running {n} session(s) from '{}'...",
        manifest.manifest.name
    );

    let runner = SessionRunner::new(git, repo_root);
    let results = match runner.run_manifest(&manifest).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            return Ok(1);
        }
    };

    let mut passed = 0usize;
    let total = results.len();

    for result in &results {
        let outcome_str = match result.outcome {
            SessionOutcome::Completed => "Completed",
            SessionOutcome::Failed => "Failed",
            SessionOutcome::TimedOut => "TimedOut",
            SessionOutcome::Killed => "Killed",
        };

        println!(
            "  {}: {} ({} steps, {:.1}s)",
            result.session_name,
            outcome_str,
            result.steps_completed,
            result.duration.as_secs_f64(),
        );

        if let Some(ref reason) = result.failure_reason {
            println!("    Reason: {reason}");
        }

        if result.outcome == SessionOutcome::Completed {
            passed += 1;
        }
    }

    println!("{passed}/{total} sessions completed successfully");

    if passed == total { Ok(0) } else { Ok(1) }
}
