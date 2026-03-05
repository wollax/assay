use assay_core::spec::SpecEntry;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::Path;

/// Extra bytes added by a single ANSI color sequence pair (`\x1b[XXm` ... `\x1b[0m`).
/// `\x1b[32m` = 5 bytes, `\x1b[0m` = 4 bytes, total = 9.
const ANSI_COLOR_OVERHEAD: usize = 9;

#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows",
    long_about = None,
    after_long_help = "\
Examples:
  Initialize a new project:
    assay init
    assay init --name my-project

  List and inspect specs:
    assay spec list
    assay spec show auth-flow
    assay spec show auth-flow --json

  Run quality gates:
    assay gate run auth-flow
    assay gate run auth-flow --verbose
    assay gate run auth-flow --json
    assay gate run --all

  Start the MCP server (for AI agent integration):
    assay mcp serve"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new Assay project in the current directory
    #[command(after_long_help = "\
Examples:
  Create a project using the directory name:
    assay init

  Create a project with a custom name:
    assay init --name my-project")]
    Init {
        /// Override the inferred project name
        #[arg(long)]
        name: Option<String>,
    },
    /// MCP server operations
    #[command(after_long_help = "\
Examples:
  Start the stdio MCP server:
    assay mcp serve")]
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Manage spec files
    #[command(after_long_help = "\
Examples:
  List all specs in the project:
    assay spec list

  Show a spec's criteria as a table:
    assay spec show auth-flow

  Show a spec as JSON (for agent consumption):
    assay spec show auth-flow --json")]
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
    /// Run quality gates for a spec
    #[command(after_long_help = "\
Examples:
  Run gates for a spec:
    assay gate run auth-flow

  Run with verbose output (show all evidence):
    assay gate run auth-flow --verbose

  Run with a custom timeout (seconds):
    assay gate run auth-flow --timeout 60

  Run and output JSON (for agent consumption):
    assay gate run auth-flow --json")]
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
}

#[derive(Subcommand)]
enum McpCommand {
    /// Start the MCP server (stdio transport)
    #[command(after_long_help = "\
Examples:
  Start the server for Claude Code integration:
    assay mcp serve

  Start with debug logging:
    RUST_LOG=debug assay mcp serve")]
    Serve,
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Display a spec's criteria in detail
    #[command(after_long_help = "\
Examples:
  Show criteria as a formatted table:
    assay spec show auth-flow

  Show as JSON (for scripting or agent use):
    assay spec show auth-flow --json")]
    Show {
        /// Spec name (filename without .toml extension)
        name: String,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },
    /// List all available specs
    #[command(after_long_help = "\
Examples:
  List all specs in the project:
    assay spec list")]
    List,
    /// Create a new directory-based spec with template files
    #[command(after_long_help = "\
Examples:
  Create a new feature spec:
    assay spec new auth-flow")]
    New {
        /// Name for the new spec (used as directory name)
        name: String,
    },
}

#[derive(Subcommand)]
enum GateCommand {
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

/// Check whether terminal colors should be used.
///
/// Returns `false` when the `NO_COLOR` environment variable is set
/// (any value, including empty — per <https://no-color.org/>).
fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

/// Format a criterion type label, optionally with ANSI color.
///
/// "executable" (has a `cmd` or `path`) renders green; "descriptive" renders yellow.
fn format_criteria_type(is_executable: bool, color: bool) -> String {
    if is_executable {
        if color {
            "\x1b[32mexecutable\x1b[0m".to_string()
        } else {
            "executable".to_string()
        }
    } else if color {
        "\x1b[33mdescriptive\x1b[0m".to_string()
    } else {
        "descriptive".to_string()
    }
}

/// Format "ok" with optional green color.
fn format_pass(color: bool) -> &'static str {
    if color { "\x1b[32mok\x1b[0m" } else { "ok" }
}

/// Format "FAILED" with optional red color.
fn format_fail(color: bool) -> &'static str {
    if color {
        "\x1b[31mFAILED\x1b[0m"
    } else {
        "FAILED"
    }
}

/// Format a number with optional ANSI color, only applying color when
/// the value is non-zero.
fn format_count(value: usize, ansi_code: &str, color: bool) -> String {
    if color && value > 0 {
        format!("{ansi_code}{value}\x1b[0m")
    } else {
        value.to_string()
    }
}

/// Resolve the project root directory, exiting on failure.
fn project_root() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("Error: could not determine current directory: {e}");
        std::process::exit(1);
    })
}

/// Handle `assay spec show <name> [--json]`.
fn handle_spec_show(name: &str, json: bool) {
    let root = project_root();
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    let specs_dir = root.join(".assay").join(&config.specs_dir);

    let entry = match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(e) => e,
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            eprintln!("Error: spec '{name}' not found in {}", config.specs_dir);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    match entry {
        SpecEntry::Legacy { spec, .. } => {
            if json {
                let output = serde_json::to_string_pretty(&spec).unwrap_or_else(|e| {
                    eprintln!("Error: failed to serialize spec: {e}");
                    std::process::exit(1);
                });
                println!("{output}");
                return;
            }
            print_spec_table(&spec.name, &spec.description, &spec.criteria);
        }
        SpecEntry::Directory {
            gates, spec_path, ..
        } => {
            if json {
                let mut map = serde_json::Map::new();
                map.insert("format".into(), "directory".into());
                map.insert(
                    "gates".into(),
                    serde_json::to_value(&gates).unwrap_or_default(),
                );
                if let Some(ref sp) = spec_path
                    && let Ok(feature_spec) = assay_core::spec::load_feature_spec(sp)
                {
                    map.insert(
                        "feature_spec".into(),
                        serde_json::to_value(&feature_spec).unwrap_or_default(),
                    );
                }
                let output = serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap();
                println!("{output}");
                return;
            }

            println!("Spec: {} [srs]", gates.name);

            // Show feature spec summary if available
            if let Some(ref sp) = spec_path
                && let Ok(feature_spec) = assay_core::spec::load_feature_spec(sp)
            {
                if let Some(ref overview) = feature_spec.overview
                    && !overview.description.is_empty()
                {
                    println!("Description: {}", overview.description);
                }
                if !feature_spec.requirements.is_empty() {
                    println!(
                        "Requirements: {} ({})",
                        feature_spec.requirements.len(),
                        feature_spec
                            .requirements
                            .iter()
                            .map(|r| r.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
            println!();

            // Convert GateCriteria to Criteria for table display
            let criteria: Vec<_> = gates
                .criteria
                .iter()
                .map(assay_core::gate::to_criterion)
                .collect();
            print_criteria_table(&criteria);

            // Show requirement traceability
            let traced: Vec<_> = gates
                .criteria
                .iter()
                .filter(|c| !c.requirements.is_empty())
                .collect();
            if !traced.is_empty() {
                println!();
                println!("Traceability:");
                for c in &traced {
                    println!("  {} → {}", c.name, c.requirements.join(", "));
                }
            }
        }
    }
}

/// Print a spec table header and rows for a list of criteria.
fn print_criteria_table(criteria: &[assay_types::Criterion]) {
    let color = colors_enabled();
    let num_width = criteria.len().to_string().len().max(1);
    let name_width = criteria
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(9)
        .max(9);
    let type_width = 11;

    println!(
        "  {:<num_w$}  {:<name_w$}  {:<type_w$}  Command",
        "#",
        "Criterion",
        "Type",
        num_w = num_width,
        name_w = name_width,
        type_w = type_width,
    );
    println!(
        "  {:<num_w$}  {:<name_w$}  {:<type_w$}  {}",
        "\u{2500}".repeat(num_width),
        "\u{2500}".repeat(name_width),
        "\u{2500}".repeat(type_width),
        "\u{2500}".repeat(7),
        num_w = num_width,
        name_w = name_width,
        type_w = type_width,
    );

    for (i, criterion) in criteria.iter().enumerate() {
        let type_label =
            format_criteria_type(criterion.cmd.is_some() || criterion.path.is_some(), color);
        let cmd_display = criterion
            .cmd
            .as_deref()
            .or(criterion.path.as_deref())
            .unwrap_or("");

        if color {
            println!(
                "  {:<num_w$}  {:<name_w$}  {:<type_w$}  {cmd_display}",
                i + 1,
                criterion.name,
                type_label,
                num_w = num_width,
                name_w = name_width,
                type_w = type_width + ANSI_COLOR_OVERHEAD,
            );
        } else {
            println!(
                "  {:<num_w$}  {:<name_w$}  {:<type_w$}  {cmd_display}",
                i + 1,
                criterion.name,
                type_label,
                num_w = num_width,
                name_w = name_width,
                type_w = type_width,
            );
        }
    }
}

/// Print a full spec table (name + description + criteria).
fn print_spec_table(name: &str, description: &str, criteria: &[assay_types::Criterion]) {
    println!("Spec: {name}");
    if !description.is_empty() {
        println!("Description: {description}");
    }
    println!();
    print_criteria_table(criteria);
}

/// Handle `assay spec list`.
fn handle_spec_list() {
    let root = project_root();
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    let specs_dir = root.join(".assay").join(&config.specs_dir);

    let result = match assay_core::spec::scan(&specs_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.entries.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return;
    }

    let name_width = result
        .entries
        .iter()
        .map(|e| e.slug().len())
        .max()
        .unwrap_or(0);

    println!("Specs:");
    for entry in &result.entries {
        match entry {
            SpecEntry::Legacy { slug, spec } => {
                if spec.description.is_empty() {
                    println!("  {:<width$}", slug, width = name_width);
                } else {
                    println!(
                        "  {:<width$}  {}",
                        slug,
                        spec.description,
                        width = name_width
                    );
                }
            }
            SpecEntry::Directory { slug, gates, .. } => {
                let indicator = "[srs]";
                let criteria_count = gates.criteria.len();
                println!(
                    "  {:<width$}  {indicator} {criteria_count} criteria",
                    slug,
                    width = name_width
                );
            }
        }
    }
}

/// Handle `assay spec new <name>`.
///
/// Creates a directory-based spec with template `spec.toml` and `gates.toml`.
fn handle_spec_new(name: &str) {
    let root = project_root();
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    let specs_dir = root.join(".assay").join(&config.specs_dir);
    let spec_dir = specs_dir.join(name);

    if spec_dir.exists() {
        eprintln!(
            "Error: spec directory '{}' already exists",
            spec_dir.display()
        );
        std::process::exit(1);
    }

    // Also check for a flat file with the same name
    let flat_path = specs_dir.join(format!("{name}.toml"));
    if flat_path.exists() {
        eprintln!("Error: spec '{name}.toml' already exists as a flat file");
        std::process::exit(1);
    }

    if let Err(e) = std::fs::create_dir_all(&spec_dir) {
        eprintln!("Error: could not create directory: {e}");
        std::process::exit(1);
    }

    let spec_toml = format!(
        r#"name = "{name}"
status = "draft"
version = "0.1"

[overview]
description = ""
functions = []

[[requirements]]
id = "REQ-FUNC-001"
title = ""
statement = ""
obligation = "shall"
priority = "must"
verification = "test"
status = "draft"
"#
    );

    let gates_toml = format!(
        r#"name = "{name}"

[gate]
enforcement = "required"

[[criteria]]
name = "compiles"
description = "{name} compiles without errors"
cmd = "echo 'TODO: add compile check'"
"#
    );

    let spec_path = spec_dir.join("spec.toml");
    let gates_path = spec_dir.join("gates.toml");

    if let Err(e) = std::fs::write(&spec_path, &spec_toml) {
        eprintln!("Error: failed to write spec.toml: {e}");
        std::process::exit(1);
    }
    if let Err(e) = std::fs::write(&gates_path, &gates_toml) {
        eprintln!("Error: failed to write gates.toml: {e}");
        std::process::exit(1);
    }

    let rel_spec = spec_path.strip_prefix(&root).unwrap_or(&spec_path);
    let rel_gates = gates_path.strip_prefix(&root).unwrap_or(&gates_path);
    println!("  Created spec `{name}`");
    println!("    created {}", rel_spec.display());
    println!("    created {}", rel_gates.display());
}

/// Load project config and resolve the shared gate execution context.
/// Returns (root, config, working_dir, config_timeout). Prints errors and exits on failure.
fn load_gate_context() -> (
    std::path::PathBuf,
    assay_types::Config,
    std::path::PathBuf,
    Option<u64>,
) {
    let root = project_root();
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

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

    (root, config, working_dir, config_timeout)
}

/// Streaming display counters accumulated during criterion evaluation.
struct StreamCounters {
    passed: usize,
    failed: usize,
    skipped: usize,
}

/// Display configuration for streaming criterion evaluation.
struct StreamConfig {
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
    verbose: bool,
    color: bool,
}

/// Stream a single criterion's evaluation with live "running" -> "PASS/FAIL" display.
fn stream_criterion(
    criterion: &assay_types::Criterion,
    working_dir: &std::path::Path,
    cfg: &StreamConfig,
    counters: &mut StreamCounters,
) {
    if criterion.cmd.is_none() && criterion.path.is_none() {
        counters.skipped += 1;
        return;
    }

    let cr = if cfg.color { "\r\x1b[K" } else { "\r" };
    eprint!("{cr}  {} ... running", criterion.name);

    let timeout =
        assay_core::gate::resolve_timeout(cfg.cli_timeout, criterion.timeout, cfg.config_timeout);

    match assay_core::gate::evaluate(criterion, working_dir, timeout) {
        Ok(result) => {
            let status_label = if result.passed {
                counters.passed += 1;
                format_pass(cfg.color)
            } else {
                counters.failed += 1;
                format_fail(cfg.color)
            };

            eprintln!("{cr}  {} ... {}", criterion.name, status_label);

            if !result.passed || cfg.verbose {
                print_evidence(&result.stdout, &result.stderr, result.truncated, cfg.color);
            }
        }
        Err(err) => {
            counters.failed += 1;
            eprintln!("{cr}  {} ... {}", criterion.name, format_fail(cfg.color));
            eprintln!("    error: {err}");
        }
    }
}

/// Print a gate summary line (pass/fail/skip counts).
fn print_gate_summary(counters: &StreamCounters, color: bool, label: &str) {
    let total = counters.passed + counters.failed + counters.skipped;
    let passed_str = format_count(counters.passed, "\x1b[32m", color);
    let failed_str = format_count(counters.failed, "\x1b[31m", color);
    let skipped_str = format_count(counters.skipped, "\x1b[33m", color);

    println!();
    println!(
        "{label}: {passed_str} passed, {failed_str} failed, {skipped_str} skipped (of {total} total)"
    );
}

/// Handle `assay gate run --all [--timeout N] [--verbose] [--json]`.
///
/// Scans all specs and runs gates for each, printing results per-spec.
/// Exits 0 if all specs pass, exits 1 if any spec has required failures.
fn handle_gate_run_all(cli_timeout: Option<u64>, verbose: bool, json: bool) {
    let (root, config, working_dir, config_timeout) = load_gate_context();
    let specs_dir = root.join(".assay").join(&config.specs_dir);

    let result = match assay_core::spec::scan(&specs_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.entries.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No specs found in {}", config.specs_dir);
        }
        return;
    }

    let assay_dir = root.join(".assay");
    let max_history = config.gates.as_ref().and_then(|g| g.max_history);

    if json {
        let summaries: Vec<_> = result
            .entries
            .iter()
            .map(|entry| {
                let summary = match entry {
                    SpecEntry::Legacy { spec, .. } => {
                        assay_core::gate::evaluate_all(
                            spec,
                            &working_dir,
                            cli_timeout,
                            config_timeout,
                        )
                    }
                    SpecEntry::Directory { gates, .. } => {
                        assay_core::gate::evaluate_all_gates(
                            gates,
                            &working_dir,
                            cli_timeout,
                            config_timeout,
                        )
                    }
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

        let output = serde_json::to_string_pretty(&summaries).unwrap_or_else(|e| {
            eprintln!("Error: failed to serialize gate results: {e}");
            std::process::exit(1);
        });
        println!("{output}");

        if any_failed {
            std::process::exit(1);
        }
        return;
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
        skipped: 0,
    };
    let spec_count = result.entries.len();
    let mut has_required_failure = false;

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
            let pre_fail = counters.failed;
            stream_criterion(criterion, &working_dir, &cfg, &mut counters);
            if counters.failed > pre_fail {
                let enforcement =
                    assay_core::gate::resolve_enforcement(criterion.enforcement, gate_section);
                if enforcement == assay_types::Enforcement::Required {
                    has_required_failure = true;
                }
            }
        }

        // Save per-spec history (streaming mode)
        let spec_passed = counters.passed - before_passed;
        let spec_failed = counters.failed - before_failed;
        let spec_skipped = counters.skipped - before_skipped;
        save_run_record(
            &assay_dir,
            entry.slug(),
            &working_dir,
            assay_types::GateRunSummary {
                spec_name: entry.slug().to_string(),
                results: Vec::new(),
                passed: spec_passed,
                failed: spec_failed,
                skipped: spec_skipped,
                total_duration_ms: 0,
                enforcement: assay_types::EnforcementSummary::default(),
            },
            max_history,
            false,
        );
    }

    print_gate_summary(&counters, color, &format!("Results ({spec_count} specs)"));
    if has_required_failure {
        std::process::exit(1);
    }
}

/// Handle `assay gate run <name> [--timeout N] [--verbose] [--json]`.
fn handle_gate_run(name: &str, cli_timeout: Option<u64>, verbose: bool, json: bool) {
    let (root, config, working_dir, config_timeout) = load_gate_context();
    let specs_dir = root.join(".assay").join(&config.specs_dir);

    let entry = match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(e) => e,
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            eprintln!("Error: spec '{name}' not found in {}", config.specs_dir);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let assay_dir = root.join(".assay");
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

        let output = serde_json::to_string_pretty(&summary).unwrap_or_else(|e| {
            eprintln!("Error: failed to serialize gate results: {e}");
            std::process::exit(1);
        });
        println!("{output}");
        if summary.enforcement.required_failed > 0 {
            std::process::exit(1);
        }
        return;
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
        skipped: 0,
    };
    let executable_count = criteria
        .iter()
        .filter(|c| c.cmd.is_some() || c.path.is_some())
        .count();

    if executable_count == 0 {
        println!("No executable criteria found");
        return;
    }

    let mut has_required_failure = false;
    for criterion in &criteria {
        let before_failed = counters.failed;
        stream_criterion(criterion, &working_dir, &cfg, &mut counters);
        if counters.failed > before_failed {
            let enforcement =
                assay_core::gate::resolve_enforcement(criterion.enforcement, gate_section);
            if enforcement == assay_types::Enforcement::Required {
                has_required_failure = true;
            }
        }
    }

    // Save history (streaming mode has no per-criterion results)
    save_run_record(
        &assay_dir,
        name,
        &working_dir,
        assay_types::GateRunSummary {
            spec_name: name.to_string(),
            results: Vec::new(),
            passed: counters.passed,
            failed: counters.failed,
            skipped: counters.skipped,
            total_duration_ms: 0,
            enforcement: assay_types::EnforcementSummary::default(),
        },
        max_history,
        false,
    );

    print_gate_summary(&counters, color, "Results");
    if has_required_failure {
        std::process::exit(1);
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

/// Format a timestamp as a relative age string (e.g., "5m", "2h") or absolute when >24h.
fn format_relative_timestamp(ts: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let delta = now.signed_duration_since(*ts);
    let secs = delta.num_seconds();
    if secs < 0 {
        return ts.format("%Y-%m-%d %H:%M").to_string();
    }
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        ts.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// Format a duration in milliseconds as a human-readable string.
fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        let secs = ms as f64 / 1000.0;
        if ms % 1000 == 0 {
            format!("{secs:.0}s")
        } else {
            format!("{secs:.1}s")
        }
    } else {
        let total_secs = ms / 1000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if secs == 0 {
            format!("{mins}m")
        } else {
            format!("{mins}m {secs}s")
        }
    }
}

/// Handle `assay gate history <name> [--json] [--limit N]` — table view.
fn handle_gate_history(name: &str, json: bool, limit: usize) {
    let root = project_root();
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };
    let assay_dir = root.join(".assay");

    // Verify spec exists
    let specs_dir = assay_dir.join(&config.specs_dir);
    match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(_) => {}
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            eprintln!("Error: spec '{name}' not found in {}", config.specs_dir);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    let ids = match assay_core::history::list(&assay_dir, name) {
        Ok(ids) => ids,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if ids.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No history for {name}");
        }
        return;
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

    // Load all records for display
    let records: Vec<assay_types::GateRunRecord> = display_ids
        .iter()
        .filter_map(|id| assay_core::history::load(&assay_dir, name, id).ok())
        .collect();

    if json {
        let output = serde_json::to_string_pretty(&records).unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            std::process::exit(1);
        });
        println!("{output}");
        return;
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
}

/// Handle `assay gate history <name> <run-id> [--json]` — detail view.
fn handle_gate_history_detail(name: &str, run_id: &str, json: bool) {
    let root = project_root();
    let assay_dir = root.join(".assay");

    let record = match assay_core::history::load(&assay_dir, name, run_id) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if json {
        let output = serde_json::to_string_pretty(&record).unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            std::process::exit(1);
        });
        println!("{output}");
        return;
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
    println!(
        "Time: {}",
        record.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
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
                "  {} ... {} [{}] {}",
                cr.criterion_name, status_str, enforcement_label, duration_str
            );
        }
    }
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

/// Display project status for bare `assay` invocation inside an initialized project.
///
/// Shows the binary version, project name, and a spec inventory with criteria counts.
/// Returns `Err` on config load failure so the caller controls the exit.
///
/// Unlike `handle_spec_list`, scan errors are soft warnings here — bare invocation
/// should degrade gracefully since the user didn't explicitly ask for spec data.
fn show_status(root: &Path) -> Result<(), String> {
    let config = assay_core::config::load(root).map_err(|e| format!("{e}"))?;

    println!(
        "assay {} -- {}",
        env!("CARGO_PKG_VERSION"),
        config.project_name
    );
    println!();

    let specs_dir = root.join(".assay").join(&config.specs_dir);
    let result = match assay_core::spec::scan(&specs_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: could not scan specs: {e}");
            return Ok(());
        }
    };

    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.entries.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return Ok(());
    }

    let name_width = result
        .entries
        .iter()
        .map(|e| e.slug().len())
        .max()
        .unwrap_or(0);

    println!("Specs:");
    for entry in &result.entries {
        match entry {
            SpecEntry::Legacy { slug, spec } => {
                let total = spec.criteria.len();
                let executable = spec
                    .criteria
                    .iter()
                    .filter(|c| c.cmd.is_some() || c.path.is_some())
                    .count();
                println!(
                    "  {:<width$}  {total} criteria ({executable} executable)",
                    slug,
                    width = name_width,
                );
            }
            SpecEntry::Directory { slug, gates, .. } => {
                let total = gates.criteria.len();
                let executable = gates
                    .criteria
                    .iter()
                    .filter(|c| c.cmd.is_some() || c.path.is_some())
                    .count();
                println!(
                    "  {:<width$}  [srs] {total} criteria ({executable} executable)",
                    slug,
                    width = name_width,
                );
            }
        }
    }

    Ok(())
}

/// Initialize tracing to stderr for MCP server operation.
///
/// Default level is `warn`. Override via `RUST_LOG` environment variable.
/// Stdout is reserved for JSON-RPC — all diagnostics go to stderr.
fn init_mcp_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    if let Err(e) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init()
    {
        eprintln!("[assay] warning: failed to initialize tracing: {e}");
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Init { name }) => {
            let root = project_root();
            let options = assay_core::init::InitOptions { name };
            match assay_core::init::init(&root, &options) {
                Ok(result) => {
                    println!("  Created assay project `{}`", result.project_name);
                    for path in &result.created_files {
                        let display = path.strip_prefix(&root).unwrap_or(path);
                        println!("    created {}", display.display());
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Command::Mcp { command }) => match command {
            McpCommand::Serve => {
                init_mcp_tracing();
                if let Err(e) = assay_mcp::serve().await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        },
        Some(Command::Spec { command }) => match command {
            SpecCommand::Show { name, json } => handle_spec_show(&name, json),
            SpecCommand::List => handle_spec_list(),
            SpecCommand::New { name } => handle_spec_new(&name),
        },
        Some(Command::Gate { command }) => match command {
            GateCommand::Run {
                name: Some(name),
                timeout,
                verbose,
                json,
                ..
            } => {
                handle_gate_run(&name, timeout, verbose, json);
            }
            GateCommand::Run {
                all: true,
                timeout,
                verbose,
                json,
                ..
            } => {
                handle_gate_run_all(timeout, verbose, json);
            }
            GateCommand::Run { .. } => {
                eprintln!("Error: specify a spec name or use --all");
                std::process::exit(1);
            }
            GateCommand::History {
                name,
                run_id: Some(rid),
                json,
                ..
            } => {
                handle_gate_history_detail(&name, &rid, json);
            }
            GateCommand::History {
                name,
                last: true,
                json,
                ..
            } => {
                let root = project_root();
                let assay_dir = root.join(".assay");
                let ids = match assay_core::history::list(&assay_dir, &name) {
                    Ok(ids) => ids,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                };
                match ids.last() {
                    Some(last_id) => handle_gate_history_detail(&name, last_id, json),
                    None => {
                        if json {
                            println!("null");
                        } else {
                            println!("No history for {name}");
                        }
                    }
                }
            }
            GateCommand::History {
                name, json, limit, ..
            } => {
                handle_gate_history(&name, json, limit);
            }
        },
        None => {
            // Note: project detection checks cwd only — no upward traversal.
            // Running `assay` from a subdirectory of a project shows the hint.
            let root = project_root();
            if root.join(".assay").is_dir() {
                if let Err(e) = show_status(&root) {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            } else {
                eprintln!("Not an Assay project. Run `assay init` to get started.");
                if let Err(e) = Cli::command().print_help() {
                    eprintln!("Error: could not print help: {e}");
                }
                println!();
            }
        }
    }
}
