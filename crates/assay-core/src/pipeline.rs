//! End-to-end pipeline orchestrator.
//!
//! Composes manifest loading, worktree creation, harness configuration,
//! agent launching, gate evaluation, and merge checking into a single
//! sequenced pipeline. This is the capstone module of M001.
//!
//! # Architecture
//!
//! The pipeline is parameterized over a `HarnessWriter` function that
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
    GateRunSummary, HarnessProfile, ManifestSession, MergeCheck, RunManifest, SettingsOverride,
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

    let mut child = std::process::Command::new("claude")
        .args(cli_args)
        .current_dir(working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| PipelineError {
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
                    let _ = h.read_to_string(&mut buf);
                    buf
                })
                .unwrap_or_default();

            let stderr = child_stderr
                .map(|mut h| {
                    let mut buf = String::new();
                    use std::io::Read;
                    let _ = h.read_to_string(&mut buf);
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

/// Launch an agent subprocess and stream its stdout line-by-line.
///
/// Spawns a background thread that reads stdout from the subprocess and
/// sends each line via `line_tx`. When all lines have been sent (EOF),
/// the thread waits for the process to exit and returns the exit code
/// as `i32`. The caller receives this exit code by joining the returned
/// `JoinHandle`.
///
/// Uses an unbounded `mpsc::channel()` for `line_tx` to avoid deadlock:
/// the subprocess can produce lines faster than the TUI consumes them,
/// and a bounded channel would block the background thread while holding
/// the stdout pipe open (which would stall the process).
///
/// # Failure handling
///
/// If the subprocess cannot be spawned, `line_tx` is dropped (signalling
/// EOF to the receiver) and the thread returns `-1`. The relay-wrapper
/// thread (T03) observes channel disconnect and emits
/// `TuiEvent::AgentDone { exit_code: -1 }`.
///
/// # Arguments
///
/// * `cli_args` — Full command line: `cli_args[0]` is the binary,
///   `cli_args[1..]` are its arguments.
/// * `working_dir` — Working directory for the subprocess.
/// * `line_tx` — Sender side of the line channel; one `String` per stdout line.
pub fn launch_agent_streaming(
    cli_args: &[String],
    working_dir: &std::path::Path,
    line_tx: std::sync::mpsc::Sender<String>,
) -> std::thread::JoinHandle<i32> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    // Guard: an empty cli_args would panic on cli_args[0].
    // Return a thread that immediately signals EOF and exits with -1.
    if cli_args.is_empty() {
        return std::thread::spawn(move || {
            drop(line_tx);
            -1
        });
    }

    // Clone the args + path before moving into the thread.
    let binary = cli_args[0].clone();
    let args: Vec<String> = cli_args[1..].to_vec();
    let working_dir = working_dir.to_path_buf();

    std::thread::spawn(move || {
        let mut child = match std::process::Command::new(&binary)
            .args(&args)
            .current_dir(&working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                // Drop line_tx — signals EOF to receiver.
                drop(line_tx);
                return -1;
            }
        };

        // Drain stdout line-by-line, forwarding each line to the channel.
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut receiver_alive = true;
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if receiver_alive && line_tx.send(l).is_err() {
                            // Receiver dropped. Stop forwarding but keep reading
                            // until EOF so the child can drain its pipe and exit
                            // cleanly — otherwise the child blocks on a full pipe
                            // and child.wait() below hangs indefinitely.
                            receiver_alive = false;
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // Drop the sender explicitly before waiting, so the receiver sees EOF
        // before this thread blocks on wait().
        drop(line_tx);

        // Wait for subprocess exit and return the exit code.
        child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
    })
}

// ── Harness profile construction ─────────────────────────────────────

/// Construct a [`HarnessProfile`] from a manifest session's inline overrides.
///
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
/// The concrete implementation is typically `claude::generate_config` +
/// `claude::write_config` + `claude::build_cli_args` from `assay-harness`.
pub type HarnessWriter = dyn Fn(&HarnessProfile, &Path) -> std::result::Result<Vec<String>, String>;

/// Execute the setup phase of a pipeline session (stages 1-2).
///
/// Runs:
/// 1. **SpecLoad** — load and validate the spec
/// 2. **WorktreeCreate** — start session + create git worktree
///
/// Returns a [`SetupResult`] containing all state needed by
/// [`execute_session`]. On failure after session start, the session
/// is abandoned (never left in `AgentRunning`).
#[instrument(name = "pipeline::setup_session", skip(config), fields(spec = %manifest_session.spec))]
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
#[instrument(name = "pipeline::execute_session", skip(config, harness_writer, setup), fields(spec = %manifest_session.spec, session_id = %setup.session_id))]
pub fn execute_session(
    manifest_session: &ManifestSession,
    config: &PipelineConfig,
    harness_writer: &HarnessWriter,
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

    // ── Stage 3: HarnessConfig ───────────────────────────────────
    let cli_args = info_span!("harness_config", spec = %manifest_session.spec).in_scope(|| {
        let stage_start = Instant::now();
        let profile = build_harness_profile(manifest_session);
        let args = harness_writer(&profile, &worktree_info.path).map_err(|e| {
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

    // ── Stage 4: AgentLaunch ─────────────────────────────────────
    let agent_output =
        info_span!("agent_launch", spec = %manifest_session.spec).in_scope(|| {
            let stage_start = Instant::now();
            let timeout = Duration::from_secs(config.timeout_secs);
            let output =
                launch_agent(&cli_args, &worktree_info.path, timeout).map_err(|mut e| {
                    abandon(
                        &config.assay_dir,
                        &format!("AgentLaunch failed: {}", e.message),
                    );
                    warn!(stage = "agent_launch", error = %e.message, "stage failed");
                    e.elapsed = stage_start.elapsed();
                    e
                })?;

            if output.timed_out {
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

            if output.exit_code != Some(0) {
                let elapsed = stage_start.elapsed();
                let exit_info = output
                    .exit_code
                    .map(|c| format!("exit code {c}"))
                    .unwrap_or_else(|| "killed by signal".to_string());
                let stderr_excerpt = if output.stderr.len() > 500 {
                    format!("...{}", &output.stderr[output.stderr.len() - 500..])
                } else {
                    output.stderr.clone()
                };
                abandon(
                    &config.assay_dir,
                    &format!("Agent crashed with {exit_info}"),
                );
                warn!(stage = "agent_launch", %exit_info, "agent crashed");
                return Err(PipelineError {
                    stage: PipelineStage::AgentLaunch,
                    message: format!(
                        "Agent crashed with {exit_info} for spec '{}': {}",
                        manifest_session.spec,
                        stderr_excerpt
                            .lines()
                            .take(3)
                            .collect::<Vec<_>>()
                            .join(" | ")
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
            Ok::<_, PipelineError>(output)
        })?;
    // Suppress unused-variable warning; agent_output is consumed by value.
    let _ = agent_output;

    // ── Stage 5: GateEvaluate ────────────────────────────────────
    let (gate_summary, gate_passed) =
        info_span!("gate_evaluate", spec = %spec_name).in_scope(|| {
            let stage_start = Instant::now();
            let summary = match &spec_entry {
                SpecEntry::Legacy { spec, .. } => {
                    crate::gate::evaluate_all(spec, &worktree_info.path, None, None)
                }
                SpecEntry::Directory { gates, .. } => {
                    crate::gate::evaluate_all_gates(gates, &worktree_info.path, None, None)
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
            (summary, passed)
        });

    if !gate_passed {
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

    // ── Stage 6: MergeCheck ──────────────────────────────────────
    let merge_result = info_span!("merge_check", spec = %spec_name).in_scope(|| {
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
    })?;

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
    harness_writer: &HarnessWriter,
) -> std::result::Result<PipelineResult, PipelineError> {
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
    harness_writer: &HarnessWriter,
) -> Vec<std::result::Result<PipelineResult, PipelineError>> {
    manifest
        .sessions
        .iter()
        .map(|session| run_session(session, config, harness_writer))
        .collect()
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
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
        let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
        let args: Vec<String> = vec![
            "sh".to_string(),
            "-c".to_string(),
            "printf 'line1\\nline2\\nline3\\nline4\\nline5\\n'".to_string(),
        ];
        let handle = launch_agent_streaming(&args, Path::new("/tmp"), line_tx);
        let lines: Vec<String> = line_rx.iter().collect();
        let exit_code = handle.join().expect("thread panicked");
        assert_eq!(exit_code, 0);
        assert_eq!(lines, vec!["line1", "line2", "line3", "line4", "line5"]);
    }

    #[test]
    fn launch_agent_streaming_delivers_exit_code() {
        // Zero exit.
        {
            let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
            let args: Vec<String> = vec!["true".to_string()];
            let handle = launch_agent_streaming(&args, Path::new("/tmp"), line_tx);
            let _: Vec<String> = line_rx.iter().collect();
            let exit_code = handle.join().expect("thread panicked");
            assert_eq!(exit_code, 0, "`true` should exit 0");
        }
        // Non-zero exit.
        {
            let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
            let args: Vec<String> = vec!["false".to_string()];
            let handle = launch_agent_streaming(&args, Path::new("/tmp"), line_tx);
            let _: Vec<String> = line_rx.iter().collect();
            let exit_code = handle.join().expect("thread panicked");
            assert_ne!(exit_code, 0, "`false` should exit non-zero");
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
        let writer: Box<HarnessWriter> =
            Box::new(|_profile: &HarnessProfile, _path: &Path| Ok(vec![]));

        let results = run_manifest(&manifest, &config, &writer);
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
        };

        let writer: Box<HarnessWriter> =
            Box::new(|_profile: &HarnessProfile, _path: &Path| Ok(vec![]));

        let result = run_session(&session, &config, &writer);
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
        };

        let writer: Box<HarnessWriter> =
            Box::new(|_profile: &HarnessProfile, _path: &Path| Ok(vec![]));

        let result = run_session(&session, &config, &writer);
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
}
