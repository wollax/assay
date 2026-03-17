//! `smelt status` subcommand — show status of a running job.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::Args;

use smelt_core::monitor::{JobMonitor, JobPhase, RunState};

/// Show status of a running job.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Path to the project root directory (defaults to current directory).
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,
}

/// Execute the `status` subcommand.
pub async fn execute(args: &StatusArgs) -> Result<i32> {
    let state_dir = args.dir.join(".smelt");

    let state = match JobMonitor::read(&state_dir) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("No running job.");
            return Ok(1);
        }
    };

    let stale = is_pid_stale(state.pid);

    print_status(&state, stale);

    if stale || is_terminal_phase(&state.phase) {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Check if a PID is no longer running.
///
/// Uses `kill -0` on Unix to probe without sending a signal.
/// Returns `true` if the process is definitely dead, `false` if alive or
/// detection is unavailable (non-Unix).
fn is_pid_stale(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        match output {
            Ok(o) => !o.status.success(),
            Err(_) => false, // can't determine — assume alive
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Returns `true` for terminal phases where the job is no longer running.
fn is_terminal_phase(phase: &JobPhase) -> bool {
    matches!(
        phase,
        JobPhase::Complete | JobPhase::Failed | JobPhase::Timeout | JobPhase::Cancelled
    )
}

/// Format elapsed seconds into human-readable "Xh Ym Zs" form.
fn format_elapsed(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {secs}s")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

/// Print formatted job status to stdout.
fn print_status(state: &RunState, stale: bool) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs();
    let elapsed = now.saturating_sub(state.started_at);

    if stale {
        eprintln!(
            "Warning: process (PID {}) is no longer running — state may be stale.",
            state.pid
        );
    }

    println!("Job:       {}", state.job_name);
    println!("Phase:     {:?}", state.phase);
    if let Some(ref cid) = state.container_id {
        println!("Container: {cid}");
    }
    if !state.sessions.is_empty() {
        println!("Sessions:  {}", state.sessions.join(", "));
    }
    println!("PID:       {}", state.pid);
    println!("Started:   {} ago", format_elapsed(elapsed));
    println!("Elapsed:   {}", format_elapsed(elapsed));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: write a RunState TOML file directly to a temp directory.
    fn write_state(dir: &std::path::Path, state: &RunState) {
        let state_dir = dir.join(".smelt");
        std::fs::create_dir_all(&state_dir).unwrap();
        let content = toml::to_string_pretty(state).unwrap();
        std::fs::write(state_dir.join("run-state.toml"), content).unwrap();
    }

    fn make_active_state() -> RunState {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        RunState {
            job_name: "integration-test".into(),
            phase: JobPhase::Executing,
            container_id: Some("abc123def".into()),
            sessions: vec!["lint".into(), "test".into()],
            started_at: now - 120,
            updated_at: now - 10,
            pid: std::process::id(), // current process — definitely alive
        }
    }

    #[tokio::test]
    async fn test_status_no_state_file() {
        let dir = TempDir::new().unwrap();
        let args = StatusArgs {
            dir: dir.path().to_path_buf(),
        };
        let code = execute(&args).await.unwrap();
        assert_eq!(code, 1, "should return 1 when no state file exists");
    }

    #[tokio::test]
    async fn test_status_with_active_job() {
        let dir = TempDir::new().unwrap();
        let state = make_active_state();
        write_state(dir.path(), &state);

        let args = StatusArgs {
            dir: dir.path().to_path_buf(),
        };
        let code = execute(&args).await.unwrap();
        assert_eq!(code, 0, "should return 0 for active job with live PID");
    }

    #[tokio::test]
    async fn test_status_stale_pid() {
        let dir = TempDir::new().unwrap();
        let mut state = make_active_state();
        state.pid = 99_999_999; // extremely unlikely to be a real PID
        write_state(dir.path(), &state);

        let args = StatusArgs {
            dir: dir.path().to_path_buf(),
        };
        let code = execute(&args).await.unwrap();
        assert_eq!(code, 1, "should return 1 for stale PID");
    }

    #[test]
    fn test_format_elapsed_seconds_only() {
        assert_eq!(format_elapsed(42), "42s");
    }

    #[test]
    fn test_format_elapsed_minutes_seconds() {
        assert_eq!(format_elapsed(125), "2m 5s");
    }

    #[test]
    fn test_format_elapsed_hours() {
        assert_eq!(format_elapsed(3661), "1h 1m 1s");
    }

    #[test]
    fn test_is_terminal_phase() {
        assert!(is_terminal_phase(&JobPhase::Complete));
        assert!(is_terminal_phase(&JobPhase::Failed));
        assert!(is_terminal_phase(&JobPhase::Timeout));
        assert!(is_terminal_phase(&JobPhase::Cancelled));
        assert!(!is_terminal_phase(&JobPhase::Executing));
        assert!(!is_terminal_phase(&JobPhase::Provisioning));
    }
}
