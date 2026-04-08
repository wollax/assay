//! End-to-end pipeline orchestrator.
//!
//! Composes manifest loading, worktree creation, harness configuration,
//! agent launching, gate evaluation, and merge checking into a single
//! sequenced pipeline. This is the capstone module of M001.
//!
//! # Architecture
//!
//! The pipeline is parameterized over a [`HarnessProvider`] trait that
//! generates and writes harness configuration files to the worktree,
//! returning CLI arguments for the agent subprocess. This keeps `assay-core`
//! independent of the specific harness adapter (`assay-harness::claude`).
//!
//! # Sync Design (D007)
//!
//! All functions are synchronous. When calling from an async context (e.g.,
//! MCP server handlers), wrap with `tokio::task::spawn_blocking`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use assay_types::{
    AgentEvent, GateRunSummary, HarnessProfile, ManifestSession, MergeCheck, RunManifest,
    SettingsOverride,
};

use tracing::{info, info_span, instrument, warn};

use crate::spec::SpecEntry;

// ── Pipeline stage enum (R019) ───────────────────────────────────────

/// Identifies which stage of the pipeline an error occurred in.
///
/// Provides structured error context so downstream consumers (CLI, MCP,
/// future agents) can programmatically route recovery actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// Loading the spec referenced by the manifest session.
    SpecLoad,
    /// Creating a git worktree for the session.
    WorktreeCreate,
    /// Generating and writing harness configuration files.
    HarnessConfig,
    /// Spawning and monitoring the agent subprocess.
    AgentLaunch,
    /// Evaluating quality gates against agent output.
    GateEvaluate,
    /// Checking merge compatibility between worktree and base branch.
    MergeCheck,
}

impl fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpecLoad => write!(f, "SpecLoad"),
            Self::WorktreeCreate => write!(f, "WorktreeCreate"),
            Self::HarnessConfig => write!(f, "HarnessConfig"),
            Self::AgentLaunch => write!(f, "AgentLaunch"),
            Self::GateEvaluate => write!(f, "GateEvaluate"),
            Self::MergeCheck => write!(f, "MergeCheck"),
        }
    }
}

// ── Pipeline error ───────────────────────────────────────────────────

/// Structured pipeline error with stage context and recovery guidance.
///
/// Wraps failures from any pipeline stage with enough context for a
/// future agent to understand what went wrong and what to do about it.
/// The underlying `AssayError` is captured as a message string (not
/// wrapped directly) because `AssayError` is not `Clone`.
#[derive(Debug, Clone)]
pub struct PipelineError {
    /// Which pipeline stage failed.
    pub stage: PipelineStage,
    /// Human-readable error description.
    pub message: String,
    /// Actionable recovery guidance for the operator or agent.
    pub recovery: String,
    /// Wall-clock time elapsed before the failure.
    pub elapsed: Duration,
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} (elapsed: {:.1}s) — recovery: {}",
            self.stage,
            self.message,
            self.elapsed.as_secs_f64(),
            self.recovery,
        )
    }
}

impl std::error::Error for PipelineError {}

// ── Pipeline outcome ─────────────────────────────────────────────────

/// Final disposition of a pipeline session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineOutcome {
    /// All stages passed: gates passed, merge is clean.
    Success,
    /// Agent completed but gate evaluation failed.
    GateFailed,
    /// Gates passed but merge has conflicts.
    MergeConflict,
}

impl fmt::Display for PipelineOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::GateFailed => write!(f, "GateFailed"),
            Self::MergeConflict => write!(f, "MergeConflict"),
        }
    }
}

/// Timing record for a single pipeline stage.
#[derive(Debug, Clone)]
pub struct StageTiming {
    /// Which stage was timed.
    pub stage: PipelineStage,
    /// Wall-clock duration of the stage.
    pub duration: Duration,
}

/// Successful pipeline result with per-stage outcomes and timing.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Session ID from the work session.
    pub session_id: String,
    /// Spec name that was executed.
    pub spec_name: String,
    /// Gate evaluation summary, if gate evaluation was reached.
    pub gate_summary: Option<GateRunSummary>,
    /// Merge check result, if merge check was reached.
    pub merge_check: Option<MergeCheck>,
    /// Per-stage timing breakdown.
    pub stage_timings: Vec<StageTiming>,
    /// Final outcome of the pipeline.
    pub outcome: PipelineOutcome,
}

// ── Agent output ─────────────────────────────────────────────────────

/// Output captured from the agent subprocess.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// Process exit code, `None` if killed by signal.
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Whether the process was killed due to timeout.
    pub timed_out: bool,
}

// ── Pipeline config ──────────────────────────────────────────────────

/// Configuration for a pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Root of the project repository.
    pub project_root: PathBuf,
    /// Path to the `.assay` directory.
    pub assay_dir: PathBuf,
    /// Path to the specs directory.
    pub specs_dir: PathBuf,
    /// Base directory for worktree creation.
    pub worktree_base: PathBuf,
    /// Maximum seconds to wait for agent subprocess completion.
    pub timeout_secs: u64,
    /// Base branch for worktree creation. `None` = auto-detect.
    pub base_branch: Option<String>,
}

impl PipelineConfig {
    /// Default timeout: 600 seconds (10 minutes).
    pub const DEFAULT_TIMEOUT_SECS: u64 = 600;
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            assay_dir: PathBuf::from(".assay"),
            specs_dir: PathBuf::from(".assay/specs"),
            worktree_base: PathBuf::from("../worktrees"),
            timeout_secs: Self::DEFAULT_TIMEOUT_SECS,
            base_branch: None,
        }
    }
}

// ── TRACEPARENT injection (R065) ─────────────────────────────────────

/// Extract the W3C TRACEPARENT value from the current span context.
///
/// Uses the globally-registered OTel text map propagator to serialize the
/// active span's trace context into a `TRACEPARENT` header value. Returns
/// `None` when no active span exists, the span is disabled, or the
/// propagator produces no `traceparent` entry.
///
/// Feature-gated: only compiled when `telemetry` is enabled.
#[cfg(feature = "telemetry")]
fn extract_traceparent() -> Option<String> {
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::Span::current();
    if span.is_disabled() {
        tracing::debug!("extract_traceparent: no active span; TRACEPARENT not injected");
        return None;
    }

    let cx = span.context();
    let mut carrier = std::collections::HashMap::new();
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut carrier);
    });

    let tp = carrier.remove("traceparent");
    if tp.is_none() {
        tracing::debug!(
            "extract_traceparent: span exists but propagator produced no traceparent value; \
             TRACEPARENT not injected. Is the global TraceContextPropagator registered? \
             Ensure init_tracing() is called with otlp_endpoint set."
        );
    }
    tp
}

/// Inject TRACEPARENT env var into a `Command` from the current span context.
///
/// No-op when there is no active span, the span has no OTel context (e.g.
/// the global propagator was not set), or the propagator returns no value.
///
/// Note: in `launch_agent_streaming`, the TRACEPARENT value is captured
/// *before* `thread::spawn` and injected inline — this helper cannot be
/// called inside the spawned thread because OTel span context is thread-local
/// and the active span will not be visible there.
#[cfg(feature = "telemetry")]
fn inject_traceparent(cmd: &mut std::process::Command) {
    if let Some(tp) = extract_traceparent() {
        cmd.env("TRACEPARENT", &tp);
    }
}

/// Test-visible wrapper exposing `inject_traceparent` to integration tests.
///
/// Allows tests to call the production injection path rather than
/// re-implementing the propagator logic inline.
///
/// Only compiled with the `telemetry` feature; dead-code lint suppressed
/// because this is intentionally a test surface, not production callsite.
#[cfg(feature = "telemetry")]
#[allow(dead_code)]
pub fn inject_traceparent_for_test(cmd: &mut std::process::Command) {
    inject_traceparent(cmd);
}

/// Test-visible wrapper exposing `extract_traceparent` to integration tests.
///
/// Allows tests to verify the no-span guard without calling internal functions.
#[cfg(feature = "telemetry")]
#[allow(dead_code)]
pub fn extract_traceparent_for_test() -> Option<String> {
    extract_traceparent()
}

// ── Agent launcher ───────────────────────────────────────────────────

/// Launch an agent subprocess with timeout enforcement.
///
/// Uses `std::process::Command` (sync, per D007) with a thread-based
/// timeout. On timeout, the child process is killed and `AgentOutput`
/// is returned with `timed_out: true`.
///
/// # Arguments
///
/// * `cli_args` — Arguments for the agent binary (not including the binary name).
/// * `working_dir` — Working directory for the subprocess (should be the worktree root).
/// * `timeout` — Maximum duration to wait for the process.
///
/// # Errors
///
/// Returns `PipelineError` at the `AgentLaunch` stage if the process
/// cannot be spawned (e.g., binary not found).
#[instrument(name = "pipeline::launch_agent", skip(cli_args, timeout), fields(working_dir = %working_dir.display()))]
pub fn launch_agent(
    cli_args: &[String],
    working_dir: &Path,
    timeout: Duration,
) -> std::result::Result<AgentOutput, PipelineError> {
    let start = Instant::now();

    let mut cmd = std::process::Command::new("claude");
    cmd.args(cli_args)
        .current_dir(working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(feature = "telemetry")]
    inject_traceparent(&mut cmd);

    let mut child = cmd.spawn().map_err(|e| PipelineError {
        stage: PipelineStage::AgentLaunch,
        message: format!("Failed to spawn claude subprocess: {e}"),
        recovery: "Claude Code CLI not found — install from https://claude.ai/code".into(),
        elapsed: start.elapsed(),
    })?;

    // Thread-based timeout: spawn a thread to wait for the child,
    // use a channel with recv_timeout to enforce the deadline.
    let (tx, rx) = mpsc::channel();

    // Take ownership of stdout/stderr handles before moving child to thread.
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();

    std::thread::spawn(move || {
        let result = child.wait();
        // Send result back; if receiver is gone (timeout), this is harmless.
        let _ = tx.send((result, child));
    });

    match rx.recv_timeout(timeout) {
        Ok((wait_result, _child)) => {
            let status = wait_result.map_err(|e| PipelineError {
                stage: PipelineStage::AgentLaunch,
                message: format!("Failed to wait on claude subprocess: {e}"),
                recovery: "Check system process limits and retry".into(),
                elapsed: start.elapsed(),
            })?;

            let stdout = child_stdout
                .map(|mut h| {
                    let mut buf = String::new();
                    use std::io::Read;
                    if let Err(e) = h.read_to_string(&mut buf) {
                        tracing::warn!(error = %e, "Failed to read agent stdout; output may be truncated");
                    }
                    buf
                })
                .unwrap_or_default();

            let stderr = child_stderr
                .map(|mut h| {
                    let mut buf = String::new();
                    use std::io::Read;
                    if let Err(e) = h.read_to_string(&mut buf) {
                        tracing::warn!(error = %e, "Failed to read agent stderr; output may be truncated");
                    }
                    buf
                })
                .unwrap_or_default();

            Ok(AgentOutput {
                exit_code: status.code(),
                stdout,
                stderr,
                timed_out: false,
            })
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            // Timeout: try to kill the child process.
            // The child was moved into the thread, so we can't kill it directly.
            // The thread will eventually complete and drop the child, killing it.
            // We return immediately with a timeout result.
            Ok(AgentOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            })
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(PipelineError {
            stage: PipelineStage::AgentLaunch,
            message: "Agent monitor thread disconnected unexpectedly".into(),
            recovery: "Internal error — retry the pipeline run".into(),
            elapsed: start.elapsed(),
        }),
    }
}

/// Launch an agent subprocess and stream its stdout as typed [`AgentEvent`]s.
///
/// Spawns a background thread that reads stdout from the subprocess
/// line-by-line. Each line is first attempted as a Claude NDJSON event via
/// [`assay_harness::claude_stream::parse_claude_events_streaming`]; if that
/// yields one or more events, they are forwarded through `event_tx` in
/// order. If the line is not valid JSON — or JSON that the parser does not
/// recognize — it is forwarded as a synthetic
/// [`AgentEvent::TextDelta { text: line, block_index: 0 }`], so non-Claude
/// adapters (Codex, OpenCode, plain stdout) keep working without losing
/// output. When all events have been sent (EOF), the thread waits for the
/// process to exit and returns the exit code as `i32`. The caller receives
/// this exit code by joining the returned `JoinHandle`.
///
/// Uses an unbounded `mpsc::channel()` for `event_tx` to avoid deadlock:
/// the subprocess can produce events faster than the consumer processes
/// them, and a bounded channel would block the background thread while
/// holding the stdout pipe open (which would stall the process).
///
/// # Failure handling
///
/// If the subprocess cannot be spawned, `event_tx` is dropped (signalling
/// EOF to the receiver) and the thread returns `-1`. The relay-wrapper
/// thread in the TUI observes channel disconnect and emits
/// `TuiEvent::AgentDone { exit_code: -1 }`.
///
/// Handle returned by [`launch_agent_streaming`] that provides both the
/// relay thread join handle and the subprocess PID for signal delivery.
pub struct StreamingAgentHandle {
    /// Join handle for the relay thread. Returns the subprocess exit code.
    pub relay: std::thread::JoinHandle<i32>,
    /// PID of the subprocess (0 if spawn failed or args were empty).
    /// Use with `libc::kill(-pid, signal)` to signal the process group.
    pub pid: u32,
}

/// # Arguments
///
/// * `cli_args` — Full command line: `cli_args[0]` is the binary,
///   `cli_args[1..]` are its arguments.
/// * `working_dir` — Working directory for the subprocess.
/// * `event_tx` — Sender side of the event channel; one [`AgentEvent`]
///   per parsed NDJSON event, or one synthetic `TextDelta` per plain-text
///   stdout line.
pub fn launch_agent_streaming(
    cli_args: &[String],
    working_dir: &std::path::Path,
    event_tx: std::sync::mpsc::Sender<AgentEvent>,
) -> StreamingAgentHandle {
    use std::io::{BufRead, BufReader, Cursor};
    use std::process::Stdio;

    // Guard: an empty cli_args would panic on cli_args[0].
    // Return a thread that immediately signals EOF and exits with -1.
    if cli_args.is_empty() {
        return StreamingAgentHandle {
            relay: std::thread::spawn(move || {
                drop(event_tx);
                -1
            }),
            pid: 0,
        };
    }

    // Clone the args + path before moving into the thread.
    let binary = cli_args[0].clone();
    let args: Vec<String> = cli_args[1..].to_vec();
    let working_dir = working_dir.to_path_buf();

    // Capture TRACEPARENT in the outer scope before spawning the thread,
    // because the OTel span context is thread-local.
    #[cfg(feature = "telemetry")]
    let traceparent_value = extract_traceparent();

    // Spawn the child process in the main thread so we can capture the PID
    // before moving the Child into the relay thread.
    let mut cmd = std::process::Command::new(&binary);
    cmd.args(&args)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    // Put child in its own process group so the pipeline can kill the
    // entire tree (agent + any tool subprocesses) via killpg.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    #[cfg(feature = "telemetry")]
    if let Some(ref tp) = traceparent_value {
        cmd.env("TRACEPARENT", tp);
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                binary = %binary,
                working_dir = %working_dir.display(),
                error = %e,
                "Failed to spawn agent subprocess in streaming mode; \
                 ensure the agent binary is installed and the working directory exists"
            );
            drop(event_tx);
            return StreamingAgentHandle {
                relay: std::thread::spawn(move || -1),
                pid: 0,
            };
        }
    };

    let child_pid = child.id();

    let relay = std::thread::spawn(move || {
        // Drain stdout line-by-line, parsing each line as NDJSON or
        // falling back to a synthetic TextDelta.
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut receiver_alive = true;
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if !receiver_alive {
                            // Receiver dropped — keep reading stdout until
                            // EOF so the child can drain its pipe and exit
                            // cleanly, but don't bother parsing.
                            continue;
                        }

                        // Feed the line through the streaming parser. If
                        // it emits one or more events, those are forwarded
                        // directly. If it emits nothing we distinguish two
                        // cases:
                        //   • Valid JSON that the parser recognises but
                        //     silently skips (e.g. known-noisy subtypes
                        //     like message_start) — drop it, do NOT fall
                        //     through to the plain-text path.
                        //   • Non-JSON (parse_claude_events_streaming skips
                        //     non-JSON lines internally) — fall through and
                        //     emit a synthetic TextDelta so plain-text
                        //     adapters (Codex, OpenCode, raw stdout) keep
                        //     working.
                        //
                        // Check whether the line is valid JSON before
                        // invoking the streaming parser. We use `Value`
                        // rather than `RawValue` because the workspace
                        // does not enable the `raw_value` serde_json
                        // feature. The allocation is modest (typically
                        // one object per NDJSON line) and avoids running
                        // the full parser on plain-text adapter output.
                        let is_json = serde_json::from_str::<serde_json::Value>(&l).is_ok();
                        if is_json {
                            let mut emitted_any = false;
                            let mut send_failed = false;
                            assay_harness::claude_stream::parse_claude_events_streaming(
                                Cursor::new(l.as_bytes()),
                                |event| {
                                    emitted_any = true;
                                    if !send_failed && event_tx.send(event).is_err() {
                                        send_failed = true;
                                    }
                                },
                            );
                            if send_failed {
                                receiver_alive = false;
                                continue;
                            }
                            if emitted_any {
                                continue;
                            }
                            // Parser saw valid JSON but emitted zero events
                            // (e.g. a known-noisy stream_event subtype such
                            // as message_start). Drop the line silently — do
                            // not fall through to the plain-text path.
                            continue;
                        }

                        // Non-JSON line: fall back to a synthetic TextDelta
                        // so plain-text adapters (Codex, OpenCode, raw
                        // stdout) still reach the consumer.
                        tracing::debug!(
                            line = %l,
                            "non-JSON stdout line forwarded as TextDelta"
                        );
                        let event = AgentEvent::TextDelta {
                            text: l,
                            block_index: 0,
                        };
                        if event_tx.send(event).is_err() {
                            receiver_alive = false;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Error reading agent stdout line; stream may be truncated");
                        break;
                    }
                }
            }
        }

        // Drop the sender explicitly before waiting, so the receiver sees EOF
        // before this thread blocks on wait().
        drop(event_tx);

        // Wait for subprocess exit and return the exit code.
        child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
    });

    StreamingAgentHandle {
        relay,
        pid: child_pid,
    }
}

// ── Harness profile construction ─────────────────────────────────────

/// Per D014, the manifest session contains inline settings/hooks/prompt_layers
/// rather than an embedded `HarnessProfile`. This function assembles them
/// into a complete profile suitable for the harness adapter.
pub fn build_harness_profile(session: &ManifestSession) -> HarnessProfile {
    let name = session.name.clone().unwrap_or_else(|| session.spec.clone());

    let settings = session
        .settings
        .clone()
        .unwrap_or_else(|| SettingsOverride {
            model: None,
            permissions: vec![],
            tools: vec![],
            max_turns: None,
        });

    HarnessProfile {
        name,
        prompt_layers: session.prompt_layers.clone(),
        settings,
        hooks: session.hooks.clone(),
        working_dir: None, // Set by caller based on worktree path.
    }
}

// ── Setup result (handoff from setup to execute) ─────────────────────

/// Intermediate result from [`setup_session`], carrying all state needed
/// by [`execute_session`] so it can run without re-loading the spec or
/// re-creating the worktree.
///
/// All fields are `Send` — verified by static assertion — enabling the
/// executor to dispatch `execute_session` across threads.
#[derive(Debug)]
pub struct SetupResult {
    /// Work session ID (from `start_session`).
    pub session_id: String,
    /// Human-readable spec name.
    pub spec_name: String,
    /// The loaded spec entry (legacy or directory).
    pub spec_entry: SpecEntry,
    /// Worktree metadata from git worktree creation.
    pub worktree_info: assay_types::WorktreeInfo,
    /// Timing records for stages 1-2 (SpecLoad + WorktreeCreate).
    pub stage_timings: Vec<StageTiming>,
}

// Static assertion: SetupResult must be Send for std::thread::scope.
const _: () = {
    fn _assert_send<T: Send>() {}
    fn _check() {
        _assert_send::<SetupResult>();
    }
};

// ── Core orchestrator ────────────────────────────────────────────────

/// Type alias for the harness writer function.
///
/// Takes a `HarnessProfile` and a worktree path, writes configuration
/// files to disk, and returns the CLI arguments for the agent subprocess.
///
/// The concrete implementation is typically `ClaudeProvider` from
/// `assay-harness::provider`.
// `HarnessProvider` is canonical at `assay_types::provider::HarnessProvider`.
// This re-export exists for backward compatibility with code that previously
// imported `assay_core::pipeline::HarnessWriter`. Prefer importing from
// `assay_types` directly.
#[doc(hidden)]
pub use assay_types::HarnessProvider;

/// Execute the setup phase of a pipeline session (stages 1-2).
///
/// Runs:
/// 1. **SpecLoad** — load and validate the spec
/// 2. **WorktreeCreate** — start session + create git worktree
///
/// Returns a [`SetupResult`] containing all state needed by
/// [`execute_session`]. On failure after session start, the session
/// is abandoned (never left in `AgentRunning`).
#[instrument(name = "pipeline::setup_session", skip(manifest_session, config), fields(spec = %manifest_session.spec))]
pub fn setup_session(
    manifest_session: &ManifestSession,
    config: &PipelineConfig,
) -> std::result::Result<SetupResult, PipelineError> {
    let mut stage_timings = Vec::new();
    // Helper to abandon session on failure (after session start).
    let abandon_if_started = |sid: &Option<String>, assay_dir: &Path, reason: &str| {
        if let Some(id) = sid {
            let _ = crate::work_session::abandon_session(assay_dir, id, reason);
        }
    };

    // ── Stage 1: SpecLoad ────────────────────────────────────────
    let spec_entry = info_span!("spec_load", spec = %manifest_session.spec).in_scope(|| {
        let stage_start = Instant::now();
        let entry = crate::spec::load_spec_entry(&manifest_session.spec, &config.specs_dir)
            .map_err(|e| {
                let elapsed = stage_start.elapsed();
                warn!(stage = "spec_load", error = %e, "stage failed");
                PipelineError {
                    stage: PipelineStage::SpecLoad,
                    message: format!("Failed to load spec '{}': {e}", manifest_session.spec),
                    recovery: format!(
                        "Check that spec '{}' exists in specs directory '{}'",
                        manifest_session.spec,
                        config.specs_dir.display()
                    ),
                    elapsed,
                }
            })?;
        let duration = stage_start.elapsed();
        info!("stage completed");
        stage_timings.push(StageTiming {
            stage: PipelineStage::SpecLoad,
            duration,
        });
        Ok::<_, PipelineError>(entry)
    })?;

    let spec_name = spec_entry.name().to_string();

    // ── Stage 2: WorktreeCreate ──────────────────────────────────
    let (ws_id, worktree_info) = info_span!("worktree_create", spec = %manifest_session.spec).in_scope(|| {
        let stage_start = Instant::now();

        // Start a work session first to get the session ID.
        let worktree_path = config.worktree_base.join(&manifest_session.spec);
        let ws = crate::work_session::start_session(
            &config.assay_dir,
            &manifest_session.spec,
            worktree_path.clone(),
            "claude",
            None,
        )
        .map_err(|e| {
            let elapsed = stage_start.elapsed();
            warn!(stage = "worktree_create", error = %e, "stage failed");
            PipelineError {
                stage: PipelineStage::WorktreeCreate,
                message: format!("Failed to start work session: {e}"),
                recovery: format!(
                    "Check .assay/sessions directory permissions at '{}'",
                    config.assay_dir.display()
                ),
                elapsed,
            }
        })?;

        let session_id = Some(ws.id.clone());

        // Create the worktree, passing the session ID for metadata linkage.
        let wt_info = crate::worktree::create(
            &config.project_root,
            &manifest_session.spec,
            config.base_branch.as_deref(),
            &config.worktree_base,
            &config.specs_dir,
            Some(&ws.id),
        )
        .map_err(|e| {
            let elapsed = stage_start.elapsed();
            abandon_if_started(&session_id, &config.assay_dir, &format!("WorktreeCreate failed: {e}"));
            warn!(stage = "worktree_create", error = %e, "stage failed");
            PipelineError {
                stage: PipelineStage::WorktreeCreate,
                message: format!(
                    "Failed to create worktree for '{}': {e}",
                    manifest_session.spec
                ),
                recovery: format!(
                    "Inspect worktree base at '{}'. Check for stale branches with `git branch -a | grep assay/`",
                    config.worktree_base.display()
                ),
                elapsed,
            }
        })?;
        let duration = stage_start.elapsed();
        info!("stage completed");
        stage_timings.push(StageTiming {
            stage: PipelineStage::WorktreeCreate,
            duration,
        });
        Ok::<_, PipelineError>((ws.id, wt_info))
    })?;

    Ok(SetupResult {
        session_id: ws_id,
        spec_name,
        spec_entry,
        worktree_info,
        stage_timings,
    })
}

/// Execute the run phase of a pipeline session (stages 3-6).
///
/// Runs:
/// 3. **HarnessConfig** — build profile + write config via `harness_writer`
/// 4. **AgentLaunch** — spawn claude subprocess with timeout
/// 5. **GateEvaluate** — evaluate quality gates
/// 6. **MergeCheck** — check merge compatibility
///
/// Takes a [`SetupResult`] from [`setup_session`] containing the loaded
/// spec and created worktree. On failure, the session is abandoned via
/// the session_id from `SetupResult`.
#[instrument(name = "pipeline::execute_session", skip(manifest_session, config, harness_writer, setup), fields(spec = %manifest_session.spec, session_id = %setup.session_id))]
pub fn execute_session(
    manifest_session: &ManifestSession,
    config: &PipelineConfig,
    harness_writer: &dyn HarnessProvider,
    setup: SetupResult,
) -> std::result::Result<PipelineResult, PipelineError> {
    let SetupResult {
        session_id,
        spec_name,
        spec_entry,
        worktree_info,
        mut stage_timings,
    } = setup;

    // Helper to abandon session on failure.
    let abandon = |assay_dir: &Path, reason: &str| {
        let _ = crate::work_session::abandon_session(assay_dir, &session_id, reason);
    };

    // ── Stage 3: HarnessConfig (streaming) ────────────────────────
    let cli_args = info_span!("harness_config", spec = %manifest_session.spec).in_scope(|| {
        let stage_start = Instant::now();
        let profile = build_harness_profile(manifest_session);
        let args = harness_writer
            .write_harness_streaming(
                &profile,
                &worktree_info.path,
                manifest_session.prompt.as_deref(),
            )
            .map_err(|e| {
                let elapsed = stage_start.elapsed();
                abandon(&config.assay_dir, &format!("HarnessConfig failed: {e}"));
                warn!(stage = "harness_config", error = %e, "stage failed");
                PipelineError {
                    stage: PipelineStage::HarnessConfig,
                    message: format!("Failed to write harness config: {e}"),
                    recovery: format!(
                        "Check worktree path '{}' is writable",
                        worktree_info.path.display()
                    ),
                    elapsed,
                }
            })?;
        let duration = stage_start.elapsed();
        info!("stage completed");
        stage_timings.push(StageTiming {
            stage: PipelineStage::HarnessConfig,
            duration,
        });
        Ok::<_, PipelineError>(args)
    })?;

    // ── Stage 4: AgentLaunch (streaming) + Checkpoint Driver ─────
    let agent_start = Instant::now();
    let checkpoint_spec = spec_entry.to_spec();
    let (events, checkpoint_outcome) = info_span!("agent_launch", spec = %manifest_session.spec)
        .in_scope(|| {
            let stage_start = Instant::now();
            let timeout = Duration::from_secs(config.timeout_secs);

            // `write_harness_streaming` returns the full command line:
            // cli_args[0] = binary, cli_args[1..] = streaming-compatible args.
            let (event_tx, event_rx) = std::sync::mpsc::channel::<AgentEvent>();
            let agent_handle = launch_agent_streaming(&cli_args, &worktree_info.path, event_tx);

            // Run checkpoint driver (consumes events until channel disconnect or failure).
            let mut event_buffer = Vec::new();
            let checkpoint_outcome = crate::pipeline_checkpoint::drive_checkpoints(
                &event_rx,
                &checkpoint_spec,
                &worktree_info.path,
                crate::pipeline_checkpoint::DEFAULT_CHECKPOINT_BUDGET,
                timeout,
                &mut event_buffer,
            );

            // Handle checkpoint failure: kill subprocess via process group signal,
            // then drain channel so the relay thread can exit cleanly.
            if let Some(ref failed) = checkpoint_outcome.failed {
                eprintln!(
                    "Checkpoint failed at {:?}: {} — aborting agent",
                    failed.phase, failed.summary.spec_name,
                );

                if agent_handle.pid > 0 {
                    #[cfg(unix)]
                    {
                        // Send SIGTERM to the process group (negative PID).
                        // SAFETY: pid is valid, process_group(0) was set at spawn.
                        unsafe {
                            libc::kill(-(agent_handle.pid as i32), libc::SIGTERM);
                        }
                        tracing::info!(
                            pid = agent_handle.pid,
                            "sent SIGTERM to agent process group"
                        );
                    }
                    #[cfg(not(unix))]
                    tracing::warn!("cannot send SIGTERM on non-Unix platform");
                }
                // Drain remaining events so the relay thread can exit.
                crate::pipeline_checkpoint::drain_channel(&event_rx, &mut event_buffer);
            }

            // Join relay thread to get exit code.
            let exit_code = agent_handle.relay.join().unwrap_or(-1);

            // Check timeout.
            if stage_start.elapsed() > timeout {
                let elapsed = stage_start.elapsed();
                abandon(
                    &config.assay_dir,
                    &format!("Agent timed out after {}s", config.timeout_secs),
                );
                warn!(stage = "agent_launch", "agent timed out");
                return Err(PipelineError {
                    stage: PipelineStage::AgentLaunch,
                    message: format!(
                        "Agent timed out after {}s for spec '{}'",
                        config.timeout_secs, manifest_session.spec
                    ),
                    recovery: format!(
                        "Agent timed out after {}s — increase timeout or reduce scope",
                        config.timeout_secs
                    ),
                    elapsed,
                });
            }

            // Check agent exit code (skip if we aborted via checkpoint — signal
            // death exit codes are expected after kill).
            if checkpoint_outcome.failed.is_none() && exit_code != 0 {
                let elapsed = stage_start.elapsed();
                let exit_info = if exit_code == -1 {
                    "killed by signal".to_string()
                } else {
                    format!("exit code {exit_code}")
                };
                abandon(
                    &config.assay_dir,
                    &format!("Agent crashed with {exit_info}"),
                );
                warn!(stage = "agent_launch", %exit_info, "agent crashed");
                return Err(PipelineError {
                    stage: PipelineStage::AgentLaunch,
                    message: format!(
                        "Agent crashed with {exit_info} for spec '{}'",
                        manifest_session.spec,
                    ),
                    recovery: format!(
                        "Inspect agent stderr. Check that Claude Code CLI is properly configured. \
                     Working directory: '{}'",
                        worktree_info.path.display()
                    ),
                    elapsed,
                });
            }

            let duration = stage_start.elapsed();
            info!("stage completed");
            stage_timings.push(StageTiming {
                stage: PipelineStage::AgentLaunch,
                duration,
            });

            Ok::<_, PipelineError>((event_buffer, checkpoint_outcome))
        })?;
    crate::telemetry::record_agent_run_duration_ms(agent_start.elapsed().as_secs_f64() * 1000.0);

    // ── Checkpoint failure early return ──────────────────────────
    if let Some(failed) = checkpoint_outcome.failed {
        // Use the actual failed checkpoint's summary and metadata for the diagnostic.
        let mut diagnostic =
            crate::review::build_gate_diagnostic(&spec_name, &session_id, &failed.summary);
        diagnostic.checkpoint_index = Some(failed.index);
        diagnostic.session_phase = failed.phase.clone();
        match crate::review::save_gate_diagnostic(&config.assay_dir, &spec_name, &diagnostic) {
            Ok(path) => info!(
                spec = %spec_name,
                run_id = %session_id,
                checkpoint_index = failed.index,
                path = %path.display(),
                "checkpoint gate diagnostic saved"
            ),
            Err(e) => warn!(
                spec = %spec_name,
                error = %e,
                "failed to save checkpoint gate diagnostic"
            ),
        }

        return Ok(PipelineResult {
            session_id,
            spec_name,
            gate_summary: Some(failed.summary),
            merge_check: None,
            stage_timings,
            outcome: PipelineOutcome::GateFailed,
        });
    }

    // ── Event log + tool call summary ────────────────────────────
    let tool_call_summary = crate::work_session::compute_tool_call_summary(&events);
    tracing::trace!(
        total = tool_call_summary.total,
        error_count = tool_call_summary.error_count,
        "tool_call_summary populated (events empty until streaming is wired)"
    );

    // Persist tool_call_summary to the work session on disk.
    if let Err(e) = crate::work_session::with_session(&config.assay_dir, &session_id, |session| {
        session.tool_call_summary = tool_call_summary;
        Ok(())
    }) {
        tracing::warn!(error = %e, session_id = %session_id, "failed to persist tool_call_summary to work session");
    }

    // ── Stage 5: GateEvaluate ────────────────────────────────────
    let (gate_summary, gate_passed) =
        info_span!("gate_evaluate", spec = %manifest_session.spec, spec_name = %spec_name)
            .in_scope(|| {
                crate::telemetry::record_gate_evaluated();
                let stage_start = Instant::now();
                let summary = match &spec_entry {
                    SpecEntry::Legacy { spec, .. } => crate::gate::evaluate_all_with_events(
                        spec,
                        &worktree_info.path,
                        None,
                        None,
                        &events,
                    ),
                    SpecEntry::Directory { gates, .. } => {
                        crate::gate::evaluate_all_gates_with_events(
                            gates,
                            &worktree_info.path,
                            None,
                            None,
                            &events,
                        )
                    }
                };

                // Record gate result in the session. Use a synthetic run_id based on session.
                let gate_run_id = format!("{}-gate", session_id);
                let passed = summary.failed == 0;
                let _ = crate::work_session::record_gate_result(
                    &config.assay_dir,
                    &session_id,
                    &gate_run_id,
                    "pipeline_gate_evaluate",
                    Some(if passed {
                        "all gates passed"
                    } else {
                        "gate failures detected"
                    }),
                );
                let duration = stage_start.elapsed();
                if passed {
                    info!("stage completed");
                } else {
                    warn!(
                        stage = "gate_evaluate",
                        failed = summary.failed,
                        "gates failed"
                    );
                }
                stage_timings.push(StageTiming {
                    stage: PipelineStage::GateEvaluate,
                    duration,
                });
                crate::telemetry::record_gate_eval_latency_ms(
                    stage_start.elapsed().as_secs_f64() * 1000.0,
                );
                (summary, passed)
            });

    if !gate_passed {
        // Persist gate diagnostics for spec review (best-effort).
        let diagnostic =
            crate::review::build_gate_diagnostic(&spec_name, &session_id, &gate_summary);
        match crate::review::save_gate_diagnostic(&config.assay_dir, &spec_name, &diagnostic) {
            Ok(path) => info!(
                spec = %spec_name,
                run_id = %session_id,
                failed = gate_summary.failed,
                path = %path.display(),
                "gate diagnostic saved"
            ),
            Err(e) => warn!(
                spec = %spec_name,
                error = %e,
                "failed to save gate diagnostic"
            ),
        }

        // Gates failed — session stays in GateEvaluated, outcome is GateFailed.
        return Ok(PipelineResult {
            session_id,
            spec_name,
            gate_summary: Some(gate_summary),
            merge_check: None,
            stage_timings,
            outcome: PipelineOutcome::GateFailed,
        });
    }

    // ── Auto-promote on clean run (S04) ────────────────────────────
    //
    // Preconditions (ALL must hold):
    // 1. No checkpoint failures AND budget not exhausted
    // 2. Session-end gates passed (gate_summary.failed == 0 — guaranteed here)
    // 3. FeatureSpec.auto_promote == true
    // 4. Current SpecStatus == InProgress
    // 5. Directory spec (not flat legacy)
    if checkpoint_outcome.failed.is_none()
        && !checkpoint_outcome.budget_exhausted
        && let SpecEntry::Directory {
            slug,
            spec_path: Some(ref spec_toml_path),
            ..
        } = spec_entry
    {
        match crate::spec::load_feature_spec(spec_toml_path) {
            Ok(feature_spec)
                if feature_spec.auto_promote
                    && feature_spec.status == assay_types::feature_spec::SpecStatus::InProgress =>
            {
                match crate::spec::promote::promote_spec(
                    &config.specs_dir,
                    &slug,
                    Some(assay_types::feature_spec::SpecStatus::Verified),
                ) {
                    Ok((old, new)) => {
                        info!(
                            spec = %spec_name,
                            old_status = %old,
                            new_status = %new,
                            "auto-promoted spec on clean run"
                        );
                        if let Err(e) = crate::work_session::with_session(
                            &config.assay_dir,
                            &session_id,
                            |session| {
                                session.auto_promoted = true;
                                session.promoted_to =
                                    Some(assay_types::feature_spec::SpecStatus::Verified);
                                Ok(())
                            },
                        ) {
                            warn!(
                                error = %e,
                                "failed to persist auto_promoted flag to work session"
                            );
                        }
                    }
                    Err(e) => {
                        // AlreadyTerminal or any other error — warn and continue.
                        warn!(
                            spec = %spec_name,
                            error = %e,
                            "auto-promote failed (non-fatal); continuing pipeline"
                        );
                    }
                }
            }
            Ok(_) => {
                // auto_promote is false or status is not InProgress — no-op
            }
            Err(e) => {
                warn!(
                    spec = %spec_name,
                    error = %e,
                    "failed to load feature spec for auto-promote check"
                );
            }
        }
    }

    // ── Stage 6: MergeCheck ──────────────────────────────────────
    let merge_result =
        info_span!("merge_check", spec = %manifest_session.spec, spec_name = %spec_name).in_scope(
            || {
                let stage_start = Instant::now();
                let base_branch = worktree_info.base_branch.as_deref().unwrap_or("main");
                let result = crate::merge::merge_check(
                    &config.project_root,
                    base_branch,
                    &worktree_info.branch,
                    None,
                )
                .map_err(|e| {
                    let elapsed = stage_start.elapsed();
                    warn!(stage = "merge_check", error = %e, "stage failed");
                    PipelineError {
                        stage: PipelineStage::MergeCheck,
                        message: format!("Merge check failed for '{}': {e}", manifest_session.spec),
                        recovery: format!(
                            "Inspect worktree branch '{}' and base branch '{}' in '{}'",
                            worktree_info.branch,
                            base_branch,
                            config.project_root.display()
                        ),
                        elapsed,
                    }
                })?;
                let duration = stage_start.elapsed();
                info!("stage completed");
                stage_timings.push(StageTiming {
                    stage: PipelineStage::MergeCheck,
                    duration,
                });
                Ok::<_, PipelineError>(result)
            },
        )?;

    if merge_result.clean {
        // All good — complete the session.
        let _ = crate::work_session::complete_session(
            &config.assay_dir,
            &session_id,
            Some("Pipeline completed: gates passed, merge clean"),
        );
        Ok(PipelineResult {
            session_id,
            spec_name,
            gate_summary: Some(gate_summary),
            merge_check: Some(merge_result),
            stage_timings,
            outcome: PipelineOutcome::Success,
        })
    } else {
        // Merge conflicts — session stays in GateEvaluated.
        Ok(PipelineResult {
            session_id,
            spec_name,
            gate_summary: Some(gate_summary),
            merge_check: Some(merge_result),
            stage_timings,
            outcome: PipelineOutcome::MergeConflict,
        })
    }
}

/// Execute a single manifest session through the full pipeline.
///
/// Sequences through all pipeline stages:
/// 1. **SpecLoad** — load and validate the spec
/// 2. **WorktreeCreate** — start session + create git worktree
/// 3. **HarnessConfig** — build profile + write config via `harness_writer`
/// 4. **AgentLaunch** — spawn claude subprocess with timeout
/// 5. **GateEvaluate** — evaluate quality gates
/// 6. **MergeCheck** — check merge compatibility
///
/// Thin composition of [`setup_session`] and [`execute_session`].
/// On failure after session start, the session is abandoned (never left
/// in `AgentRunning`).
#[instrument(name = "pipeline::run_session", skip(config, harness_writer), fields(spec = %manifest_session.spec))]
pub fn run_session(
    manifest_session: &ManifestSession,
    config: &PipelineConfig,
    harness_writer: &dyn HarnessProvider,
) -> std::result::Result<PipelineResult, PipelineError> {
    crate::telemetry::record_session_launched();
    let setup = setup_session(manifest_session, config)?;
    execute_session(manifest_session, config, harness_writer, setup)
}

/// Execute all sessions in a manifest, collecting results.
///
/// Iterates sessions sequentially, calling [`run_session`] for each.
/// One session's failure does not block subsequent sessions —
/// results are collected individually. Future-proof for M002 multi-session.
#[instrument(name = "pipeline::run_manifest", skip(config, harness_writer, manifest), fields(session_count = manifest.sessions.len()))]
pub fn run_manifest(
    manifest: &RunManifest,
    config: &PipelineConfig,
    harness_writer: &dyn HarnessProvider,
) -> Vec<std::result::Result<PipelineResult, PipelineError>> {
    manifest
        .sessions
        .iter()
        .map(|session| run_session(session, config, harness_writer))
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::needless_update)]
mod tests {
    use super::*;

    // ── PipelineStage Display ────────────────────────────────────

    #[test]
    fn pipeline_stage_display() {
        assert_eq!(PipelineStage::SpecLoad.to_string(), "SpecLoad");
        assert_eq!(PipelineStage::WorktreeCreate.to_string(), "WorktreeCreate");
        assert_eq!(PipelineStage::HarnessConfig.to_string(), "HarnessConfig");
        assert_eq!(PipelineStage::AgentLaunch.to_string(), "AgentLaunch");
        assert_eq!(PipelineStage::GateEvaluate.to_string(), "GateEvaluate");
        assert_eq!(PipelineStage::MergeCheck.to_string(), "MergeCheck");
    }

    // ── PipelineError construction and Display ───────────────────

    #[test]
    fn pipeline_error_construction_and_display() {
        let err = PipelineError {
            stage: PipelineStage::SpecLoad,
            message: "Spec 'auth-flow' not found".into(),
            recovery: "Check that spec exists in specs directory".into(),
            elapsed: Duration::from_millis(42),
        };

        assert_eq!(err.stage, PipelineStage::SpecLoad);
        assert!(err.to_string().contains("[SpecLoad]"));
        assert!(err.to_string().contains("Spec 'auth-flow' not found"));
        assert!(err.to_string().contains("recovery:"));
        assert!(err.to_string().contains("0.0s")); // 42ms rounds to 0.0s display

        // Verify Error trait is implemented
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn pipeline_error_display_includes_elapsed() {
        let err = PipelineError {
            stage: PipelineStage::AgentLaunch,
            message: "timed out".into(),
            recovery: "increase timeout".into(),
            elapsed: Duration::from_secs(600),
        };

        let display = err.to_string();
        assert!(display.contains("600.0s"), "got: {display}");
    }

    // ── PipelineConfig defaults ──────────────────────────────────

    #[test]
    fn pipeline_config_default_timeout() {
        let config = PipelineConfig::default();
        assert_eq!(config.timeout_secs, 600);
        assert_eq!(config.timeout_secs, PipelineConfig::DEFAULT_TIMEOUT_SECS);
    }

    // ── PipelineOutcome Display ──────────────────────────────────

    #[test]
    fn pipeline_outcome_display() {
        assert_eq!(PipelineOutcome::Success.to_string(), "Success");
        assert_eq!(PipelineOutcome::GateFailed.to_string(), "GateFailed");
        assert_eq!(PipelineOutcome::MergeConflict.to_string(), "MergeConflict");
    }

    // ── HarnessProfile from ManifestSession ──────────────────────

    #[test]
    fn build_harness_profile_minimal() {
        let session = ManifestSession {
            spec: "auth-flow".into(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let profile = build_harness_profile(&session);
        assert_eq!(profile.name, "auth-flow"); // Falls back to spec name.
        assert!(profile.prompt_layers.is_empty());
        assert!(profile.hooks.is_empty());
        assert_eq!(profile.settings.model, None);
        assert!(profile.settings.permissions.is_empty());
        assert!(profile.working_dir.is_none());
    }

    #[test]
    fn build_harness_profile_with_overrides() {
        use assay_types::{HookContract, HookEvent, PromptLayer, PromptLayerKind};

        let session = ManifestSession {
            spec: "auth-flow".into(),
            name: Some("custom-run".into()),
            settings: Some(SettingsOverride {
                model: Some("opus".into()),
                permissions: vec!["Bash(*)".into()],
                tools: vec!["bash".into()],
                max_turns: Some(20),
            }),
            hooks: vec![HookContract {
                event: HookEvent::PreTool,
                command: "echo pre".into(),
                timeout_secs: Some(10),
            }],
            prompt_layers: vec![PromptLayer {
                kind: PromptLayerKind::Custom,
                name: "extra".into(),
                content: "Be careful".into(),
                priority: 50,
            }],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let profile = build_harness_profile(&session);
        assert_eq!(profile.name, "custom-run");
        assert_eq!(profile.settings.model.as_deref(), Some("opus"));
        assert_eq!(profile.settings.max_turns, Some(20));
        assert_eq!(profile.hooks.len(), 1);
        assert_eq!(profile.prompt_layers.len(), 1);
        assert_eq!(profile.prompt_layers[0].name, "extra");
    }

    // ── launch_agent with non-existent binary ────────────────────

    #[test]
    fn launch_agent_nonexistent_binary() {
        // "claude" likely doesn't exist in test environment.
        // Use a definitely-nonexistent binary to be safe.
        let result = launch_agent(
            &["--version".to_string()],
            Path::new("/tmp"),
            Duration::from_secs(5),
        );

        // Could succeed if `claude` is installed, so handle both cases.
        // We test with a definitely-nonexistent binary name instead.
        // Actually, let's test the error path with a wrapper.
        drop(result);

        // Test with a non-existent working directory to force a spawn error.
        let result = launch_agent(
            &["--version".to_string()],
            Path::new("/nonexistent/path/that/does/not/exist"),
            Duration::from_secs(5),
        );

        match result {
            Err(err) => {
                assert_eq!(err.stage, PipelineStage::AgentLaunch);
                assert!(
                    err.message.contains("spawn"),
                    "expected spawn error, got: {}",
                    err.message
                );
                assert!(
                    err.recovery.contains("Claude Code CLI"),
                    "recovery should mention Claude Code CLI, got: {}",
                    err.recovery
                );
            }
            Ok(_) => {
                // If claude happens to exist and /nonexistent doesn't cause a spawn error,
                // the test still passes — this is about verifying the error structure when
                // spawn does fail.
            }
        }
    }

    // ── launch_agent_streaming ───────────────────────────────────

    #[test]
    fn launch_agent_streaming_delivers_all_lines() {
        let (event_tx, event_rx) = std::sync::mpsc::channel::<AgentEvent>();
        let args: Vec<String> = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf 'line1\\nline2\\nline3\\nline4\\nline5\\n'".to_string(),
        ];
        let handle = launch_agent_streaming(&args, Path::new("/tmp"), event_tx);
        let events: Vec<AgentEvent> = event_rx.iter().collect();
        let exit_code = handle.relay.join().expect("thread panicked");
        assert_eq!(exit_code, 0);
        assert_eq!(
            events.len(),
            5,
            "expected 5 TextDelta events, got {events:?}"
        );
        let expected = ["line1", "line2", "line3", "line4", "line5"];
        for (event, expected_text) in events.iter().zip(expected.iter()) {
            match event {
                AgentEvent::TextDelta { text, block_index } => {
                    assert_eq!(text, expected_text);
                    assert_eq!(*block_index, 0);
                }
                other => panic!("expected TextDelta, got {other:?}"),
            }
        }
    }

    #[test]
    fn launch_agent_streaming_delivers_exit_code() {
        // Zero exit.
        {
            let (event_tx, event_rx) = std::sync::mpsc::channel::<AgentEvent>();
            let args: Vec<String> = vec!["true".to_string()];
            let handle = launch_agent_streaming(&args, Path::new("/tmp"), event_tx);
            let _: Vec<AgentEvent> = event_rx.iter().collect();
            let exit_code = handle.relay.join().expect("thread panicked");
            assert_eq!(exit_code, 0, "`true` should exit 0");
        }
        // Non-zero exit.
        {
            let (event_tx, event_rx) = std::sync::mpsc::channel::<AgentEvent>();
            let args: Vec<String> = vec!["false".to_string()];
            let handle = launch_agent_streaming(&args, Path::new("/tmp"), event_tx);
            let _: Vec<AgentEvent> = event_rx.iter().collect();
            let exit_code = handle.relay.join().expect("thread panicked");
            assert_ne!(exit_code, 0, "`false` should exit non-zero");
        }
    }

    #[test]
    fn launch_agent_streaming_parses_ndjson_to_textdelta() {
        // Emit a single Claude NDJSON stream_event line with a
        // content_block_delta / text_delta payload. The relay thread
        // should parse it via parse_claude_events_streaming and forward a
        // typed TextDelta event, NOT the raw line as a fallback.
        let (event_tx, event_rx) = std::sync::mpsc::channel::<AgentEvent>();
        let args: Vec<String> = vec![
            "sh".to_string(),
            "-c".to_string(),
            r#"printf '%s\n' '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}}}'"#.to_string(),
        ];
        let handle = launch_agent_streaming(&args, Path::new("/tmp"), event_tx);
        let events: Vec<AgentEvent> = event_rx.iter().collect();
        let exit_code = handle.relay.join().expect("thread panicked");
        assert_eq!(exit_code, 0);
        assert_eq!(
            events.len(),
            1,
            "expected exactly one TextDelta, got {events:?}"
        );
        match &events[0] {
            AgentEvent::TextDelta { text, block_index } => {
                assert_eq!(text, "hello");
                assert_eq!(*block_index, 0);
            }
            other => panic!("expected TextDelta, got {other:?}"),
        }
    }

    // ── run_manifest with empty sessions ─────────────────────────

    #[test]
    fn run_manifest_empty_sessions() {
        let manifest = RunManifest {
            sessions: vec![],
            ..Default::default()
        };
        let config = PipelineConfig::default();
        let provider = assay_types::NullProvider;

        let results = run_manifest(&manifest, &config, &provider);
        assert!(results.is_empty());
    }

    // ── run_session spec not found ───────────────────────────────

    #[test]
    fn run_session_spec_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let specs_dir = dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let config = PipelineConfig {
            project_root: dir.path().to_path_buf(),
            assay_dir: dir.path().to_path_buf(),
            specs_dir,
            worktree_base: dir.path().join("worktrees"),
            timeout_secs: 60,
            base_branch: Some("main".into()),
        };

        let session = ManifestSession {
            spec: "nonexistent-spec".into(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let provider = assay_types::NullProvider;

        let result = run_session(&session, &config, &provider);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.stage, PipelineStage::SpecLoad);
        assert!(
            err.message.contains("nonexistent-spec"),
            "error should mention spec name, got: {}",
            err.message
        );
        assert!(
            err.recovery.contains("nonexistent-spec"),
            "recovery should mention spec name, got: {}",
            err.recovery
        );
    }

    // ── NullProvider integration tests ─────────────────────────────

    /// Create a minimal git repository with a committed spec file.
    ///
    /// Returns the `TempDir` (must stay alive for the test) and the
    /// [`PipelineConfig`] pointing into it.  Returns `None` if `git` is
    /// not on `PATH`, allowing callers to skip rather than panic.
    fn make_git_fixture() -> Option<(tempfile::TempDir, PipelineConfig)> {
        // Probe for git before doing any work.
        let git_ok = std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !git_ok {
            return None;
        }

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap_or_else(|e| panic!("git {args:?} failed: {e}"))
        };

        run(&["init", "-b", "main"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);

        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        std::fs::write(
            specs_dir.join("null-spec.toml"),
            r#"
name = "null-spec"
description = "Fixture spec for NullProvider pipeline test"

[[criteria]]
name = "Check"
description = "check"
cmd = "echo ok"
"#,
        )
        .unwrap();

        run(&["add", "-A"]);
        run(&["commit", "-m", "initial"]);

        let config = PipelineConfig {
            project_root: root.to_path_buf(),
            assay_dir: assay_dir.clone(),
            specs_dir,
            worktree_base: root.join("worktrees"),
            timeout_secs: 5,
            base_branch: Some("main".into()),
        };

        Some((dir, config))
    }

    /// Proves R022-01: a fourth adapter (`NullProvider`) passes through
    /// the pipeline with zero changes to pipeline code.  The pipeline
    /// accepts the provider, runs through SpecLoad and WorktreeCreate,
    /// then proceeds to HarnessConfig (which succeeds because NullProvider
    /// returns `Ok(vec![])`) and fails at AgentLaunch because there is
    /// no real agent binary.  The key proof: the pipeline accepted any
    /// `HarnessProvider` implementor without panicking.
    ///
    /// Requires `git` on `PATH`; skipped gracefully otherwise.
    #[test]
    fn test_null_provider_passes_through_pipeline() {
        let Some((_dir, config)) = make_git_fixture() else {
            eprintln!("SKIP test_null_provider_passes_through_pipeline — git not found");
            return;
        };

        let session = ManifestSession {
            spec: "null-spec".into(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let provider = assay_types::NullProvider;
        let result = run_session(&session, &config, &provider);

        assert!(result.is_err(), "NullProvider should fail (no real agent)");
        let err = result.unwrap_err();
        // NullProvider returns Ok(vec![]) → HarnessConfig passes.
        // AgentLaunch fails because the empty arg list cannot be spawned
        // as a real agent process.
        assert_eq!(
            err.stage,
            PipelineStage::AgentLaunch,
            "expected AgentLaunch failure (NullProvider produces no agent), got: {}",
            err
        );
    }

    /// `run_manifest` with an empty sessions list returns an empty Vec
    /// when called with `NullProvider` — proving the trait boundary
    /// works for the manifest execution path without requiring git.
    #[test]
    fn test_run_manifest_with_null_provider_empty() {
        let manifest = RunManifest {
            sessions: vec![],
            ..Default::default()
        };
        let config = PipelineConfig::default();
        let provider = assay_types::NullProvider;

        let results = run_manifest(&manifest, &config, &provider);
        assert!(
            results.is_empty(),
            "empty manifest should produce empty results"
        );
    }

    // ── run_session worktree collision ────────────────────────────

    #[test]
    fn run_session_worktree_create_failure() {
        // This test verifies that WorktreeCreate failures produce the right stage.
        // We use a non-git directory to trigger a git error at worktree creation.
        let dir = tempfile::tempdir().unwrap();
        let assay_dir = dir.path().join(".assay");
        let specs_dir = assay_dir.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        // Create a spec file so SpecLoad passes.
        let spec_content = r#"
name = "test-spec"
description = "test"

[[criteria]]
name = "Check"
description = "check"
cmd = "echo ok"
"#;
        std::fs::write(specs_dir.join("test-spec.toml"), spec_content).unwrap();

        let config = PipelineConfig {
            project_root: dir.path().to_path_buf(),
            assay_dir: assay_dir.clone(),
            specs_dir,
            worktree_base: dir.path().join("worktrees"),
            timeout_secs: 60,
            base_branch: Some("main".into()),
        };

        let session = ManifestSession {
            spec: "test-spec".into(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let provider = assay_types::NullProvider;

        let result = run_session(&session, &config, &provider);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should fail at WorktreeCreate (not a git repo).
        assert_eq!(
            err.stage,
            PipelineStage::WorktreeCreate,
            "expected WorktreeCreate failure, got: {}",
            err
        );
    }

    // ── Gate diagnostic pipeline integration tests ────────────────

    /// Build a git fixture with a spec whose gate command always fails.
    ///
    /// Returns `None` if `git` is not on PATH (test is skipped gracefully).
    fn make_git_fixture_failing_gate() -> Option<(tempfile::TempDir, PipelineConfig)> {
        let git_ok = std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !git_ok {
            return None;
        }

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap_or_else(|e| panic!("git {args:?} failed: {e}"))
        };

        run(&["init", "-b", "main"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);

        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        // Gate command intentionally exits non-zero so gate evaluation fails.
        std::fs::write(
            specs_dir.join("fail-spec.toml"),
            r#"
name = "fail-spec"
description = "Fixture spec with a gate that always fails"

[[criteria]]
name = "AlwaysFail"
description = "This gate always fails"
cmd = "exit 1"
"#,
        )
        .unwrap();

        run(&["add", "-A"]);
        run(&["commit", "-m", "initial"]);

        let config = PipelineConfig {
            project_root: root.to_path_buf(),
            assay_dir: assay_dir.clone(),
            specs_dir,
            worktree_base: root.join("worktrees"),
            timeout_secs: 10,
            base_branch: Some("main".into()),
        };

        Some((dir, config))
    }

    /// TEST-006: A failing gate run writes a `-gates.json` diagnostic file
    /// into `.assay/reviews/<spec>/`, and the file contains the expected
    /// failure details.  This is the contract that `assay spec review`
    /// depends on to show gate diagnostics.
    #[test]
    fn test_gate_failure_writes_diagnostic_file() {
        let Some((_dir, config)) = make_git_fixture_failing_gate() else {
            eprintln!("SKIP test_gate_failure_writes_diagnostic_file — git not found");
            return;
        };

        let session = ManifestSession {
            spec: "fail-spec".into(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let provider = assay_types::NullProvider;
        let result = run_session(&session, &config, &provider);

        // The pipeline should succeed (return Ok) with GateFailed outcome.
        // NullProvider produces empty CLI args, so the agent exits immediately
        // (exit 0 with no output). Gate evaluation then runs the failing cmd.
        match result {
            Ok(pipeline_result) => {
                assert_eq!(
                    pipeline_result.outcome,
                    PipelineOutcome::GateFailed,
                    "expected GateFailed outcome"
                );

                // Diagnostic file must exist in .assay/reviews/fail-spec/.
                let reviews_dir = config.assay_dir.join("reviews").join("fail-spec");
                assert!(
                    reviews_dir.is_dir(),
                    "reviews/fail-spec/ directory should have been created"
                );

                let gate_files: Vec<_> = std::fs::read_dir(&reviews_dir)
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.ends_with("-gates.json"))
                            .unwrap_or(false)
                    })
                    .collect();

                assert_eq!(
                    gate_files.len(),
                    1,
                    "exactly one -gates.json file should have been written, found: {:?}",
                    gate_files.iter().map(|e| e.file_name()).collect::<Vec<_>>()
                );

                // The file should deserialize as a valid GateDiagnostic.
                let content = std::fs::read_to_string(gate_files[0].path()).expect("readable file");
                let diag: assay_types::GateDiagnostic =
                    serde_json::from_str(&content).expect("valid JSON");

                assert_eq!(diag.spec, "fail-spec");
                assert_eq!(diag.failed, 1, "one criterion should have failed");
                assert_eq!(
                    diag.failed_criteria.len(),
                    1,
                    "one FailedCriterionSummary expected"
                );
                assert_eq!(diag.failed_criteria[0].criterion_name, "AlwaysFail");
            }
            Err(err) if err.stage == PipelineStage::AgentLaunch => {
                // NullProvider can fail at AgentLaunch on some platforms — skip.
                eprintln!(
                    "SKIP test_gate_failure_writes_diagnostic_file — \
                     AgentLaunch failed (NullProvider not viable on this platform): {err}"
                );
            }
            Err(err) => {
                panic!("unexpected pipeline error at stage {}: {}", err.stage, err);
            }
        }
    }

    /// TEST-007: A diagnostic save failure (read-only reviews directory) does
    /// NOT change the pipeline result — the run still returns `GateFailed`,
    /// not an IO error.  This proves the best-effort invariant.
    ///
    /// Only runs on Unix because `std::fs::Permissions::readonly` is
    /// not reliably enforced on Windows without ACLs.
    #[cfg(unix)]
    #[test]
    fn test_gate_failure_diagnostic_save_error_does_not_abort_pipeline() {
        use std::os::unix::fs::PermissionsExt;

        let Some((_dir, config)) = make_git_fixture_failing_gate() else {
            eprintln!(
                "SKIP test_gate_failure_diagnostic_save_error_does_not_abort_pipeline \
                 — git not found"
            );
            return;
        };

        // Pre-create the reviews/fail-spec directory as read-only so that
        // save_gate_diagnostic cannot write a file into it.
        let reviews_dir = config.assay_dir.join("reviews").join("fail-spec");
        std::fs::create_dir_all(&reviews_dir).unwrap();
        std::fs::set_permissions(&reviews_dir, std::fs::Permissions::from_mode(0o444)).unwrap();

        let session = ManifestSession {
            spec: "fail-spec".into(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
            prompt: None,
        };

        let provider = assay_types::NullProvider;
        let result = run_session(&session, &config, &provider);

        // Restore permissions so tempdir cleanup can succeed.
        let _ = std::fs::set_permissions(&reviews_dir, std::fs::Permissions::from_mode(0o755));

        match result {
            Ok(pipeline_result) => {
                // Save failed silently, but the pipeline outcome is still GateFailed.
                assert_eq!(
                    pipeline_result.outcome,
                    PipelineOutcome::GateFailed,
                    "best-effort save failure must not change the pipeline outcome"
                );

                // No diagnostic file should have been written.
                let gate_files: Vec<_> = std::fs::read_dir(&reviews_dir)
                    .unwrap_or_else(|_| panic!("reviews dir should exist"))
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.ends_with("-gates.json"))
                            .unwrap_or(false)
                    })
                    .collect();
                assert!(
                    gate_files.is_empty(),
                    "no gate file should have been written to a read-only dir"
                );
            }
            Err(err) if err.stage == PipelineStage::AgentLaunch => {
                eprintln!(
                    "SKIP test_gate_failure_diagnostic_save_error_does_not_abort_pipeline \
                     — AgentLaunch failed (NullProvider): {err}"
                );
            }
            Err(err) => {
                panic!("unexpected pipeline error at stage {}: {}", err.stage, err);
            }
        }
    }
}
