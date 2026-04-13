use anyhow::{Context, bail};
use assay_core::spec::SpecEntry;
use assay_types::Criterion;
use clap::Subcommand;
use std::path::Path;

/// Maximum number of chars to display for evidence/reasoning fields in gate history detail view.
const EVIDENCE_DISPLAY_CHARS: usize = 200;

use super::{
    ANSI_COLOR_OVERHEAD, assay_dir, colors_enabled, criterion_label, format_count,
    format_duration_ms, format_fail, format_pass, format_relative_timestamp, gate_kind_label,
    project_root,
};

#[derive(Subcommand)]
pub(crate) enum GateCommand {
    /// Run all executable criteria for a spec
    #[command(after_long_help = "\
Examples:
  Run gates for a single spec:
    assay gate run auth-flow

  Run gates for all specs:
    assay gate run --all

  Run with verbose evidence output:
    assay gate run auth-flow --verbose

  Override timeout to 60 seconds:
    assay gate run auth-flow --timeout 60

  Output as JSON:
    assay gate run auth-flow --json

  Run all specs as JSON:
    assay gate run --all --json")]
    Run {
        /// Spec name (filename without .toml extension)
        #[arg(conflicts_with = "all")]
        name: Option<String>,
        /// Run gates for all specs in the project
        #[arg(long)]
        all: bool,
        /// Override timeout for all criteria (seconds)
        #[arg(long)]
        timeout: Option<u64>,
        /// Show evidence for all criteria, not just failures
        #[arg(short, long)]
        verbose: bool,
        /// Output as JSON instead of streaming display
        #[arg(long)]
        json: bool,
    },
    /// Interactively create or edit a gate definition
    Wizard {
        /// Edit an existing gate by slug rather than creating a new one.
        #[arg(long)]
        edit: Option<String>,
    },
    /// View gate run history for a spec
    #[command(after_long_help = "\
Examples:
  Show recent gate runs:
    assay gate history auth-flow

  Show the last 5 runs:
    assay gate history auth-flow --limit 5

  Show details for a specific run:
    assay gate history auth-flow 20260305T143000Z-a3f1b2

  Show details of the most recent run:
    assay gate history auth-flow --last

  Output as JSON:
    assay gate history auth-flow --json")]
    History {
        /// Spec name
        name: String,
        /// Run ID to show in detail (optional)
        run_id: Option<String>,
        /// Show the most recent run in detail
        #[arg(long, conflicts_with = "run_id")]
        last: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Maximum number of runs to display (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

/// Handle gate subcommands.
pub(crate) fn handle(command: GateCommand) -> anyhow::Result<i32> {
    match command {
        GateCommand::Wizard { edit } => handle_wizard(edit),
        GateCommand::Run {
            name: Some(name),
            timeout,
            verbose,
            json,
            ..
        } => handle_gate_run(&name, timeout, verbose, json),
        GateCommand::Run {
            all: true,
            timeout,
            verbose,
            json,
            ..
        } => handle_gate_run_all(timeout, verbose, json),
        GateCommand::Run { .. } => {
            bail!("specify a spec name or use --all");
        }
        GateCommand::History {
            name,
            run_id: Some(rid),
            json,
            ..
        } => handle_gate_history_detail(&name, &rid, json),
        GateCommand::History {
            name,
            last: true,
            json,
            ..
        } => {
            let root = project_root()?;
            let config = assay_core::config::load(&root)?;
            let assay_dir = assay_dir(&root);
            let specs_dir = assay_dir.join(&config.specs_dir);
            assay_core::spec::load_spec_entry_with_diagnostics(&name, &specs_dir)?;
            let ids = assay_core::history::list(&assay_dir, &name)?;
            match ids.last() {
                Some(last_id) => handle_gate_history_detail(&name, last_id, json),
                None => {
                    if json {
                        println!("null");
                    } else {
                        println!("No history for {name}");
                    }
                    Ok(0)
                }
            }
        }
        GateCommand::History {
            name, json, limit, ..
        } => handle_gate_history(&name, json, limit),
    }
}

// ── Internal helpers ──────────────────────────────────────────────

/// Load project config and resolve the shared gate execution context.
/// Returns (root, config, working_dir, config_timeout).
fn load_gate_context() -> anyhow::Result<(
    std::path::PathBuf,
    assay_types::Config,
    std::path::PathBuf,
    Option<u64>,
)> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;

    let working_dir = match config.gates.as_ref().and_then(|g| g.working_dir.as_deref()) {
        Some(dir) => {
            let path = std::path::Path::new(dir);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        }
        None => root.clone(),
    };

    let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);

    Ok((root, config, working_dir, config_timeout))
}

/// Tracks pass/fail/warn/skip counts during streaming gate execution.
#[derive(Default)]
struct StreamCounters {
    /// Number of criteria that passed evaluation.
    passed: usize,
    /// Number of required criteria that failed evaluation.
    failed: usize,
    /// Number of criteria that failed evaluation (advisory enforcement).
    warned: usize,
    /// Number of criteria skipped (agent-report or non-executable).
    skipped: usize,
}

impl StreamCounters {
    /// Total number of criteria processed (passed + failed + warned + skipped).
    fn tally(&self) -> usize {
        self.passed + self.failed + self.warned + self.skipped
    }

    /// Whether any required criteria failed, blocking the gate.
    fn gate_blocked(&self) -> bool {
        self.failed > 0
    }
}

/// Display configuration for streaming criterion evaluation.
struct StreamConfig {
    /// Override timeout in seconds from the CLI `--timeout` flag.
    cli_timeout: Option<u64>,
    /// Config-level default timeout in seconds (used as fallback).
    config_timeout: Option<u64>,
    /// Whether to show evidence for all criteria, not just failures.
    verbose: bool,
    /// Whether to use ANSI color codes in output.
    color: bool,
}

impl StreamConfig {
    /// Create a new `StreamConfig` with the given timeout, verbosity, and color settings.
    fn new(
        cli_timeout: Option<u64>,
        config_timeout: Option<u64>,
        verbose: bool,
        color: bool,
    ) -> Self {
        Self {
            cli_timeout,
            config_timeout,
            verbose,
            color,
        }
    }
}

/// Format a source annotation tag for CLI streaming output.
///
/// Returns an empty string for `Own` criteria (reduces signal-to-noise for the common case).
/// Returns `" [Parent: <slug>]"` for parent-inherited criteria.
/// Returns `" [Library: <slug>]"` for library-included criteria.
fn source_tag(source: Option<&assay_types::CriterionSource>) -> String {
    use assay_types::CriterionSource;
    match source {
        Some(CriterionSource::Parent { gate_slug }) => format!(" [Parent: {gate_slug}]"),
        Some(CriterionSource::Library { slug }) => format!(" [Library: {slug}]"),
        Some(CriterionSource::Own) | None => String::new(),
    }
}

/// Save a precondition-blocked gate run to history.
///
/// Non-fatal: logs warnings on failure rather than propagating errors.
fn save_precondition_blocked_record(
    assay_dir: &std::path::Path,
    spec_name: &str,
    working_dir: &std::path::Path,
    max_history: Option<usize>,
) {
    match assay_core::history::save_blocked_run(
        assay_dir,
        spec_name,
        Some(working_dir.display().to_string()),
        max_history,
    ) {
        Ok(result) => {
            if result.pruned > 0 {
                tracing::info!(pruned = result.pruned, spec_name = %spec_name, "Pruned old run(s)");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Could not save precondition-blocked run history");
        }
    }
}

/// Stream a single criterion's evaluation with live "running" -> "PASS/FAIL/WARN" display.
///
/// Enforcement is resolved per-criterion: advisory failures increment `warned` (not `failed`)
/// and display as yellow WARN. Advisory criteria (pass or fail) are labeled `[advisory]`.
fn stream_criterion(
    criterion: &assay_types::Criterion,
    source: Option<&assay_types::CriterionSource>,
    working_dir: &std::path::Path,
    cfg: &StreamConfig,
    counters: &mut StreamCounters,
    gate_section: Option<&assay_types::GateSection>,
) {
    // Event-based criteria (EventCount, NoToolErrors) have no cmd/path but are
    // evaluable -- fall through to the evaluate() call below which will return
    // Err(InvalidCriterion) for them, surfacing "[events] skipped" rather than
    // silently dropping them. TODO(M024/S02): stream_criterion should accept an
    // events slice and call evaluate_event_criterion directly once the pipeline
    // wires event streaming to the CLI gate run path.
    if criterion.cmd.is_none()
        && criterion.path.is_none()
        && !matches!(
            criterion.kind,
            Some(
                assay_types::CriterionKind::AgentReport
                    | assay_types::CriterionKind::EventCount { .. }
                    | assay_types::CriterionKind::NoToolErrors
            )
        )
    {
        counters.skipped += 1;
        return;
    }

    // AgentReport criteria are pending (not evaluable via CLI gate run).
    if criterion.kind == Some(assay_types::CriterionKind::AgentReport) {
        counters.skipped += 1;
        let label = criterion_label(criterion);
        tracing::info!(criterion_name = %criterion.name, kind = %label, status = "pending", "Criterion pending (agent-report)");
        return;
    }

    // Event-based criteria (EventCount, NoToolErrors) require the live agent
    // event log produced by a running pipeline session. The CLI gate-run path
    // has no event stream, so these are marked pending. Use `assay run` to
    // exercise them through the full pipeline.
    if matches!(
        criterion.kind,
        Some(
            assay_types::CriterionKind::EventCount { .. }
                | assay_types::CriterionKind::NoToolErrors
        )
    ) {
        counters.skipped += 1;
        let label = criterion_label(criterion);
        tracing::info!(criterion_name = %criterion.name, kind = %label, status = "pending", "Criterion pending (event-based, requires pipeline session)");
        return;
    }

    let enforcement = assay_core::gate::resolve_enforcement(criterion.enforcement, gate_section);
    let is_advisory = enforcement == assay_types::Enforcement::Advisory;

    let label = criterion_label(criterion);
    let tag = source_tag(source);
    let cr = if cfg.color { "\r\x1b[K" } else { "\r" };
    eprint!("{cr}  {label} {}{tag} ... running", criterion.name);

    let timeout =
        assay_core::gate::resolve_timeout(cfg.cli_timeout, criterion.timeout, cfg.config_timeout);

    match assay_core::gate::evaluate(criterion, working_dir, timeout) {
        Ok(result) => {
            if result.passed {
                counters.passed += 1;
                tracing::info!(criterion_name = %criterion.name, passed = true, advisory = is_advisory, "Criterion passed");
            } else if is_advisory {
                counters.warned += 1;
                tracing::warn!(criterion_name = %criterion.name, passed = false, advisory = true, "Criterion warned");
            } else {
                counters.failed += 1;
                tracing::error!(criterion_name = %criterion.name, passed = false, "Criterion failed");
            }

            // Show actionable hint for exit code 127/126
            if !result.passed
                && let Some(code) = result.exit_code
                && let Some(kind) = assay_core::gate::classify_exit_code(code)
                && let Some(cmd) = criterion.cmd.as_deref()
            {
                let hint = assay_core::gate::format_command_error(cmd, kind);
                tracing::error!(hint = %hint, "Command error");
            }

            if !result.passed || cfg.verbose {
                print_evidence(&result.stdout, &result.stderr, result.truncated);
            }
        }
        Err(err) => {
            if is_advisory {
                counters.warned += 1;
                tracing::warn!(criterion_name = %criterion.name, advisory = true, error = %assay_core::gate::enriched_error_display(&err, criterion.cmd.as_deref()), "Criterion error (advisory)");
            } else {
                counters.failed += 1;
                tracing::error!(criterion_name = %criterion.name, error = %assay_core::gate::enriched_error_display(&err, criterion.cmd.as_deref()), "Criterion error");
            };
        }
    }
}

/// Returns true if a criterion is executable (has a cmd or path to evaluate).
///
/// See also: [`assay_core::gate::classify_exit_code`]
fn is_executable(criterion: &Criterion) -> bool {
    criterion.cmd.is_some() || criterion.path.is_some()
}

/// Extract gate section and criteria from a `SpecEntry`.
///
/// Returns `(gate_section, criteria)` for use in streaming evaluation.
fn spec_entry_gate_info(
    entry: &SpecEntry,
) -> (
    Option<&assay_types::GateSection>,
    Vec<assay_types::Criterion>,
) {
    match entry {
        SpecEntry::Legacy { spec, .. } => (spec.gate.as_ref(), spec.criteria.clone()),
        SpecEntry::Directory { gates, .. } => (
            gates.gate.as_ref(),
            gates
                .criteria
                .iter()
                .map(assay_core::gate::to_criterion)
                .collect(),
        ),
    }
}

/// Determine the exit code after gate execution.
/// Returns 1 if any required criteria failed (gate blocked), 0 otherwise.
fn gate_exit_code(counters: &StreamCounters) -> i32 {
    if counters.gate_blocked() { 1 } else { 0 }
}

/// Print a gate summary line (pass/fail/warn/skip counts).
fn print_gate_summary(counters: &StreamCounters, color: bool, label: &str) {
    let total = counters.tally();
    let passed_str = format_count(counters.passed, "\x1b[32m", color);
    let failed_str = format_count(counters.failed, "\x1b[31m", color);
    let warned_str = format_count(counters.warned, "\x1b[33m", color);
    let skipped_str = format_count(counters.skipped, "\x1b[33m", color);

    println!();
    println!(
        "{label}: {passed_str} passed, {failed_str} failed, {warned_str} warned, {skipped_str} skipped (of {total} total)"
    );
}

/// Handle `assay gate run --all [--timeout N] [--verbose] [--json]`.
///
/// Scans all specs and runs gates for each, printing results per-spec.
/// Returns:
/// - 0 if all specs pass and none are precondition-blocked
/// - 1 if any required criterion failed
/// - 2 if any spec is precondition-blocked and no spec had gate failures
fn handle_gate_run_all(cli_timeout: Option<u64>, verbose: bool, json: bool) -> anyhow::Result<i32> {
    let (root, config, working_dir, config_timeout) = load_gate_context()?;
    let assay_dir = assay_dir(&root);
    let specs_dir = assay_dir.join(&config.specs_dir);

    let result = assay_core::spec::scan(&specs_dir)?;

    for err in &result.errors {
        tracing::warn!(error = %err, "Spec scan warning");
    }

    if result.entries.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No specs found in {}", config.specs_dir);
        }
        return Ok(0);
    }

    let max_history = config.gates.as_ref().and_then(|g| g.max_history);

    if json {
        let mut outcomes: Vec<assay_types::GateEvalOutcome> = Vec::new();
        let mut any_failed = false;
        let mut any_blocked = false;

        for entry in &result.entries {
            match entry {
                SpecEntry::Legacy { spec, .. } => {
                    let summary = assay_core::gate::evaluate_all(
                        spec,
                        &working_dir,
                        cli_timeout,
                        config_timeout,
                    );
                    if summary.enforcement.required_failed > 0 {
                        any_failed = true;
                    }
                    save_run_record(
                        &assay_dir,
                        &summary.spec_name,
                        &working_dir,
                        summary.clone(),
                        max_history,
                        true,
                    );
                    outcomes.push(assay_types::GateEvalOutcome::Evaluated(summary));
                }
                SpecEntry::Directory { gates, slug, .. } => {
                    // Resolve extends + include
                    let specs_dir_clone = specs_dir.clone();
                    let assay_dir_clone = assay_dir.clone();
                    let resolved = match assay_core::spec::compose::resolve(
                        gates,
                        slug,
                        |parent_slug| {
                            let path = specs_dir_clone.join(parent_slug).join("gates.toml");
                            assay_core::spec::load_gates(&path)
                        },
                        |lib_slug| {
                            assay_core::spec::compose::load_library_by_slug(
                                &assay_dir_clone,
                                lib_slug,
                            )
                        },
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::warn!(spec = %slug, error = %e, "Could not resolve spec — skipping");
                            continue;
                        }
                    };

                    // Check preconditions
                    if let Some(preconditions) = &gates.preconditions {
                        let assay_dir_for_prec = assay_dir.clone();
                        let status = assay_core::gate::check_preconditions(
                            preconditions,
                            |s| assay_core::history::last_gate_passed(&assay_dir_for_prec, s),
                            &working_dir,
                            cli_timeout,
                            config_timeout,
                        );
                        if !status.all_passed() {
                            tracing::warn!(spec = %slug, "Preconditions not met — skipping gate evaluation");
                            save_precondition_blocked_record(
                                &assay_dir,
                                slug,
                                &working_dir,
                                max_history,
                            );
                            any_blocked = true;
                            outcomes.push(assay_types::GateEvalOutcome::PreconditionFailed(status));
                            continue;
                        }
                    }

                    // Evaluate resolved criteria
                    let summary = assay_core::gate::evaluate_all_resolved(
                        slug,
                        &resolved.criteria,
                        gates.gate.as_ref(),
                        &working_dir,
                        cli_timeout,
                        config_timeout,
                    );
                    if summary.enforcement.required_failed > 0 {
                        any_failed = true;
                    }
                    save_run_record(
                        &assay_dir,
                        &summary.spec_name,
                        &working_dir,
                        summary.clone(),
                        max_history,
                        true,
                    );
                    outcomes.push(assay_types::GateEvalOutcome::Evaluated(summary));
                }
            }
        }

        let output =
            serde_json::to_string_pretty(&outcomes).context("failed to serialize gate results")?;
        println!("{output}");

        return Ok(if any_failed {
            1
        } else if any_blocked {
            2
        } else {
            0
        });
    }

    let color = colors_enabled();
    let cfg = StreamConfig::new(cli_timeout, config_timeout, verbose, color);
    let mut counters = StreamCounters::default();
    let spec_count = result.entries.len();
    let mut blocked_count: usize = 0;

    for (i, entry) in result.entries.iter().enumerate() {
        if i > 0 {
            tracing::debug!("---");
        }
        tracing::info!(spec = %entry.slug(), "Running gates for spec");

        match entry {
            SpecEntry::Legacy { .. } => {
                let (gate_section, criteria) = spec_entry_gate_info(entry);
                let executable_count = criteria.iter().filter(|c| is_executable(c)).count();
                if executable_count == 0 {
                    tracing::info!(spec = %entry.slug(), "No executable criteria");
                    counters.skipped += criteria.len();
                    continue;
                }

                let before_passed = counters.passed;
                let before_failed = counters.failed;
                let before_skipped = counters.skipped;

                for criterion in &criteria {
                    stream_criterion(
                        criterion,
                        None,
                        &working_dir,
                        &cfg,
                        &mut counters,
                        gate_section,
                    );
                }

                let spec_passed = counters.passed - before_passed;
                let spec_failed = counters.failed - before_failed;
                let spec_skipped = counters.skipped - before_skipped;
                save_run_record(
                    &assay_dir,
                    entry.slug(),
                    &working_dir,
                    streaming_summary(entry.slug(), spec_passed, spec_failed, spec_skipped),
                    max_history,
                    false,
                );
            }
            SpecEntry::Directory { gates, slug, .. } => {
                // Resolve extends + include
                let specs_dir_clone = specs_dir.clone();
                let assay_dir_clone = assay_dir.clone();
                let resolved = match assay_core::spec::compose::resolve(
                    gates,
                    slug,
                    |parent_slug| {
                        let path = specs_dir_clone.join(parent_slug).join("gates.toml");
                        assay_core::spec::load_gates(&path)
                    },
                    |lib_slug| {
                        assay_core::spec::compose::load_library_by_slug(&assay_dir_clone, lib_slug)
                    },
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(spec = %slug, error = %e, "Could not resolve spec — skipping");
                        continue;
                    }
                };

                // Check preconditions
                if let Some(preconditions) = &gates.preconditions {
                    let assay_dir_for_prec = assay_dir.clone();
                    let status = assay_core::gate::check_preconditions(
                        preconditions,
                        |s| assay_core::history::last_gate_passed(&assay_dir_for_prec, s),
                        &working_dir,
                        cli_timeout,
                        config_timeout,
                    );
                    if !status.all_passed() {
                        tracing::warn!(spec = %slug, "Preconditions not met — skipping gate evaluation");
                        save_precondition_blocked_record(
                            &assay_dir,
                            slug,
                            &working_dir,
                            max_history,
                        );
                        blocked_count += 1;
                        continue;
                    }
                }

                // Stream resolved criteria
                let executable_count = resolved
                    .criteria
                    .iter()
                    .filter(|rc| is_executable(&rc.criterion))
                    .count();
                if executable_count == 0 {
                    tracing::info!(spec = %slug, "No executable criteria");
                    counters.skipped += resolved.criteria.len();
                    continue;
                }

                let before_passed = counters.passed;
                let before_failed = counters.failed;
                let before_skipped = counters.skipped;

                for rc in &resolved.criteria {
                    stream_criterion(
                        &rc.criterion,
                        Some(&rc.source),
                        &working_dir,
                        &cfg,
                        &mut counters,
                        gates.gate.as_ref(),
                    );
                }

                let spec_passed = counters.passed - before_passed;
                let spec_failed = counters.failed - before_failed;
                let spec_skipped = counters.skipped - before_skipped;
                save_run_record(
                    &assay_dir,
                    slug,
                    &working_dir,
                    streaming_summary(slug, spec_passed, spec_failed, spec_skipped),
                    max_history,
                    false,
                );
            }
        }
    }

    print_gate_summary(&counters, color, &format!("Results ({spec_count} specs)"));
    Ok(if counters.gate_blocked() {
        1
    } else if blocked_count > 0 {
        2
    } else {
        0
    })
}

/// Handle `assay gate run <name> [--timeout N] [--verbose] [--json]`.
fn handle_gate_run(
    name: &str,
    cli_timeout: Option<u64>,
    verbose: bool,
    json: bool,
) -> anyhow::Result<i32> {
    let (root, config, working_dir, config_timeout) = load_gate_context()?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;

    let assay_dir = assay_dir(&root);
    let max_history = config.gates.as_ref().and_then(|g| g.max_history);

    if json {
        match &entry {
            SpecEntry::Legacy { spec, .. } => {
                let summary =
                    assay_core::gate::evaluate_all(spec, &working_dir, cli_timeout, config_timeout);
                save_run_record(
                    &assay_dir,
                    name,
                    &working_dir,
                    summary.clone(),
                    max_history,
                    true,
                );
                let output = serde_json::to_string_pretty(&summary)
                    .context("failed to serialize gate results")?;
                println!("{output}");
                return Ok(if summary.enforcement.required_failed > 0 {
                    1
                } else {
                    0
                });
            }
            SpecEntry::Directory { gates, slug, .. } => {
                // Step 1: Resolve extends + include
                let specs_dir_clone = specs_dir.clone();
                let assay_dir_clone = assay_dir.clone();
                let resolved = assay_core::spec::compose::resolve(
                    gates,
                    slug,
                    |parent_slug| {
                        let path = specs_dir_clone.join(parent_slug).join("gates.toml");
                        assay_core::spec::load_gates(&path)
                    },
                    |lib_slug| {
                        assay_core::spec::compose::load_library_by_slug(&assay_dir_clone, lib_slug)
                    },
                )?;

                // Step 2: Check preconditions
                if let Some(preconditions) = &gates.preconditions {
                    let assay_dir_for_prec = assay_dir.clone();
                    let status = assay_core::gate::check_preconditions(
                        preconditions,
                        |s| assay_core::history::last_gate_passed(&assay_dir_for_prec, s),
                        &working_dir,
                        cli_timeout,
                        config_timeout,
                    );
                    if !status.all_passed() {
                        let outcome = assay_types::GateEvalOutcome::PreconditionFailed(status);
                        let output = serde_json::to_string_pretty(&outcome)
                            .context("failed to serialize precondition status")?;
                        println!("{output}");
                        save_precondition_blocked_record(
                            &assay_dir,
                            name,
                            &working_dir,
                            max_history,
                        );
                        return Ok(2);
                    }
                }

                // Step 3: Evaluate resolved criteria
                let summary = assay_core::gate::evaluate_all_resolved(
                    slug,
                    &resolved.criteria,
                    gates.gate.as_ref(),
                    &working_dir,
                    cli_timeout,
                    config_timeout,
                );
                save_run_record(
                    &assay_dir,
                    name,
                    &working_dir,
                    summary.clone(),
                    max_history,
                    true,
                );
                let output = serde_json::to_string_pretty(&summary)
                    .context("failed to serialize gate results")?;
                println!("{output}");
                return Ok(if summary.enforcement.required_failed > 0 {
                    1
                } else {
                    0
                });
            }
        }
    }

    // Streaming mode
    let color = colors_enabled();
    let cfg = StreamConfig::new(cli_timeout, config_timeout, verbose, color);
    let mut counters = StreamCounters::default();

    match &entry {
        SpecEntry::Legacy { .. } => {
            let (gate_section, criteria) = spec_entry_gate_info(&entry);
            let executable_count = criteria.iter().filter(|c| is_executable(c)).count();
            if executable_count == 0 {
                println!("No executable criteria found");
                return Ok(0);
            }
            for criterion in &criteria {
                stream_criterion(
                    criterion,
                    None,
                    &working_dir,
                    &cfg,
                    &mut counters,
                    gate_section,
                );
            }
        }
        SpecEntry::Directory { gates, slug, .. } => {
            // Step 1: Resolve extends + include
            let specs_dir_clone = specs_dir.clone();
            let assay_dir_clone = assay_dir.clone();
            let resolved = assay_core::spec::compose::resolve(
                gates,
                slug,
                |parent_slug| {
                    let path = specs_dir_clone.join(parent_slug).join("gates.toml");
                    assay_core::spec::load_gates(&path)
                },
                |lib_slug| {
                    assay_core::spec::compose::load_library_by_slug(&assay_dir_clone, lib_slug)
                },
            )?;

            // Step 2: Check preconditions
            if let Some(preconditions) = &gates.preconditions {
                let assay_dir_for_prec = assay_dir.clone();
                let status = assay_core::gate::check_preconditions(
                    preconditions,
                    |s| assay_core::history::last_gate_passed(&assay_dir_for_prec, s),
                    &working_dir,
                    cli_timeout,
                    config_timeout,
                );
                if !status.all_passed() {
                    tracing::warn!(spec = %name, "Preconditions not met — skipping gate evaluation");
                    save_precondition_blocked_record(&assay_dir, name, &working_dir, max_history);
                    return Ok(2);
                }
            }

            // Step 3: Stream resolved criteria
            let executable_count = resolved
                .criteria
                .iter()
                .filter(|rc| is_executable(&rc.criterion))
                .count();
            if executable_count == 0 {
                println!("No executable criteria found");
                return Ok(0);
            }
            for rc in &resolved.criteria {
                stream_criterion(
                    &rc.criterion,
                    Some(&rc.source),
                    &working_dir,
                    &cfg,
                    &mut counters,
                    gates.gate.as_ref(),
                );
            }
        }
    }

    // Save history (streaming mode has no per-criterion results)
    save_run_record(
        &assay_dir,
        name,
        &working_dir,
        streaming_summary(name, counters.passed, counters.failed, counters.skipped),
        max_history,
        false,
    );

    print_gate_summary(&counters, color, "Results");
    Ok(gate_exit_code(&counters))
}

/// Build a [`GateRunSummary`] for streaming mode (no per-criterion results or timing).
fn streaming_summary(
    spec_name: &str,
    passed: usize,
    failed: usize,
    skipped: usize,
) -> assay_types::GateRunSummary {
    assay_types::GateRunSummary {
        spec_name: spec_name.to_string(),
        results: Vec::new(),
        passed,
        failed,
        skipped,
        total_duration_ms: 0,
        enforcement: assay_types::EnforcementSummary::default(),
    }
}

/// Build a [`GateRunRecord`] and persist it via [`assay_core::history::save()`].
///
/// Prune messages are printed to stderr unless `suppress_prune_msg` is true (e.g., JSON mode).
/// Save failures are non-fatal warnings.
fn save_run_record(
    assay_dir: &Path,
    name: &str,
    working_dir: &Path,
    summary: assay_types::GateRunSummary,
    max_history: Option<usize>,
    suppress_prune_msg: bool,
) {
    match assay_core::history::save_run(
        assay_dir,
        summary,
        Some(working_dir.display().to_string()),
        max_history,
    ) {
        Ok(result) => {
            if result.pruned > 0 && !suppress_prune_msg {
                tracing::info!(pruned = result.pruned, spec_name = %name, "Pruned old run(s)");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Could not save run history");
        }
    }
}

/// Handle `assay gate history <name> [--json] [--limit N]` — table view.
fn handle_gate_history(name: &str, json: bool, limit: usize) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let assay_dir = assay_dir(&root);

    // Verify spec exists
    let specs_dir = assay_dir.join(&config.specs_dir);
    assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;

    let ids = assay_core::history::list(&assay_dir, name)?;

    if ids.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No history for {name}");
        }
        return Ok(0);
    }

    // Take the last `limit` entries (most recent, since list is sorted oldest-first)
    let start = ids.len().saturating_sub(limit);
    let display_ids: Vec<&str> = ids[start..].iter().map(|s| s.as_str()).collect();

    // Load all records for display (warn on corrupt files)
    let records: Vec<assay_types::GateRunRecord> = display_ids
        .iter()
        .filter_map(|id| match assay_core::history::load(&assay_dir, name, id) {
            Ok(r) => Some(r),
            Err(e) => {
                tracing::warn!(run_id = %id, error = %e, "Skipping corrupt run");
                None
            }
        })
        .collect();

    if json {
        let output = serde_json::to_string_pretty(&records)
            .context("failed to serialize history records")?;
        println!("{output}");
        return Ok(0);
    }

    // Print table
    let color = colors_enabled();
    let num_width = records.len().to_string().len().max(1);

    println!(
        "  {:<nw$}  {:<16}  {:<6}  {:>6}  {:>6}  {:>7}  {:>10}  {:>10}  {:>8}",
        "#",
        "Timestamp",
        "Status",
        "Passed",
        "Failed",
        "Skipped",
        "Req Failed",
        "Adv Failed",
        "Duration",
        nw = num_width,
    );
    println!(
        "  {:<nw$}  {:<16}  {:<6}  {:>6}  {:>6}  {:>7}  {:>10}  {:>10}  {:>8}",
        "\u{2500}".repeat(num_width),
        "\u{2500}".repeat(16),
        "\u{2500}".repeat(6),
        "\u{2500}".repeat(6),
        "\u{2500}".repeat(6),
        "\u{2500}".repeat(7),
        "\u{2500}".repeat(10),
        "\u{2500}".repeat(10),
        "\u{2500}".repeat(8),
        nw = num_width,
    );

    for (i, record) in records.iter().enumerate() {
        let s = &record.summary;
        let ts = format_relative_timestamp(&record.timestamp);
        let (status_plain, status_color_code) = if s.failed == 0 {
            ("pass", "\x1b[32m")
        } else {
            ("fail", "\x1b[31m")
        };
        let status = if color {
            format!("{status_color_code}{status_plain}\x1b[0m")
        } else {
            status_plain.to_string()
        };
        let dur = format_duration_ms(s.total_duration_ms);

        let status_width = if color { 6 + ANSI_COLOR_OVERHEAD } else { 6 };

        println!(
            "  {:<nw$}  {:<16}  {:<sw$}  {:>6}  {:>6}  {:>7}  {:>10}  {:>10}  {:>8}",
            i + 1,
            ts,
            status,
            s.passed,
            s.failed,
            s.skipped,
            s.enforcement.required_failed,
            s.enforcement.advisory_failed,
            dur,
            nw = num_width,
            sw = status_width,
        );
    }
    Ok(0)
}

/// Handle `assay gate history <name> <run-id> [--json]` — detail view.
fn handle_gate_history_detail(name: &str, run_id: &str, json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let assay_dir = assay_dir(&root);

    // Verify spec exists
    let specs_dir = assay_dir.join(&config.specs_dir);
    assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;

    let record = assay_core::history::load(&assay_dir, name, run_id)?;

    if json {
        let output =
            serde_json::to_string_pretty(&record).context("failed to serialize history record")?;
        println!("{output}");
        return Ok(0);
    }

    let s = &record.summary;
    let color = colors_enabled();
    let overall = if s.failed == 0 {
        format_pass(color)
    } else {
        format_fail(color)
    };

    println!("Run: {}", record.run_id);
    println!("Spec: {}", s.spec_name);
    println!("Time: {}", record.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("Duration: {}", format_duration_ms(s.total_duration_ms));
    println!("Status: {overall}");
    println!("Assay: {}", record.assay_version);
    if let Some(ref wd) = record.working_dir {
        println!("Working dir: {wd}");
    }
    println!();
    println!(
        "Summary: {} passed, {} failed, {} skipped",
        s.passed, s.failed, s.skipped
    );
    println!(
        "Enforcement: {} req passed, {} req failed, {} adv passed, {} adv failed",
        s.enforcement.required_passed,
        s.enforcement.required_failed,
        s.enforcement.advisory_passed,
        s.enforcement.advisory_failed
    );

    if !s.results.is_empty() {
        println!();
        println!("Criteria:");
        for cr in &s.results {
            let status_str = match &cr.result {
                Some(r) if r.passed => format_pass(color),
                Some(_) => format_fail(color),
                None => "skip",
            };
            let kind_label = cr
                .result
                .as_ref()
                .map(|r| gate_kind_label(&r.kind))
                .unwrap_or("");
            let enforcement_label = match cr.enforcement {
                assay_types::Enforcement::Required => "req",
                assay_types::Enforcement::Advisory => "adv",
            };
            let duration_str = cr
                .result
                .as_ref()
                .map(|r| format_duration_ms(r.duration_ms))
                .unwrap_or_default();
            println!(
                "  {kind_label} {} ... {} [{}] {}",
                cr.criterion_name, status_str, enforcement_label, duration_str
            );

            // Show agent evaluation fields when present
            if let Some(ref r) = cr.result {
                if let Some(ref role) = r.evaluator_role {
                    println!("    evaluator: {role:?}");
                }
                if let Some(ref confidence) = r.confidence {
                    println!("    confidence: {confidence:?}");
                }
                if let Some(ref evidence) = r.evidence {
                    let truncated: String = evidence.chars().take(EVIDENCE_DISPLAY_CHARS).collect();
                    let suffix = if evidence.chars().count() > EVIDENCE_DISPLAY_CHARS {
                        "..."
                    } else {
                        ""
                    };
                    println!("    evidence: {truncated}{suffix}");
                }
                if let Some(ref reasoning) = r.reasoning {
                    let truncated: String =
                        reasoning.chars().take(EVIDENCE_DISPLAY_CHARS).collect();
                    let suffix = if reasoning.chars().count() > EVIDENCE_DISPLAY_CHARS {
                        "..."
                    } else {
                        ""
                    };
                    println!("    reasoning: {truncated}{suffix}");
                }
            }
        }
    }
    Ok(0)
}

// ── Gate wizard ───────────────────────────────────────────────────

/// Handle `assay gate wizard [--edit <slug>]`.
///
/// Requires an interactive TTY. Returns `Ok(1)` immediately if stdin is not
/// a terminal. All persistence and validation are delegated to
/// `assay_core::wizard::apply_gate_wizard` — no validation logic lives here.
pub(crate) fn handle_wizard(edit: Option<String>) -> anyhow::Result<i32> {
    use std::io::IsTerminal as _;

    if !std::io::stdin().is_terminal() {
        tracing::error!(
            "assay gate wizard requires an interactive terminal. \
             For non-interactive authoring, use the gate wizard MCP tool."
        );
        return Ok(1);
    }

    // ── Resolve project paths ────────────────────────────────────────────────
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let assay_dir = assay_dir(&root);
    let specs_dir = assay_dir.join(&config.specs_dir);

    // ── Edit mode: load existing GatesSpec to pre-fill defaults ─────────────
    let existing: Option<assay_types::GatesSpec> = match &edit {
        Some(slug) => Some(load_gate_for_edit(slug, &specs_dir)?),
        None => None,
    };

    // ── Slug (fixed in edit mode) ────────────────────────────────────────────
    let slug = match &edit {
        Some(slug) => slug.clone(),
        None => super::wizard_helpers::prompt_slug("Gate name (slug)", None)?,
    };

    // ── Description ─────────────────────────────────────────────────────────
    let description: String = {
        let existing_desc = existing.as_ref().map(|g| g.description.as_str());
        let mut b = dialoguer::Input::<String>::new()
            .with_prompt("Description")
            .allow_empty(true);
        if let Some(d) = existing_desc.filter(|d| !d.is_empty()) {
            b = b.with_initial_text(d);
        }
        b.interact_text()?
    };

    // ── Extends: select from available gates, or "(none)" ───────────────────
    let scan = assay_core::spec::scan(&specs_dir)?;
    let mut gate_options: Vec<String> = scan
        .entries
        .iter()
        .map(|e| e.slug().to_string())
        .filter(|s| {
            edit.as_ref()
                .map(|edit_slug| s != edit_slug)
                .unwrap_or(true)
        })
        .collect();
    gate_options.insert(0, "(none)".to_string());

    let default_extends_idx = existing
        .as_ref()
        .and_then(|g| g.extends.as_ref())
        .and_then(|e| gate_options.iter().position(|o| o == e))
        .unwrap_or(0);

    let extends_idx = super::wizard_helpers::select_from_list(
        "Extends (parent gate)",
        &gate_options,
        default_extends_idx,
    )?;
    let extends = if extends_idx == 0 {
        None
    } else {
        Some(gate_options[extends_idx].clone())
    };

    // ── Include: multi-select criteria libraries ─────────────────────────────
    let libs = assay_core::spec::compose::scan_libraries(&assay_dir)?;
    let lib_names: Vec<String> = libs.iter().map(|l| l.name.clone()).collect();
    let preselected: Vec<usize> = existing
        .as_ref()
        .map(|g| {
            g.include
                .iter()
                .filter_map(|name| lib_names.iter().position(|n| n == name))
                .collect()
        })
        .unwrap_or_default();
    let include_indices = super::wizard_helpers::multi_select_from_list(
        "Include criteria libraries",
        &lib_names,
        &preselected,
    )?;
    let include: Vec<String> = include_indices
        .iter()
        .map(|&i| lib_names[i].clone())
        .collect();

    // ── Criteria: inline add-another loop ───────────────────────────────────
    let existing_criteria: Vec<assay_types::CriterionInput> = existing
        .as_ref()
        .map(|g| {
            g.criteria
                .iter()
                .map(|c| assay_types::CriterionInput {
                    name: c.name.clone(),
                    description: c.description.clone(),
                    cmd: c.cmd.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let criteria = super::wizard_helpers::prompt_criteria_loop(&existing_criteria)?;

    // ── Preconditions: opt-in section ────────────────────────────────────────
    let has_existing_preconditions = existing
        .as_ref()
        .map(|g| g.preconditions.is_some())
        .unwrap_or(false);
    let add_preconditions = dialoguer::Confirm::new()
        .with_prompt("Add preconditions?")
        .default(has_existing_preconditions)
        .interact()?;
    let preconditions = if add_preconditions {
        Some(prompt_preconditions(
            &specs_dir,
            existing.as_ref().and_then(|g| g.preconditions.as_ref()),
        )?)
    } else {
        None
    };

    // ── Final confirmation ───────────────────────────────────────────────────
    let write = dialoguer::Confirm::new()
        .with_prompt("Write gate?")
        .default(true)
        .interact()?;
    if !write {
        println!("  aborted (no file written)");
        return Ok(0);
    }

    // ── Build input and delegate all logic to core ───────────────────────────
    let input = assay_types::GateWizardInput {
        slug: slug.clone(),
        description: if description.is_empty() {
            None
        } else {
            Some(description)
        },
        extends,
        include,
        criteria,
        preconditions,
        overwrite: edit.is_some(),
    };

    let output = assay_core::wizard::apply_gate_wizard(&input, &assay_dir, &specs_dir)?;
    println!(
        "  {} gate '{slug}'",
        if edit.is_some() { "Updated" } else { "Created" }
    );
    println!("    written {}", output.path.display());
    Ok(0)
}

/// Load an existing `GatesSpec` for use in edit-mode pre-fill.
///
/// Returns a user-facing error for legacy flat-file specs and a fuzzy
/// not-found diagnostic when the slug does not exist.
fn load_gate_for_edit(
    slug: &str,
    specs_dir: &std::path::Path,
) -> anyhow::Result<assay_types::GatesSpec> {
    use assay_core::spec::SpecEntry;

    let entry = assay_core::spec::load_spec_entry_with_diagnostics(slug, specs_dir)
        .map_err(anyhow::Error::from)?;

    match entry {
        SpecEntry::Directory { gates, .. } => Ok(gates),
        SpecEntry::Legacy { .. } => {
            anyhow::bail!(
                "gate '{}' uses the legacy flat-file format; \
                 gate wizard only supports directory-based specs",
                slug
            )
        }
    }
}

/// Prompt for a `SpecPreconditions` block.
///
/// `existing` is used to pre-select the `requires` specs and pre-populate
/// the commands list.
fn prompt_preconditions(
    specs_dir: &std::path::Path,
    existing: Option<&assay_types::SpecPreconditions>,
) -> anyhow::Result<assay_types::SpecPreconditions> {
    let scan = assay_core::spec::scan(specs_dir)?;
    let slugs: Vec<String> = scan.entries.iter().map(|e| e.slug().to_string()).collect();

    let preselected: Vec<usize> = existing
        .map(|p| {
            p.requires
                .iter()
                .filter_map(|r| slugs.iter().position(|s| s == r))
                .collect()
        })
        .unwrap_or_default();

    let requires_indices =
        super::wizard_helpers::multi_select_from_list("Requires specs", &slugs, &preselected)?;
    let requires = requires_indices.iter().map(|&i| slugs[i].clone()).collect();

    let mut commands: Vec<String> = existing.map(|p| p.commands.clone()).unwrap_or_default();
    loop {
        let add = dialoguer::Confirm::new()
            .with_prompt("  Add a precondition command?")
            .default(commands.is_empty())
            .interact()?;
        if !add {
            break;
        }
        let cmd: String = dialoguer::Input::new()
            .with_prompt("    Command")
            .interact_text()?;
        commands.push(cmd);
    }

    Ok(assay_types::SpecPreconditions { requires, commands })
}

/// Emit stdout/stderr evidence as `tracing::debug!` events.
///
/// Multi-line content is passed as-is in the `output` field.
/// A separate event is emitted when the output was truncated by the caller.
fn print_evidence(stdout: &str, stderr: &str, truncated: bool) {
    let stdout = stdout.trim();
    let stderr = stderr.trim();

    if !stdout.is_empty() {
        tracing::debug!(stream = "stdout", output = %stdout, "Criterion evidence");
    }
    if !stderr.is_empty() {
        tracing::debug!(stream = "stderr", output = %stderr, "Criterion evidence");
    }
    if truncated {
        tracing::debug!("Criterion output truncated");
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// In a `cargo test` run stdin is not a TTY, so `handle_wizard(None)` must
    /// return `Ok(1)` immediately without attempting any dialoguer prompts.
    #[test]
    fn handle_wizard_non_tty() {
        let result = handle_wizard(None).expect("handle_wizard should not return Err in non-TTY");
        assert_eq!(result, 1, "non-TTY handle_wizard must return exit code 1");
    }

    /// `handle_wizard(Some("..."))` in edit mode also exits early on non-TTY
    /// — the TTY guard fires before spec loading.
    #[test]
    fn handle_wizard_edit_non_tty() {
        let result = handle_wizard(Some("does-not-exist".into()))
            .expect("handle_wizard should not Err on non-TTY even in edit mode");
        assert_eq!(result, 1, "non-TTY edit mode must also return exit code 1");
    }

    /// `load_gate_for_edit` with a slug that does not exist returns an error
    /// whose message contains the "not found" phrasing from the enriched
    /// SpecNotFoundDiagnostic. When a fuzzy match is available ("my-gat" is
    /// 1 edit away from "my-gate"), the message also contains "Did you mean".
    #[test]
    fn handle_wizard_edit_not_found() {
        let tmp = tempfile::TempDir::new().expect("create tempdir");
        let specs_dir = tmp.path();

        // Create a valid directory-based gate named "my-gate" so the scanner
        // can find it and produce a fuzzy suggestion.
        let gate_dir = specs_dir.join("my-gate");
        std::fs::create_dir_all(&gate_dir).unwrap();
        std::fs::write(
            gate_dir.join("gates.toml"),
            "name = \"my-gate\"\ncriteria = []\n",
        )
        .unwrap();

        // "my-gat" is 1 edit away from "my-gate" (distance 1 <= threshold 2).
        let err =
            load_gate_for_edit("my-gat", specs_dir).expect_err("should fail for missing slug");
        let msg = format!("{err}");
        assert!(
            msg.contains("Did you mean") || msg.contains("not found"),
            "error should mention not found or a suggestion, got: {msg}"
        );
    }

    /// `load_gate_for_edit` rejects legacy (flat-file) specs with a clear
    /// user-facing message referencing "legacy flat-file format".
    #[test]
    fn load_gate_for_edit_rejects_legacy() {
        let tmp = tempfile::TempDir::new().expect("create tempdir");
        let specs_dir = tmp.path();

        // Write a valid legacy flat-file spec: <slug>.toml in specs_dir.
        // Spec requires at least one criterion with a cmd (required enforcement).
        std::fs::write(
            specs_dir.join("old-gate.toml"),
            concat!(
                "name = \"old-gate\"\n\n",
                "[[criteria]]\n",
                "name = \"check\"\n",
                "description = \"A check\"\n",
                "cmd = \"echo ok\"\n",
            ),
        )
        .unwrap();

        let err =
            load_gate_for_edit("old-gate", specs_dir).expect_err("should fail for legacy spec");
        let msg = format!("{err}");
        assert!(
            msg.contains("legacy flat-file format"),
            "error should mention legacy flat-file format, got: {msg}"
        );
    }

    // ── source_tag tests ──────────────────────────────────────────────────────

    /// `source_tag` with `CriterionSource::Parent` formats the parent slug.
    #[test]
    fn source_tag_parent() {
        let source = assay_types::CriterionSource::Parent {
            gate_slug: "base-gate".to_string(),
        };
        assert_eq!(source_tag(Some(&source)), " [Parent: base-gate]");
    }

    /// `source_tag` with `CriterionSource::Library` formats the library slug.
    #[test]
    fn source_tag_library() {
        let source = assay_types::CriterionSource::Library {
            slug: "rust-basics".to_string(),
        };
        assert_eq!(source_tag(Some(&source)), " [Library: rust-basics]");
    }

    /// `source_tag` with `CriterionSource::Own` returns an empty string.
    #[test]
    fn source_tag_own() {
        let source = assay_types::CriterionSource::Own;
        assert_eq!(source_tag(Some(&source)), "");
    }

    /// `source_tag` with `None` returns an empty string.
    #[test]
    fn source_tag_none() {
        assert_eq!(source_tag(None), "");
    }

    // ── handle_gate_run integration tests ────────────────────────────────────
    //
    // These tests temporarily change the process CWD (because `handle_gate_run`
    // discovers the project root via `std::env::current_dir()`). Cargo runs tests
    // in parallel by default, so CWD-mutating tests must hold a process-wide mutex.

    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Helper: build a minimal assay project layout in a temp dir.
    ///
    /// Creates:
    ///   <root>/
    ///     .assay/
    ///       config.toml        (project config, required by assay_core::config::load)
    ///       specs/             (specs_dir, default "specs/" relative to .assay/)
    ///
    /// Returns `(TempDir, root_path)`.
    fn setup_assay_project() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::TempDir::new().expect("create tempdir");
        let root = tmp.path().to_path_buf();
        let assay_dir = root.join(".assay");
        std::fs::create_dir_all(assay_dir.join("specs")).unwrap();
        // Minimal config.toml at .assay/config.toml
        // `project_name` is required; specs_dir defaults to "specs/"
        std::fs::write(
            assay_dir.join("config.toml"),
            "project_name = \"test-project\"\n",
        )
        .unwrap();
        (tmp, root)
    }

    /// Helper: write a directory-based gate spec.
    fn write_directory_gate(specs_dir: &std::path::Path, slug: &str, toml_content: &str) {
        let gate_dir = specs_dir.join(slug);
        std::fs::create_dir_all(&gate_dir).unwrap();
        std::fs::write(gate_dir.join("gates.toml"), toml_content).unwrap();
        // Also write a minimal spec.toml required by the validator
        if !gate_dir.join("spec.toml").exists() {
            std::fs::write(
                gate_dir.join("spec.toml"),
                format!("name = \"{slug}\"\ndescription = \"test\"\nstatus = \"active\"\n",),
            )
            .unwrap();
        }
    }

    /// `handle_gate_run` on a Directory spec with `extends` evaluates both parent and own criteria.
    #[test]
    fn handle_gate_run_extends_evaluates_merged_criteria() {
        let (_tmp, root) = setup_assay_project();
        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");

        // Parent gate with one criterion
        write_directory_gate(
            &specs_dir,
            "base-gate",
            concat!(
                "name = \"base-gate\"\n\n",
                "[[criteria]]\n",
                "name = \"parent-check\"\n",
                "description = \"A parent criterion\"\n",
                "cmd = \"echo parent\"\n",
            ),
        );

        // Child gate that extends the parent and adds its own criterion
        write_directory_gate(
            &specs_dir,
            "child-gate",
            concat!(
                "name = \"child-gate\"\n",
                "extends = \"base-gate\"\n\n",
                "[[criteria]]\n",
                "name = \"child-check\"\n",
                "description = \"A child criterion\"\n",
                "cmd = \"echo child\"\n",
            ),
        );

        // Run in JSON mode so we can inspect the summary
        let result = run_handle_gate_run_in_dir(&root, "child-gate", None, false, true);
        assert!(
            result.is_ok(),
            "handle_gate_run should succeed: {:?}",
            result
        );

        // The JSON output should contain both criteria names
        // We verify via history instead since we can't capture stdout here.
        // Check that a history record was saved
        let ids = assay_core::history::list(&assay_dir, "child-gate").unwrap();
        assert!(!ids.is_empty(), "A history record should have been saved");

        let record =
            assay_core::history::load(&assay_dir, "child-gate", ids.last().unwrap()).unwrap();
        assert_eq!(
            record.precondition_blocked, None,
            "Normal run should not be marked precondition_blocked"
        );
        // Both criteria ran (parent-check and child-check)
        assert_eq!(
            record.summary.passed, 2,
            "Both parent + child criteria should pass"
        );
    }

    /// `handle_gate_run` on a Directory spec with `include` evaluates library + own criteria.
    #[test]
    fn handle_gate_run_include_evaluates_library_criteria() {
        let (_tmp, root) = setup_assay_project();
        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");

        // Create criteria library
        let criteria_dir = assay_dir.join("criteria");
        std::fs::create_dir_all(&criteria_dir).unwrap();
        std::fs::write(
            criteria_dir.join("my-lib.toml"),
            concat!(
                "name = \"my-lib\"\n\n",
                "[[criteria]]\n",
                "name = \"lib-check\"\n",
                "description = \"Library criterion\"\n",
                "cmd = \"echo lib\"\n",
            ),
        )
        .unwrap();

        // Gate that includes the library
        write_directory_gate(
            &specs_dir,
            "uses-lib",
            concat!(
                "name = \"uses-lib\"\n",
                "include = [\"my-lib\"]\n\n",
                "[[criteria]]\n",
                "name = \"own-check\"\n",
                "description = \"Own criterion\"\n",
                "cmd = \"echo own\"\n",
            ),
        );

        let result = run_handle_gate_run_in_dir(&root, "uses-lib", None, false, true);
        assert!(
            result.is_ok(),
            "handle_gate_run should succeed: {:?}",
            result
        );

        let ids = assay_core::history::list(&assay_dir, "uses-lib").unwrap();
        assert!(!ids.is_empty(), "A history record should have been saved");
        let record =
            assay_core::history::load(&assay_dir, "uses-lib", ids.last().unwrap()).unwrap();
        assert_eq!(
            record.summary.passed, 2,
            "Both library + own criteria should pass"
        );
    }

    /// `handle_gate_run` on a spec with failing `preconditions.requires` returns exit code 2
    /// and records the run as precondition-blocked in history.
    #[test]
    fn handle_gate_run_precondition_requires_fails_returns_exit_2() {
        let (_tmp, root) = setup_assay_project();
        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");

        // A spec that requires "other-spec" (which has no history)
        write_directory_gate(
            &specs_dir,
            "blocked-spec",
            concat!(
                "name = \"blocked-spec\"\n\n",
                "[preconditions]\n",
                "requires = [\"other-spec\"]\n\n",
                "[[criteria]]\n",
                "name = \"would-run\"\n",
                "description = \"This should not run\"\n",
                "cmd = \"echo ok\"\n",
            ),
        );

        let result = run_handle_gate_run_in_dir(&root, "blocked-spec", None, false, true);
        assert!(
            result.is_ok(),
            "handle_gate_run should not error: {:?}",
            result
        );
        assert_eq!(
            result.unwrap(),
            2,
            "Precondition failure should return exit code 2"
        );

        // A history record should have been saved with precondition_blocked = true
        let ids = assay_core::history::list(&assay_dir, "blocked-spec").unwrap();
        assert!(!ids.is_empty(), "A history record should have been saved");
        let record =
            assay_core::history::load(&assay_dir, "blocked-spec", ids.last().unwrap()).unwrap();
        assert_eq!(
            record.precondition_blocked,
            Some(true),
            "Precondition-blocked run should have precondition_blocked = true"
        );
        assert_eq!(
            record.summary.passed, 0,
            "No criteria should have been evaluated"
        );
    }

    /// `handle_gate_run` on a spec with failing `preconditions.commands` returns exit code 2.
    #[test]
    fn handle_gate_run_precondition_command_fails_returns_exit_2() {
        let (_tmp, root) = setup_assay_project();
        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");

        // A spec with a failing precondition command ("false" always exits non-zero)
        write_directory_gate(
            &specs_dir,
            "cmd-blocked",
            concat!(
                "name = \"cmd-blocked\"\n\n",
                "[preconditions]\n",
                "commands = [\"false\"]\n\n",
                "[[criteria]]\n",
                "name = \"would-run\"\n",
                "description = \"This should not run\"\n",
                "cmd = \"echo ok\"\n",
            ),
        );

        let result = run_handle_gate_run_in_dir(&root, "cmd-blocked", None, false, true);
        assert!(
            result.is_ok(),
            "handle_gate_run should not error: {:?}",
            result
        );
        assert_eq!(
            result.unwrap(),
            2,
            "Failing command precondition should return exit code 2"
        );
    }

    /// `handle_gate_run` on a Legacy spec uses the original evaluation path (unchanged).
    #[test]
    fn handle_gate_run_legacy_spec_works_unchanged() {
        let (_tmp, root) = setup_assay_project();
        let assay_dir = root.join(".assay");
        let specs_dir = assay_dir.join("specs");

        // Write a legacy flat-file spec
        std::fs::write(
            specs_dir.join("legacy.toml"),
            concat!(
                "name = \"legacy\"\n\n",
                "[[criteria]]\n",
                "name = \"check\"\n",
                "description = \"A check\"\n",
                "cmd = \"echo ok\"\n",
            ),
        )
        .unwrap();

        let result = run_handle_gate_run_in_dir(&root, "legacy", None, false, true);
        assert!(result.is_ok(), "Legacy spec should work: {:?}", result);
        assert_eq!(result.unwrap(), 0, "Passing legacy spec should return 0");
    }

    /// `save_precondition_blocked_record` writes a history record with precondition_blocked=true.
    #[test]
    fn save_precondition_blocked_record_writes_history() {
        let tmp = tempfile::TempDir::new().expect("create tempdir");
        let assay_dir = tmp.path();
        let working_dir = tmp.path();

        save_precondition_blocked_record(assay_dir, "my-spec", working_dir, None);

        let ids = assay_core::history::list(assay_dir, "my-spec").unwrap();
        assert_eq!(ids.len(), 1, "One history record should be saved");
        let record = assay_core::history::load(assay_dir, "my-spec", &ids[0]).unwrap();
        assert_eq!(record.precondition_blocked, Some(true));
        assert_eq!(record.summary.passed, 0);
        assert_eq!(record.summary.failed, 0);
    }

    /// Run `handle_gate_run` with the process working directory changed to `root`.
    ///
    /// Acquires `CWD_LOCK` before changing the cwd to prevent data races with
    /// other tests that also modify the process cwd. Restores the original cwd
    /// after the call (best effort).
    fn run_handle_gate_run_in_dir(
        root: &std::path::Path,
        name: &str,
        cli_timeout: Option<u64>,
        verbose: bool,
        json: bool,
    ) -> anyhow::Result<i32> {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_dir = std::env::current_dir().unwrap_or_else(|_| root.to_path_buf());
        std::env::set_current_dir(root).expect("chdir to test root");
        let result = handle_gate_run(name, cli_timeout, verbose, json);
        // Restore cwd (best effort)
        let _ = std::env::set_current_dir(&original_dir);
        result
    }
}
