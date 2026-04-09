//! Checkpoint driver: consumes agent events from a channel, fires mid-session
//! gate evaluation at trigger points, and tracks a per-session checkpoint budget.
//!
//! The driver is a pure channel consumer — it does **not** manage the agent
//! subprocess. Subprocess lifecycle (kill on failure, relay drain) is handled
//! by the pipeline caller.
//!
//! # Warning on command/file gates at checkpoints
//!
//! Command or file-path criteria with `when = AfterToolCalls` evaluate against
//! a **partial working directory** mid-session. Spec authors should prefer
//! event-based criteria (`EventCount`, `NoToolErrors`) for checkpoints.

use std::path::Path;
use std::process::Child;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use assay_types::review::CheckpointPhase;
use assay_types::{AgentEvent, GateRunSummary, Spec};

use crate::gate::evaluate_checkpoint;

/// Maximum number of checkpoint evaluations per session before the driver
/// stops firing and lets the session run to completion. Budget exhaustion
/// is not a failure — session-end gates still evaluate normally.
pub const DEFAULT_CHECKPOINT_BUDGET: u32 = 16;

/// Result of a single checkpoint evaluation.
#[derive(Debug, Clone)]
pub struct CheckpointResult {
    /// Zero-based index of this checkpoint within the session.
    pub index: u32,
    /// The phase at which the checkpoint fired.
    pub phase: CheckpointPhase,
    /// Gate evaluation summary for the matching criteria.
    pub summary: GateRunSummary,
}

/// Outcome of running the checkpoint driver over an entire event stream.
#[derive(Debug, Clone)]
pub struct CheckpointOutcome {
    /// Checkpoints that passed (in evaluation order).
    pub passed: Vec<CheckpointResult>,
    /// The first checkpoint that failed, if any. When `Some`, the driver
    /// short-circuited and the caller should kill the subprocess.
    pub failed: Option<CheckpointResult>,
    /// Whether the checkpoint budget was exhausted. When `true`, some
    /// trigger events were skipped. S04 reads this to suppress auto-promote.
    pub budget_exhausted: bool,
}

/// Configuration for the checkpoint driver.
#[derive(Debug, Clone)]
pub struct CheckpointDriverConfig {
    /// Maximum number of checkpoint evaluations before the driver stops.
    pub budget: u32,
    /// Wall-clock deadline for the entire checkpoint loop.
    pub timeout: Duration,
    /// CLI-level timeout override (highest precedence).
    pub cli_timeout: Option<u64>,
    /// Config-file timeout override (second precedence).
    pub config_timeout: Option<u64>,
}

/// Consume events from `event_rx`, firing `evaluate_checkpoint` at trigger
/// points, and return the aggregate outcome.
///
/// The driver:
/// 1. Loops on `event_rx.recv()` until the channel disconnects (sender dropped).
/// 2. Pushes each event into `buffer`.
/// 3. Tracks `tool_call_count` (incremented on `AgentEvent::ToolCalled`).
/// 4. After each event, calls `evaluate_checkpoint` if any criterion's `when`
///    matches the current phase. Decrements `remaining_budget` on each evaluation.
/// 5. On first failure: short-circuits and returns `failed: Some(...)`.
/// 6. On budget exhaustion: stops evaluating but keeps draining the channel.
///
/// # Note for callers
///
/// On `failed: Some(...)`, the caller must kill the subprocess and drain
/// the channel to allow the relay thread to exit cleanly.
pub fn drive_checkpoints(
    event_rx: &Receiver<AgentEvent>,
    spec: &Spec,
    working_dir: &Path,
    cfg: &CheckpointDriverConfig,
    buffer: &mut Vec<AgentEvent>,
) -> CheckpointOutcome {
    let mut passed = Vec::new();
    let mut remaining_budget = cfg.budget;
    let mut tool_call_count: u32 = 0;
    let mut checkpoint_index: u32 = 0;
    let mut budget_exhausted = false;
    let deadline = Instant::now() + cfg.timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            tracing::warn!("checkpoint driver timed out waiting for events");
            break;
        }
        let event = match event_rx.recv_timeout(remaining) {
            Ok(e) => e,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tracing::warn!("checkpoint driver timed out waiting for events");
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        };
        if matches!(event, AgentEvent::ToolCalled { .. }) {
            tool_call_count += 1;
        }

        buffer.push(event);

        // Skip evaluation if budget is exhausted.
        if budget_exhausted {
            continue;
        }

        // No checkpoint criteria in spec → skip evaluation entirely.
        if !has_checkpoint_criteria(spec) {
            continue;
        }

        let phase = CheckpointPhase::AtToolCall { n: tool_call_count };
        let summary = evaluate_checkpoint(
            spec,
            working_dir,
            cfg.cli_timeout,
            cfg.config_timeout,
            buffer,
            phase.clone(),
        );

        // If no criteria matched this phase, no evaluation happened.
        if summary.results.is_empty() {
            continue;
        }

        remaining_budget = remaining_budget.saturating_sub(1);

        let result = CheckpointResult {
            index: checkpoint_index,
            phase,
            summary,
        };

        if result.summary.failed > 0 {
            return CheckpointOutcome {
                passed,
                failed: Some(result),
                budget_exhausted: false,
            };
        }

        checkpoint_index += 1;
        passed.push(result);

        if remaining_budget == 0 {
            budget_exhausted = true;
            tracing::info!(
                spec = %spec.name,
                evaluations = checkpoint_index,
                "checkpoint budget exhausted; remaining events will not trigger evaluation"
            );
        }
    }

    CheckpointOutcome {
        passed,
        failed: None,
        budget_exhausted,
    }
}

/// Returns `true` if the spec has any criteria with a non-session-end `when` field.
fn has_checkpoint_criteria(spec: &Spec) -> bool {
    use assay_types::criterion::When;
    spec.criteria
        .iter()
        .any(|c| matches!(c.when, When::AfterToolCalls { .. } | When::OnEvent { .. }))
}

// ── Subprocess kill helper ───────────────────────────────────────────

/// Grace period between SIGTERM and SIGKILL.
const KILL_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Poll interval when waiting for the child to exit after SIGTERM.
const KILL_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Kill an agent subprocess gracefully: SIGTERM → grace period → SIGKILL.
///
/// 1. Sends SIGTERM to the process (or process group if `process_group(0)` was set).
/// 2. Drains `event_rx` in a loop to prevent the relay thread from blocking
///    on `event_tx.send()`. Drained events are appended to `drain_buffer`.
/// 3. Polls `child.try_wait()` until the process exits or the grace period expires.
/// 4. If still alive after the grace period, sends SIGKILL.
///
/// Returns the exit code (or `None` if the process was killed and has no code).
///
/// # Platform
///
/// SIGTERM/SIGKILL are Unix-only. On non-Unix platforms, falls back to
/// `child.kill()` (immediate SIGKILL equivalent).
pub fn kill_agent_subprocess(
    child: &mut Child,
    event_rx: &Receiver<AgentEvent>,
    drain_buffer: &mut Vec<AgentEvent>,
) -> Option<i32> {
    let pid = child.id();

    // Step 1: Send SIGTERM.
    #[cfg(unix)]
    {
        // SAFETY: child.id() returns a valid u32 PID; the subprocess was
        // spawned with process_group(0) so its pgid == pid. killpg sends
        // SIGTERM to the entire process group, terminating child processes
        // that would otherwise be orphaned.
        unsafe {
            libc::killpg(pid as libc::pid_t, libc::SIGTERM);
        }
        tracing::info!(pid, "sent SIGTERM to agent process group");
    }

    #[cfg(not(unix))]
    {
        let _ = child.kill();
        tracing::info!(pid, "sent kill to agent subprocess (non-Unix)");
    }

    // Step 2+3: Drain channel + poll for exit within grace period.
    let deadline = Instant::now() + KILL_GRACE_PERIOD;

    loop {
        // Drain any pending events to unblock the relay thread.
        while let Ok(event) = event_rx.try_recv() {
            drain_buffer.push(event);
        }

        // Check if child has exited.
        match child.try_wait() {
            Ok(Some(status)) => {
                tracing::info!(pid, ?status, "agent subprocess exited after SIGTERM");
                return status.code();
            }
            Ok(None) => {
                // Still running.
                if Instant::now() >= deadline {
                    break; // Grace period expired.
                }
                std::thread::sleep(KILL_POLL_INTERVAL);
            }
            Err(e) => {
                tracing::warn!(pid, error = %e, "error polling agent subprocess");
                break;
            }
        }
    }

    // Step 4: SIGKILL — grace period expired.
    tracing::warn!(
        pid,
        grace_secs = KILL_GRACE_PERIOD.as_secs(),
        "agent subprocess did not exit after SIGTERM; sending SIGKILL"
    );

    #[cfg(unix)]
    // SAFETY: pid is still valid (process exists, just didn't exit cleanly);
    // pgid == pid because process_group(0) was set at spawn. killpg ensures
    // the entire process group is killed, not just the direct child.
    unsafe {
        libc::killpg(pid as libc::pid_t, libc::SIGKILL);
    }

    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }

    // Final drain + reap.
    while let Ok(event) = event_rx.try_recv() {
        drain_buffer.push(event);
    }
    child.wait().ok().and_then(|s| s.code())
}

/// Drain all remaining events from the channel until it disconnects.
/// Used after kill to ensure the relay thread can exit cleanly.
pub fn drain_channel(event_rx: &Receiver<AgentEvent>, buffer: &mut Vec<AgentEvent>) {
    while let Ok(event) = event_rx.recv() {
        buffer.push(event);
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::mpsc;

    use super::*;
    use assay_types::criterion::When;
    use assay_types::{Criterion, CriterionKind};

    const TEST_TIMEOUT: Duration = Duration::from_secs(30);

    fn test_config() -> CheckpointDriverConfig {
        CheckpointDriverConfig {
            budget: 16,
            timeout: TEST_TIMEOUT,
            cli_timeout: None,
            config_timeout: None,
        }
    }

    fn make_spec_with_checkpoint(when: When, kind: Option<CriterionKind>) -> Spec {
        Spec {
            name: "checkpoint-test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![Criterion {
                name: "check".to_string(),
                description: String::new(),
                cmd: Some("echo ok".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind,
                prompt: None,
                requirements: vec![],
                when,
            }],
        }
    }

    fn make_spec_session_end_only() -> Spec {
        Spec {
            name: "session-end-only".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![Criterion {
                name: "final-check".to_string(),
                description: String::new(),
                cmd: Some("echo ok".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: When::default(),
            }],
        }
    }

    fn tool_called(name: &str) -> AgentEvent {
        AgentEvent::ToolCalled {
            name: name.to_string(),
            input_json: "{}".to_string(),
        }
    }

    fn send_events(tx: &mpsc::Sender<AgentEvent>, events: Vec<AgentEvent>) {
        for e in events {
            tx.send(e).unwrap();
        }
    }

    #[test]
    fn clean_run_no_checkpoint_criteria() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_spec_session_end_only();
        let (tx, rx) = mpsc::channel();
        send_events(&tx, vec![tool_called("bash"), tool_called("read")]);
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_none());
        assert!(outcome.passed.is_empty());
        assert!(!outcome.budget_exhausted);
        assert_eq!(buffer.len(), 2, "buffer should contain all events");
    }

    #[test]
    fn checkpoint_passes_at_matching_tool_call_count() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_spec_with_checkpoint(
            When::AfterToolCalls {
                n: NonZeroU32::new(2).unwrap(),
            },
            None,
        );
        let (tx, rx) = mpsc::channel();
        send_events(&tx, vec![tool_called("bash"), tool_called("read")]);
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_none());
        assert_eq!(outcome.passed.len(), 1);
        assert_eq!(outcome.passed[0].index, 0);
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn checkpoint_failure_short_circuits() {
        let dir = tempfile::tempdir().unwrap();
        // Criterion: max 1 tool call event allowed, check at n=2
        let spec = Spec {
            name: "fail-test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![Criterion {
                name: "too-many-tools".to_string(),
                description: String::new(),
                cmd: Some("false".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: When::AfterToolCalls {
                    n: NonZeroU32::new(2).unwrap(),
                },
            }],
        };

        let (tx, rx) = mpsc::channel();
        // Send 3 events — checkpoint fires at n=2, fails, should not see event 3
        send_events(
            &tx,
            vec![
                tool_called("bash"),
                tool_called("read"),
                tool_called("write"),
            ],
        );
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_some());
        let failed = outcome.failed.unwrap();
        assert_eq!(failed.index, 0);
        assert!(failed.summary.failed > 0);
        // Buffer should contain at least the events up to and including the trigger
        assert!(buffer.len() >= 2);
    }

    #[test]
    fn budget_exhaustion_stops_evaluation() {
        let dir = tempfile::tempdir().unwrap();
        // 3 criteria each firing at a different tool-call count, budget of 2.
        // After 2 evaluations, the third trigger should be skipped.
        let spec = Spec {
            name: "budget-test".to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            criteria: vec![
                Criterion {
                    name: "at-1".to_string(),
                    description: String::new(),
                    cmd: Some("echo ok".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                    when: When::AfterToolCalls {
                        n: NonZeroU32::new(1).unwrap(),
                    },
                },
                Criterion {
                    name: "at-2".to_string(),
                    description: String::new(),
                    cmd: Some("echo ok".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                    when: When::AfterToolCalls {
                        n: NonZeroU32::new(2).unwrap(),
                    },
                },
                Criterion {
                    name: "at-3".to_string(),
                    description: String::new(),
                    cmd: Some("echo ok".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                    when: When::AfterToolCalls {
                        n: NonZeroU32::new(3).unwrap(),
                    },
                },
            ],
        };

        let (tx, rx) = mpsc::channel();
        // Send 3 tool calls — triggers at n=1, n=2, n=3. Budget = 2.
        send_events(
            &tx,
            vec![tool_called("a"), tool_called("b"), tool_called("c")],
        );
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(
            &rx,
            &spec,
            dir.path(),
            &CheckpointDriverConfig {
                budget: 2,
                ..test_config()
            },
            &mut buffer,
        );

        assert!(outcome.failed.is_none());
        assert!(outcome.budget_exhausted);
        // Should have evaluated exactly 2 times (budget), third skipped
        assert_eq!(outcome.passed.len(), 2);
        // All 3 events should be in the buffer (driver keeps draining)
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn empty_event_stream_returns_clean_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_spec_with_checkpoint(
            When::AfterToolCalls {
                n: NonZeroU32::new(1).unwrap(),
            },
            None,
        );
        let (tx, rx) = mpsc::channel();
        drop(tx); // Immediate disconnect

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_none());
        assert!(outcome.passed.is_empty());
        assert!(!outcome.budget_exhausted);
        assert!(buffer.is_empty());
    }

    #[test]
    fn non_tool_events_do_not_increment_counter() {
        let dir = tempfile::tempdir().unwrap();
        // Checkpoint fires at n=2 tool calls
        let spec = make_spec_with_checkpoint(
            When::AfterToolCalls {
                n: NonZeroU32::new(2).unwrap(),
            },
            None,
        );

        let (tx, rx) = mpsc::channel();
        // Send 1 tool call + 5 non-tool events + 1 tool call = 2 tool calls total
        send_events(
            &tx,
            vec![
                tool_called("bash"),
                AgentEvent::TextDelta {
                    text: "hello".to_string(),
                    block_index: 0,
                },
                AgentEvent::TextDelta {
                    text: " world".to_string(),
                    block_index: 0,
                },
                tool_called("read"),
            ],
        );
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_none());
        // Should have triggered at tool_call_count == 2
        assert_eq!(outcome.passed.len(), 1);
        assert_eq!(buffer.len(), 4);
    }

    // ------------------------------------------------------------------
    // OnEvent checkpoint coverage (WOL-467)
    // ------------------------------------------------------------------

    #[test]
    fn on_event_checkpoint_fires_for_matching_event() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_spec_with_checkpoint(
            When::OnEvent {
                event_type: "tool_called".to_string(),
            },
            None,
        );
        let (tx, rx) = mpsc::channel();
        send_events(&tx, vec![tool_called("bash")]);
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_none());
        assert_eq!(outcome.passed.len(), 1, "OnEvent checkpoint should fire");
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn on_event_checkpoint_skips_non_matching_events() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_spec_with_checkpoint(
            When::OnEvent {
                event_type: "tool_called".to_string(),
            },
            None,
        );
        let (tx, rx) = mpsc::channel();
        // Send non-matching events only
        send_events(
            &tx,
            vec![
                AgentEvent::TextDelta {
                    text: "hello".to_string(),
                    block_index: 0,
                },
                AgentEvent::TurnEnded { turn_index: 1 },
            ],
        );
        drop(tx);

        let mut buffer = Vec::new();
        let outcome = drive_checkpoints(&rx, &spec, dir.path(), &test_config(), &mut buffer);

        assert!(outcome.failed.is_none());
        assert!(
            outcome.passed.is_empty(),
            "no matching events, no checkpoint should fire"
        );
        assert_eq!(buffer.len(), 2);
    }

    // ------------------------------------------------------------------
    // kill_agent_subprocess (T03)
    // ------------------------------------------------------------------

    #[cfg(unix)]
    #[test]
    fn kill_helper_terminates_long_running_process() {
        use std::os::unix::process::CommandExt as _;
        use std::process::Command;

        // Spawn a long-running process that would outlive the test.
        // process_group(0) puts the child in its own process group (pgid == pid),
        // so killpg(pid, SIG) terminates only this child, not the test runner.
        let mut child = unsafe {
            Command::new("sh")
                .args(["-c", "sleep 300"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                })
                .spawn()
                .expect("spawn sleep")
        };

        let pid = child.id();

        // Verify the process is running.
        assert!(
            unsafe { libc::kill(pid as i32, 0) } == 0,
            "process should be alive"
        );

        let (tx, rx) = mpsc::channel::<AgentEvent>();
        // Send a few events to the channel to test draining.
        let _ = tx.send(tool_called("bash"));
        let _ = tx.send(tool_called("read"));
        drop(tx);

        let mut drain_buf = Vec::new();
        let exit_code = kill_agent_subprocess(&mut child, &rx, &mut drain_buf);

        // Process should be dead.
        assert!(
            unsafe { libc::kill(pid as i32, 0) } != 0,
            "process should be dead after kill"
        );

        // Drained events should be captured.
        assert_eq!(drain_buf.len(), 2, "should have drained 2 events");

        // Exit code: SIGTERM yields None on most Unix (signal death has no code).
        // Either None or Some(_) is acceptable — the key assertion is that
        // the process is dead.
        let _ = exit_code;
    }

    #[cfg(unix)]
    #[test]
    fn kill_helper_handles_already_exited_process() {
        use std::os::unix::process::CommandExt as _;
        use std::process::Command;

        // Spawn a process that exits immediately.
        // SAFETY: setpgid is async-signal-safe; isolates the child in its own
        // process group so killpg targets only this child, not the test runner.
        let mut child = unsafe {
            Command::new("true")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                })
                .spawn()
                .expect("spawn true")
        };

        // Wait for it to finish.
        let _ = child.wait();

        let (_tx, rx) = mpsc::channel::<AgentEvent>();
        let mut drain_buf = Vec::new();

        // Should not panic or hang — the process is already gone.
        let exit_code = kill_agent_subprocess(&mut child, &rx, &mut drain_buf);
        assert_eq!(exit_code, Some(0));
    }

    #[test]
    fn drain_channel_captures_all_events() {
        let (tx, rx) = mpsc::channel();
        tx.send(tool_called("a")).unwrap();
        tx.send(tool_called("b")).unwrap();
        tx.send(tool_called("c")).unwrap();
        drop(tx);

        let mut buffer = Vec::new();
        drain_channel(&rx, &mut buffer);
        assert_eq!(buffer.len(), 3);
    }
}
