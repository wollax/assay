use anyhow::{Context, bail};
use assay_core::spec::SpecEntry;
use clap::Subcommand;
use std::path::Path;

use super::{
    ANSI_COLOR_OVERHEAD, assay_dir, colors_enabled, criterion_label, format_count,
    format_duration_ms, format_fail, format_pass, format_relative_timestamp, format_warn,
    gate_kind_label, project_root,
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
            match assay_core::spec::load_spec_entry(&name, &specs_dir) {
                Ok(_) => {}
                Err(assay_core::AssayError::SpecNotFound { .. }) => {
                    bail!("spec '{name}' not found in {}", config.specs_dir);
                }
                Err(e) => return Err(e.into()),
            }
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

/// Streaming display counters accumulated during criterion evaluation.
struct StreamCounters {
    passed: usize,
    failed: usize,
    warned: usize,
    skipped: usize,
}

/// Display configuration for streaming criterion evaluation.
struct StreamConfig {
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
    verbose: bool,
    color: bool,
}

/// Stream a single criterion's evaluation with live "running" -> "PASS/FAIL/WARN" display.
///
/// Enforcement is resolved per-criterion: advisory failures increment `warned` (not `failed`)
/// and display as yellow WARN. Advisory criteria (pass or fail) are labeled `[advisory]`.
fn stream_criterion(
    criterion: &assay_types::Criterion,
    working_dir: &std::path::Path,
    cfg: &StreamConfig,
    counters: &mut StreamCounters,
    gate_section: Option<&assay_types::GateSection>,
) {
    if criterion.cmd.is_none()
        && criterion.path.is_none()
        && criterion.kind != Some(assay_types::CriterionKind::AgentReport)
    {
        counters.skipped += 1;
        return;
    }

    // AgentReport criteria are pending (not evaluable via CLI)
    if criterion.kind == Some(assay_types::CriterionKind::AgentReport) {
        counters.skipped += 1;
        let cr = if cfg.color { "\r\x1b[K" } else { "\r" };
        let label = criterion_label(criterion);
        eprintln!("{cr}  {label} {} ... pending", criterion.name);
        return;
    }

    let enforcement = assay_core::gate::resolve_enforcement(criterion.enforcement, gate_section);
    let is_advisory = enforcement == assay_types::Enforcement::Advisory;

    let label = criterion_label(criterion);
    let cr = if cfg.color { "\r\x1b[K" } else { "\r" };
    eprint!("{cr}  {label} {} ... running", criterion.name);

    let timeout =
        assay_core::gate::resolve_timeout(cfg.cli_timeout, criterion.timeout, cfg.config_timeout);

    let advisory_tag = if is_advisory { " [advisory]" } else { "" };

    match assay_core::gate::evaluate(criterion, working_dir, timeout) {
        Ok(result) => {
            if result.passed {
                counters.passed += 1;
                eprintln!(
                    "{cr}  {label}{advisory_tag} {} ... {}",
                    criterion.name,
                    format_pass(cfg.color)
                );
            } else if is_advisory {
                counters.warned += 1;
                eprintln!(
                    "{cr}  {label}{advisory_tag} {} ... {}",
                    criterion.name,
                    format_warn(cfg.color)
                );
            } else {
                counters.failed += 1;
                eprintln!(
                    "{cr}  {label} {} ... {}",
                    criterion.name,
                    format_fail(cfg.color)
                );
            }

            if !result.passed || cfg.verbose {
                print_evidence(&result.stdout, &result.stderr, result.truncated, cfg.color);
            }
        }
        Err(err) => {
            if is_advisory {
                counters.warned += 1;
                eprintln!(
                    "{cr}  {label}{advisory_tag} {} ... {}",
                    criterion.name,
                    format_warn(cfg.color)
                );
            } else {
                counters.failed += 1;
                eprintln!(
                    "{cr}  {label} {} ... {}",
                    criterion.name,
                    format_fail(cfg.color)
                );
            }
            eprintln!("    error: {err}");
        }
    }
}

/// Print a gate summary line (pass/fail/warn/skip counts).
fn print_gate_summary(counters: &StreamCounters, color: bool, label: &str) {
    let total = counters.passed + counters.failed + counters.warned + counters.skipped;
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
/// Returns 0 if all specs pass, 1 if any spec has required failures.
fn handle_gate_run_all(cli_timeout: Option<u64>, verbose: bool, json: bool) -> anyhow::Result<i32> {
    let (root, config, working_dir, config_timeout) = load_gate_context()?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let result = assay_core::spec::scan(&specs_dir)?;

    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.entries.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No specs found in {}", config.specs_dir);
        }
        return Ok(0);
    }

    let assay_dir = assay_dir(&root);
    let max_history = config.gates.as_ref().and_then(|g| g.max_history);

    if json {
        let summaries: Vec<_> = result
            .entries
            .iter()
            .map(|entry| {
                let summary = match entry {
                    SpecEntry::Legacy { spec, .. } => assay_core::gate::evaluate_all(
                        spec,
                        &working_dir,
                        cli_timeout,
                        config_timeout,
                    ),
                    SpecEntry::Directory { gates, .. } => assay_core::gate::evaluate_all_gates(
                        gates,
                        &working_dir,
                        cli_timeout,
                        config_timeout,
                    ),
                };
                save_run_record(
                    &assay_dir,
                    &summary.spec_name,
                    &working_dir,
                    summary.clone(),
                    max_history,
                    true,
                );
                summary
            })
            .collect();

        let any_failed = summaries.iter().any(|s| s.enforcement.required_failed > 0);

        let output =
            serde_json::to_string_pretty(&summaries).context("failed to serialize gate results")?;
        println!("{output}");

        return Ok(if any_failed { 1 } else { 0 });
    }

    let color = colors_enabled();
    let cfg = StreamConfig {
        cli_timeout,
        config_timeout,
        verbose,
        color,
    };
    let mut counters = StreamCounters {
        passed: 0,
        failed: 0,
        warned: 0,
        skipped: 0,
    };
    let spec_count = result.entries.len();

    for (i, entry) in result.entries.iter().enumerate() {
        if i > 0 {
            eprintln!();
        }
        eprintln!("--- {} ---", entry.slug());

        let gate_section: Option<&assay_types::GateSection> = match entry {
            SpecEntry::Legacy { spec, .. } => spec.gate.as_ref(),
            SpecEntry::Directory { gates, .. } => gates.gate.as_ref(),
        };

        let criteria: Vec<assay_types::Criterion> = match entry {
            SpecEntry::Legacy { spec, .. } => spec.criteria.clone(),
            SpecEntry::Directory { gates, .. } => gates
                .criteria
                .iter()
                .map(assay_core::gate::to_criterion)
                .collect(),
        };

        let executable_count = criteria
            .iter()
            .filter(|c| c.cmd.is_some() || c.path.is_some())
            .count();
        if executable_count == 0 {
            eprintln!("  No executable criteria");
            counters.skipped += criteria.len();
            continue;
        }

        let before_passed = counters.passed;
        let before_failed = counters.failed;
        let before_skipped = counters.skipped;

        for criterion in &criteria {
            stream_criterion(criterion, &working_dir, &cfg, &mut counters, gate_section);
        }

        // Save per-spec history (streaming mode)
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

    print_gate_summary(&counters, color, &format!("Results ({spec_count} specs)"));
    Ok(if counters.failed > 0 { 1 } else { 0 })
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

    let entry = match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(e) => e,
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            bail!("spec '{name}' not found in {}", config.specs_dir);
        }
        Err(e) => return Err(e.into()),
    };

    let assay_dir = assay_dir(&root);
    let max_history = config.gates.as_ref().and_then(|g| g.max_history);

    if json {
        let summary = match &entry {
            SpecEntry::Legacy { spec, .. } => {
                assay_core::gate::evaluate_all(spec, &working_dir, cli_timeout, config_timeout)
            }
            SpecEntry::Directory { gates, .. } => assay_core::gate::evaluate_all_gates(
                gates,
                &working_dir,
                cli_timeout,
                config_timeout,
            ),
        };

        // Save history with full fidelity (includes per-criterion results)
        save_run_record(
            &assay_dir,
            name,
            &working_dir,
            summary.clone(),
            max_history,
            true,
        );

        let output =
            serde_json::to_string_pretty(&summary).context("failed to serialize gate results")?;
        println!("{output}");
        return Ok(if summary.enforcement.required_failed > 0 {
            1
        } else {
            0
        });
    }

    let criteria: Vec<assay_types::Criterion> = match &entry {
        SpecEntry::Legacy { spec, .. } => spec.criteria.clone(),
        SpecEntry::Directory { gates, .. } => gates
            .criteria
            .iter()
            .map(assay_core::gate::to_criterion)
            .collect(),
    };

    let gate_section: Option<&assay_types::GateSection> = match &entry {
        SpecEntry::Legacy { spec, .. } => spec.gate.as_ref(),
        SpecEntry::Directory { gates, .. } => gates.gate.as_ref(),
    };

    let color = colors_enabled();
    let cfg = StreamConfig {
        cli_timeout,
        config_timeout,
        verbose,
        color,
    };
    let mut counters = StreamCounters {
        passed: 0,
        failed: 0,
        warned: 0,
        skipped: 0,
    };
    let executable_count = criteria
        .iter()
        .filter(|c| c.cmd.is_some() || c.path.is_some())
        .count();

    if executable_count == 0 {
        println!("No executable criteria found");
        return Ok(0);
    }

    for criterion in &criteria {
        stream_criterion(criterion, &working_dir, &cfg, &mut counters, gate_section);
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
    Ok(if counters.failed > 0 { 1 } else { 0 })
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
    let timestamp = chrono::Utc::now();
    let run_id = assay_core::history::generate_run_id(&timestamp);
    let record = assay_types::GateRunRecord {
        run_id,
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        working_dir: Some(working_dir.display().to_string()),
        summary,
    };
    match assay_core::history::save(assay_dir, &record, max_history) {
        Ok(result) => {
            if result.pruned > 0 && !suppress_prune_msg {
                eprintln!("Pruned {} old run(s) for {name}", result.pruned);
            }
        }
        Err(e) => {
            eprintln!("Warning: could not save run history: {e}");
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
    match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(_) => {}
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            bail!("spec '{name}' not found in {}", config.specs_dir);
        }
        Err(e) => return Err(e.into()),
    }

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
    let display_ids: Vec<&str> = ids
        .iter()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.as_str())
        .collect();

    // Load all records for display (warn on corrupt files)
    let records: Vec<assay_types::GateRunRecord> = display_ids
        .iter()
        .filter_map(|id| match assay_core::history::load(&assay_dir, name, id) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("Warning: skipping run {id}: {e}");
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
        let status = if s.failed == 0 {
            if color {
                "\x1b[32mpass\x1b[0m".to_string()
            } else {
                "pass".to_string()
            }
        } else if color {
            "\x1b[31mfail\x1b[0m".to_string()
        } else {
            "fail".to_string()
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
    match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(_) => {}
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            bail!("spec '{name}' not found in {}", config.specs_dir);
        }
        Err(e) => return Err(e.into()),
    }

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
                    let truncated: String = evidence.chars().take(200).collect();
                    let suffix = if evidence.len() > 200 { "..." } else { "" };
                    println!("    evidence: {truncated}{suffix}");
                }
                if let Some(ref reasoning) = r.reasoning {
                    let truncated: String = reasoning.chars().take(200).collect();
                    let suffix = if reasoning.len() > 200 { "..." } else { "" };
                    println!("    reasoning: {truncated}{suffix}");
                }
            }
        }
    }
    Ok(0)
}

/// Print evidence (stdout/stderr) for a gate result.
///
/// Multi-line output is indented with 4 spaces per line. If the output
/// was truncated, a note is appended.
fn print_evidence(stdout: &str, stderr: &str, truncated: bool, color: bool) {
    let stdout = stdout.trim();
    let stderr = stderr.trim();

    if !stdout.is_empty() {
        eprintln!("    stdout:");
        for line in stdout.lines() {
            eprintln!("      {line}");
        }
    }
    if !stderr.is_empty() {
        eprintln!("    stderr:");
        for line in stderr.lines() {
            eprintln!("      {line}");
        }
    }
    if truncated {
        let note = if color {
            "\x1b[33m[output truncated]\x1b[0m"
        } else {
            "[output truncated]"
        };
        eprintln!("    {note}");
    }
}
