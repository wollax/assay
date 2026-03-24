//! `smelt list` subcommand — enumerate all per-job runs in `.smelt/runs/`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use smelt_core::monitor::JobMonitor;

/// List past runs found in `.smelt/runs/`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Directory to search for `.smelt/runs/` (defaults to current directory).
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,
}

/// Execute the `list` subcommand.
pub async fn execute(args: &ListArgs) -> Result<i32> {
    let runs_dir = args.dir.join(".smelt").join("runs");

    if !runs_dir.exists() {
        println!("No past runs.");
        return Ok(0);
    }

    let mut entries = std::fs::read_dir(&runs_dir)?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();

    // Sort deterministically by entry name
    entries.sort_by_key(|e| e.file_name());

    let mut header_printed = false;
    for entry in entries {
        let entry_path = entry.path();
        let state_path = entry_path.join("state.toml");

        // Skip entries with no state.toml (e.g. empty directories)
        if !state_path.exists() {
            continue;
        }

        // Emit header before first data row
        if !header_printed {
            println!("  {:<20}  {:<15}  {:<8}  PR URL", "JOB", "PHASE", "ELAPSED");
            println!("  {:-<20}  {:-<15}  {:-<8}  ------", "", "", "");
            header_printed = true;
        }

        match JobMonitor::read(&entry_path) {
            Err(e) => {
                eprintln!("[WARN] skipping {}: {}", entry_path.display(), e);
            }
            Ok(state) => {
                let elapsed = state.updated_at.saturating_sub(state.started_at);
                let pr_url = state.pr_url.as_deref().unwrap_or("-");
                let phase_str = format!("{:?}", state.phase);
                println!(
                    "  {:<20}  {:<15}  {:<8}  {}",
                    state.job_name,
                    phase_str,
                    format!("{}s", elapsed),
                    pr_url,
                );
            }
        }
    }

    if !header_printed {
        println!("No past runs.");
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smelt_core::monitor::{JobMonitor, JobPhase};
    use tempfile::TempDir;

    fn make_args(dir: &std::path::Path) -> ListArgs {
        ListArgs {
            dir: dir.to_path_buf(),
        }
    }

    /// Helper: write a minimal state.toml for a named job inside `runs_dir`.
    fn write_state(runs_dir: &std::path::Path, job_name: &str, phase: JobPhase) {
        let job_dir = runs_dir.join(job_name);
        let sessions: Vec<String> = vec!["session-1".to_string()];
        let monitor = JobMonitor::new(job_name, sessions, &job_dir);
        // Manually build a state and write it
        let mut m = monitor;
        let _ = m.set_phase(phase);
    }

    #[tokio::test]
    async fn test_list_missing_runs_dir() {
        let tmp = TempDir::new().unwrap();
        let args = make_args(tmp.path());
        let result = execute(&args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_list_with_state_files() {
        let tmp = TempDir::new().unwrap();
        let runs_dir = tmp.path().join(".smelt").join("runs");
        std::fs::create_dir_all(&runs_dir).unwrap();

        write_state(&runs_dir, "job-a", JobPhase::Complete);
        write_state(&runs_dir, "job-b", JobPhase::Failed);

        let args = make_args(tmp.path());
        let result = execute(&args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_list_skips_corrupt_state() {
        let tmp = TempDir::new().unwrap();
        let runs_dir = tmp.path().join(".smelt").join("runs");
        let bad_dir = runs_dir.join("bad");
        std::fs::create_dir_all(&bad_dir).unwrap();

        // Write invalid TOML to state.toml
        std::fs::write(bad_dir.join("state.toml"), b"not valid toml = [[[").unwrap();

        let args = make_args(tmp.path());
        let result = execute(&args).await;
        assert!(result.is_ok(), "should exit 0 even with corrupt state");
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_list_skips_entry_without_state_toml() {
        let tmp = TempDir::new().unwrap();
        let runs_dir = tmp.path().join(".smelt").join("runs");
        // Create an empty subdirectory — no state.toml
        std::fs::create_dir_all(runs_dir.join("empty-dir")).unwrap();

        let args = make_args(tmp.path());
        let result = execute(&args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
