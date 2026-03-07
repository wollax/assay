use anyhow::{Context, bail};
use assay_core::spec::SpecEntry;
use assay_types::GateKind;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::Path;

/// Extra bytes added by a single ANSI color sequence pair (`\x1b[XXm` ... `\x1b[0m`).
/// `\x1b[32m` = 5 bytes, `\x1b[0m` = 4 bytes, total = 9.
const ANSI_COLOR_OVERHEAD: usize = 9;

/// Name of the Assay project directory relative to project root.
const ASSAY_DIR_NAME: &str = ".assay";

/// Build an absolute path to the Assay project directory under `root`.
fn assay_dir(root: &std::path::Path) -> std::path::PathBuf {
    root.join(ASSAY_DIR_NAME)
}

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
    /// Manage quality gates
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
    /// Context window diagnostics for Claude Code sessions
    #[command(after_long_help = "\
Examples:
  Analyze the most recent session:
    assay context diagnose

  Analyze a specific session:
    assay context diagnose 3201041c-df85-4c91-a485-7b8c189f7636

  List sessions for the current project:
    assay context list

  List all sessions with token counts:
    assay context list --all --tokens

  Output diagnostics as JSON:
    assay context diagnose --json")]
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    /// Team state checkpointing
    #[command(after_long_help = "\
Examples:
  Take a snapshot of current team state:
    assay checkpoint save

  Take a snapshot with a custom trigger label:
    assay checkpoint save --trigger \"pre-deploy\"

  Show the latest checkpoint:
    assay checkpoint show

  Show latest as JSON:
    assay checkpoint show --json

  List recent checkpoints:
    assay checkpoint list

  List more checkpoints:
    assay checkpoint list --limit 25")]
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommand,
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

#[derive(Subcommand)]
enum ContextCommand {
    /// Analyze token usage and bloat in a Claude Code session
    #[command(after_long_help = "\
Examples:
  Analyze the most recent session for this project:
    assay context diagnose

  Analyze a specific session by ID:
    assay context diagnose 3201041c-df85-4c91-a485-7b8c189f7636

  Output as JSON:
    assay context diagnose --json

  Plain output (no color, no Unicode):
    assay context diagnose --plain")]
    Diagnose {
        /// Session ID (defaults to most recent session)
        session_id: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Plain output (no color, no Unicode symbols)
        #[arg(long)]
        plain: bool,
    },
    /// List Claude Code sessions with metadata
    #[command(after_long_help = "\
Examples:
  List recent sessions:
    assay context list

  List all sessions:
    assay context list --all

  Include token counts (slower):
    assay context list --tokens

  Output as JSON:
    assay context list --json")]
    List {
        /// Maximum number of sessions to show
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Show all sessions (overrides --limit)
        #[arg(long)]
        all: bool,
        /// Include token counts (slower, reads session tails)
        #[arg(long)]
        tokens: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Plain output (no color, no Unicode symbols)
        #[arg(long)]
        plain: bool,
    },
    /// Prune session bloat using composable strategies
    #[command(after_long_help = "\
Examples:
  Dry-run with standard tier (default):
    assay context prune 3201041c-df85-4c91-a485-7b8c189f7636

  Dry-run with aggressive tier:
    assay context prune 3201041c --tier aggressive

  Run a single strategy:
    assay context prune 3201041c --strategy thinking-blocks

  Actually modify the session file:
    assay context prune 3201041c --execute

  List available backups for restore:
    assay context prune 3201041c --restore

  Output as JSON:
    assay context prune 3201041c --json")]
    Prune {
        /// Session ID (required)
        session_id: String,

        /// Prescription tier: gentle, standard, aggressive
        #[arg(long, default_value = "standard")]
        tier: String,

        /// Run a single strategy instead of a prescription
        #[arg(long, conflicts_with = "tier")]
        strategy: Option<String>,

        /// Actually modify the session file (default is dry-run)
        #[arg(long)]
        execute: bool,

        /// List and restore from a previous backup
        #[arg(long, conflicts_with_all = ["tier", "strategy", "execute"])]
        restore: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Plain output (no color, no Unicode)
        #[arg(long)]
        plain: bool,
    },
}

#[derive(Subcommand)]
enum CheckpointCommand {
    /// Take a team state snapshot
    Save {
        /// Trigger label (e.g., "manual", "pre-deploy")
        #[arg(long, default_value = "manual")]
        trigger: String,
        /// Session ID to checkpoint (default: most recent)
        #[arg(long)]
        session: Option<String>,
        /// Output as JSON instead of summary
        #[arg(long)]
        json: bool,
    },
    /// Show the latest checkpoint
    Show {
        /// Output as JSON (frontmatter data only)
        #[arg(long)]
        json: bool,
    },
    /// List archived checkpoints
    List {
        /// Maximum entries to show
        #[arg(long, default_value = "10")]
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

/// Map a [`GateKind`] to a short display label for CLI output.
fn gate_kind_label(kind: &GateKind) -> &'static str {
    match kind {
        GateKind::Command { .. } => "[cmd]",
        GateKind::FileExists { .. } => "[file]",
        GateKind::AlwaysPass => "[auto]",
        GateKind::AgentReport => "[agent]",
    }
}

/// Derive a display label from a [`Criterion`](assay_types::Criterion) struct.
///
/// Uses the same labels as [`gate_kind_label`] but infers kind from criterion fields.
fn criterion_label(criterion: &assay_types::Criterion) -> &'static str {
    if criterion.kind == Some(assay_types::CriterionKind::AgentReport) {
        "[agent]"
    } else if criterion.cmd.is_some() {
        "[cmd]"
    } else if criterion.path.is_some() {
        "[file]"
    } else {
        ""
    }
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

/// Format "WARN" with optional yellow color.
fn format_warn(color: bool) -> &'static str {
    if color { "\x1b[33mWARN\x1b[0m" } else { "WARN" }
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

/// Resolve the project root directory.
fn project_root() -> anyhow::Result<std::path::PathBuf> {
    std::env::current_dir().context("could not determine current directory")
}

/// Handle `assay spec show <name> [--json]`.
fn handle_spec_show(name: &str, json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let entry = match assay_core::spec::load_spec_entry(name, &specs_dir) {
        Ok(e) => e,
        Err(assay_core::AssayError::SpecNotFound { .. }) => {
            bail!("spec '{name}' not found in {}", config.specs_dir);
        }
        Err(e) => return Err(e.into()),
    };

    match entry {
        SpecEntry::Legacy { spec, .. } => {
            if json {
                let output =
                    serde_json::to_string_pretty(&spec).context("failed to serialize spec")?;
                println!("{output}");
                return Ok(0);
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
                let output = serde_json::to_string_pretty(&serde_json::Value::Object(map))
                    .context("failed to serialize spec")?;
                println!("{output}");
                return Ok(0);
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
    Ok(0)
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
fn handle_spec_list() -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let result = assay_core::spec::scan(&specs_dir)?;

    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.entries.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return Ok(0);
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
    Ok(0)
}

/// Handle `assay spec new <name>`.
///
/// Creates a directory-based spec with template `spec.toml` and `gates.toml`.
fn handle_spec_new(name: &str) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);
    let spec_dir = specs_dir.join(name);

    if spec_dir.exists() {
        bail!("spec directory '{}' already exists", spec_dir.display());
    }

    // Also check for a flat file with the same name
    let flat_path = specs_dir.join(format!("{name}.toml"));
    if flat_path.exists() {
        bail!("spec '{name}.toml' already exists as a flat file");
    }

    std::fs::create_dir_all(&spec_dir).context("could not create spec directory")?;

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

    std::fs::write(&spec_path, &spec_toml).context("failed to write spec.toml")?;
    std::fs::write(&gates_path, &gates_toml).context("failed to write gates.toml")?;

    let rel_spec = spec_path.strip_prefix(&root).unwrap_or(&spec_path);
    let rel_gates = gates_path.strip_prefix(&root).unwrap_or(&gates_path);
    println!("  Created spec `{name}`");
    println!("    created {}", rel_spec.display());
    println!("    created {}", rel_gates.display());
    Ok(0)
}

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
        if ms.is_multiple_of(1000) {
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

/// Format a byte count as a human-readable size string (e.g., "2.4 MB").
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a number with thousands separators (e.g., 156234 -> "156,234").
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// Format a relative time string from an ISO 8601 timestamp (e.g., "2h ago").
fn format_relative_time(iso: &str) -> String {
    match iso.parse::<chrono::DateTime<chrono::Utc>>() {
        Ok(dt) => {
            let now = chrono::Utc::now();
            let delta = now.signed_duration_since(dt);
            let secs = delta.num_seconds();
            if secs < 0 {
                return dt.format("%Y-%m-%d %H:%M").to_string();
            }
            if secs < 60 {
                format!("{secs}s ago")
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else if secs < 86400 {
                format!("{}h ago", secs / 3600)
            } else if secs < 604800 {
                format!("{}d ago", secs / 86400)
            } else {
                dt.format("%Y-%m-%d %H:%M").to_string()
            }
        }
        Err(_) => iso.to_string(),
    }
}

/// Color a string with an ANSI code, respecting the `color` flag.
fn colorize(text: &str, ansi_code: &str, color: bool) -> String {
    if color {
        format!("{ansi_code}{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Handle `assay context diagnose [session_id] [--json] [--plain]`.
fn handle_context_diagnose(
    session_id: Option<&str>,
    json: bool,
    plain: bool,
) -> anyhow::Result<i32> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    let session_dir = assay_core::context::find_session_dir(&cwd)?;
    let session_path = assay_core::context::resolve_session(&session_dir, session_id)?;

    let resolved_id = session_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let report = assay_core::context::diagnose(&session_path, &resolved_id)?;

    if json {
        let output = serde_json::to_string_pretty(&report)
            .context("failed to serialize diagnostics report")?;
        println!("{output}");
        return Ok(0);
    }

    let color = !plain && colors_enabled();
    let divider = if plain { "-" } else { "\u{2500}" };
    let section = |title: &str| {
        let pad = 60usize.saturating_sub(title.len() + 4);
        format!(
            "{d}{d} {title} {rest}",
            d = divider,
            rest = divider.repeat(pad)
        )
    };

    // Header
    println!("{}", section("Context Diagnostics"));
    println!("Session:  {}", report.session_id);
    if let Some(ref model) = report.model {
        println!("Model:    {model}");
    }
    println!(
        "File:     {} ({})",
        report.file_path,
        format_size(report.file_size_bytes)
    );

    // Overview
    println!();
    println!("{}", section("Overview"));

    if let Some(ref usage) = report.usage {
        let context_tokens = usage.context_tokens();
        let pct = report.context_utilization_pct.unwrap_or(0.0);
        let health_label = if pct >= 85.0 {
            colorize("Critical", "\x1b[31m", color)
        } else if pct >= 60.0 {
            colorize("Warning", "\x1b[33m", color)
        } else {
            colorize("Healthy", "\x1b[32m", color)
        };

        let pct_display = format!("{pct:.1}%");
        let pct_colored = if pct >= 85.0 {
            colorize(&pct_display, "\x1b[31m", color)
        } else if pct >= 60.0 {
            colorize(&pct_display, "\x1b[33m", color)
        } else {
            colorize(&pct_display, "\x1b[32m", color)
        };

        println!(
            "Context tokens:    {} / {} ({})   {}",
            format_number(context_tokens),
            format_number(report.context_window),
            pct_colored,
            health_label
        );
        println!("Output tokens:     {}", format_number(usage.output_tokens));
    } else {
        println!("Context tokens:    (no usage data)");
    }

    println!(
        "Total entries:     {} ({} messages)",
        format_number(report.total_entries),
        format_number(report.message_count)
    );

    // Bloat Breakdown
    println!();
    println!("{}", section("Bloat Breakdown"));

    // Table header
    let cat_width = 20;
    let bytes_width = 12;
    let count_width = 8;
    let pct_width = 10;
    println!(
        "{:<cw$}{:>bw$}{:>cow$}{:>pw$}",
        "Category",
        "Bytes",
        "Count",
        "% of File",
        cw = cat_width,
        bw = bytes_width,
        cow = count_width,
        pw = pct_width,
    );

    // Only show categories with non-zero counts, sorted by bytes descending
    let mut entries: Vec<_> = report
        .bloat
        .entries
        .iter()
        .filter(|e| e.count > 0)
        .collect();
    entries.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    if entries.is_empty() {
        println!("  (no bloat detected)");
    } else {
        for entry in &entries {
            let pct_str = format!("{:.1}%", entry.percentage);
            println!(
                "{:<cw$}{:>bw$}{:>cow$}{:>pw$}",
                entry.category.label(),
                format_size(entry.bytes),
                entry.count,
                pct_str,
                cw = cat_width,
                bw = bytes_width,
                cow = count_width,
                pw = pct_width,
            );
        }
    }

    Ok(0)
}

/// Handle `assay context prune <session_id> [--tier T] [--strategy S] [--execute] [--restore] [--json] [--plain]`.
fn handle_context_prune(
    session_id: &str,
    tier_str: &str,
    strategy_str: Option<&str>,
    execute: bool,
    restore: bool,
    json: bool,
    plain: bool,
) -> anyhow::Result<i32> {
    use assay_types::context::{PrescriptionTier, PruneStrategy};

    let cwd = std::env::current_dir().context("could not determine current directory")?;
    let session_dir = assay_core::context::find_session_dir(&cwd)?;
    let session_path = assay_core::context::resolve_session(&session_dir, Some(session_id))?;

    let root = project_root()?;
    let backup_dir = assay_dir(&root).join("backups");

    // Restore mode: list available backups
    if restore {
        let resolved_id = session_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let backups =
            assay_core::context::pruning::backup::list_backups(&backup_dir, &resolved_id)?;

        if backups.is_empty() {
            if json {
                println!("[]");
            } else {
                println!("No backups found for session {session_id}");
            }
            return Ok(0);
        }

        if json {
            let paths: Vec<String> = backups
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            let output =
                serde_json::to_string_pretty(&paths).context("failed to serialize backups")?;
            println!("{output}");
        } else {
            println!("Available backups for session {session_id}:");
            for (i, path) in backups.iter().enumerate() {
                println!("  {}. {}", i + 1, path.display());
            }
        }

        return Ok(0);
    }

    // Parse tier
    let tier = match tier_str {
        "gentle" => PrescriptionTier::Gentle,
        "standard" => PrescriptionTier::Standard,
        "aggressive" => PrescriptionTier::Aggressive,
        _ => bail!("unknown prescription tier: '{tier_str}'. Valid: gentle, standard, aggressive"),
    };

    // Determine strategies
    let strategies: Vec<PruneStrategy> = if let Some(s) = strategy_str {
        let single = match s {
            "progress-collapse" => PruneStrategy::ProgressCollapse,
            "stale-reads" => PruneStrategy::StaleReads,
            "thinking-blocks" => PruneStrategy::ThinkingBlocks,
            "tool-output-trim" => PruneStrategy::ToolOutputTrim,
            "metadata-strip" => PruneStrategy::MetadataStrip,
            "system-reminder-dedup" => PruneStrategy::SystemReminderDedup,
            _ => bail!(
                "unknown strategy: '{s}'. Valid: progress-collapse, stale-reads, thinking-blocks, tool-output-trim, metadata-strip, system-reminder-dedup"
            ),
        };
        vec![single]
    } else {
        tier.strategies().to_vec()
    };

    // Run pruning
    let report = assay_core::context::pruning::prune_session(
        &session_path,
        &strategies,
        tier,
        execute,
        Some(&backup_dir),
    )?;

    if json {
        let output =
            serde_json::to_string_pretty(&report).context("failed to serialize prune report")?;
        println!("{output}");
    } else {
        let color = !plain && colors_enabled();
        let output = assay_core::context::pruning::report::format_dry_run_report(&report, color);
        print!("{output}");
    }

    Ok(0)
}

/// Handle `assay context list [--limit N] [--all] [--tokens] [--json] [--plain]`.
fn handle_context_list(
    limit: usize,
    all: bool,
    tokens: bool,
    json: bool,
    plain: bool,
) -> anyhow::Result<i32> {
    let cwd = std::env::current_dir().context("could not determine current directory")?;
    let effective_limit = if all { usize::MAX } else { limit };

    let sessions = assay_core::context::list_sessions(Some(&cwd), effective_limit, tokens)?;

    if json {
        let output =
            serde_json::to_string_pretty(&sessions).context("failed to serialize session list")?;
        println!("{output}");
        return Ok(0);
    }

    if sessions.is_empty() {
        println!("No sessions found for this project.");
        return Ok(0);
    }

    let color = !plain && colors_enabled();

    // Column widths
    let id_width = 36; // UUID length
    let size_width = 10;
    let entries_width = 8;
    let modified_width = 20;

    // Header
    if tokens {
        println!(
            "{:<iw$}  {:>sw$}  {:>ew$}  {:>tw$}  {:<mw$}",
            "Session ID",
            "Size",
            "Entries",
            "Tokens",
            "Modified",
            iw = id_width,
            sw = size_width,
            ew = entries_width,
            tw = 10,
            mw = modified_width,
        );
    } else {
        println!(
            "{:<iw$}  {:>sw$}  {:>ew$}  {:<mw$}",
            "Session ID",
            "Size",
            "Entries",
            "Modified",
            iw = id_width,
            sw = size_width,
            ew = entries_width,
            mw = modified_width,
        );
    }

    for session in &sessions {
        let size = format_size(session.file_size_bytes);
        let modified = session
            .last_modified
            .as_deref()
            .map(format_relative_time)
            .unwrap_or_else(|| "unknown".to_string());

        let id_display = if color {
            colorize(&session.session_id, "\x1b[36m", true)
        } else {
            session.session_id.clone()
        };
        let id_pad = if color {
            id_width + ANSI_COLOR_OVERHEAD
        } else {
            id_width
        };

        if tokens {
            let token_str = session
                .token_count
                .map(format_number)
                .unwrap_or_else(|| "-".to_string());
            println!(
                "{:<iw$}  {:>sw$}  {:>ew$}  {:>tw$}  {:<mw$}",
                id_display,
                size,
                session.entry_count,
                token_str,
                modified,
                iw = id_pad,
                sw = size_width,
                ew = entries_width,
                tw = 10,
                mw = modified_width,
            );
        } else {
            println!(
                "{:<iw$}  {:>sw$}  {:>ew$}  {:<mw$}",
                id_display,
                size,
                session.entry_count,
                modified,
                iw = id_pad,
                sw = size_width,
                ew = entries_width,
                mw = modified_width,
            );
        }
    }

    let shown = sessions.len();
    if !all && shown == limit {
        println!();
        println!("Showing {shown} sessions. Use --all to see all sessions.");
    }

    Ok(0)
}

/// Handle `assay checkpoint save [--trigger T] [--session S] [--json]`.
fn handle_checkpoint_save(trigger: &str, session: Option<&str>, json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let ad = assay_dir(&root);
    if !ad.is_dir() {
        bail!("No Assay project found. Run `assay init` first.");
    }

    let checkpoint = assay_core::checkpoint::extract_team_state(&root, session, trigger)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let archive_path = assay_core::checkpoint::save_checkpoint(&ad, &checkpoint)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if json {
        let output =
            serde_json::to_string_pretty(&checkpoint).context("failed to serialize checkpoint")?;
        println!("{output}");
    } else {
        let rel = archive_path.strip_prefix(&root).unwrap_or(&archive_path);
        println!("Checkpoint saved: {}", rel.display());
        println!(
            "  Agents: {}  Tasks: {}  Trigger: {}",
            checkpoint.agents.len(),
            checkpoint.tasks.len(),
            checkpoint.trigger,
        );
    }

    Ok(0)
}

/// Handle `assay checkpoint show [--json]`.
fn handle_checkpoint_show(json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let ad = assay_dir(&root);
    if !ad.is_dir() {
        bail!("No Assay project found. Run `assay init` first.");
    }

    let latest_path = ad.join("checkpoints").join("latest.md");
    if !latest_path.exists() {
        bail!("No checkpoints found. Run `assay checkpoint save` to create one.");
    }

    if json {
        let checkpoint = assay_core::checkpoint::load_latest_checkpoint(&ad)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let output =
            serde_json::to_string_pretty(&checkpoint).context("failed to serialize checkpoint")?;
        println!("{output}");
    } else {
        let content =
            std::fs::read_to_string(&latest_path).context("failed to read latest checkpoint")?;
        print!("{content}");
    }

    Ok(0)
}

/// Handle `assay checkpoint list [--limit N]`.
fn handle_checkpoint_list(limit: usize) -> anyhow::Result<i32> {
    let root = project_root()?;
    let ad = assay_dir(&root);
    if !ad.is_dir() {
        bail!("No Assay project found. Run `assay init` first.");
    }

    let entries =
        assay_core::checkpoint::list_checkpoints(&ad, limit).map_err(|e| anyhow::anyhow!("{e}"))?;

    if entries.is_empty() {
        println!("No checkpoints found.");
        return Ok(0);
    }

    // Table header
    let ts_width = entries
        .iter()
        .map(|e| e.timestamp.len())
        .max()
        .unwrap_or(9)
        .max(9);
    let trigger_width = entries
        .iter()
        .map(|e| e.trigger.len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "  {:<ts_w$}  {:<trig_w$}  {:>6}  {:>5}",
        "Timestamp",
        "Trigger",
        "Agents",
        "Tasks",
        ts_w = ts_width,
        trig_w = trigger_width,
    );
    println!(
        "  {:<ts_w$}  {:<trig_w$}  {:>6}  {:>5}",
        "\u{2500}".repeat(ts_width),
        "\u{2500}".repeat(trigger_width),
        "\u{2500}".repeat(6),
        "\u{2500}".repeat(5),
        ts_w = ts_width,
        trig_w = trigger_width,
    );

    for entry in &entries {
        println!(
            "  {:<ts_w$}  {:<trig_w$}  {:>6}  {:>5}",
            entry.timestamp,
            entry.trigger,
            entry.agent_count,
            entry.task_count,
            ts_w = ts_width,
            trig_w = trigger_width,
        );
    }

    Ok(0)
}

/// Display project status for bare `assay` invocation inside an initialized project.
///
/// Shows the binary version, project name, and a spec inventory with criteria counts.
/// Returns `Err` on config load failure so the caller controls the exit.
///
/// Unlike `handle_spec_list`, scan errors are soft warnings here — bare invocation
/// should degrade gracefully since the user didn't explicitly ask for spec data.
fn show_status(root: &Path) -> anyhow::Result<()> {
    let config = assay_core::config::load(root)?;

    println!(
        "assay {} -- {}",
        env!("CARGO_PKG_VERSION"),
        config.project_name
    );
    println!();

    let specs_dir = assay_dir(root).join(&config.specs_dir);
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

/// Core CLI logic. Returns an exit code on success.
async fn run() -> anyhow::Result<i32> {
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());

    match cli.command {
        Some(Command::Init { name }) => {
            let root = project_root()?;
            let options = assay_core::init::InitOptions { name };
            let result = assay_core::init::init(&root, &options)?;
            println!("  Created assay project `{}`", result.project_name);
            for path in &result.created_files {
                let display = path.strip_prefix(&root).unwrap_or(path);
                println!("    created {}", display.display());
            }
            Ok(0)
        }
        Some(Command::Mcp { command }) => match command {
            McpCommand::Serve => {
                init_mcp_tracing();
                assay_mcp::serve()
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                Ok(0)
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
        },
        Some(Command::Context { command }) => match command {
            ContextCommand::Diagnose {
                session_id,
                json,
                plain,
            } => handle_context_diagnose(session_id.as_deref(), json, plain),
            ContextCommand::List {
                limit,
                all,
                tokens,
                json,
                plain,
            } => handle_context_list(limit, all, tokens, json, plain),
            ContextCommand::Prune {
                session_id,
                tier,
                strategy,
                execute,
                restore,
                json,
                plain,
            } => handle_context_prune(
                &session_id,
                &tier,
                strategy.as_deref(),
                execute,
                restore,
                json,
                plain,
            ),
        },
        Some(Command::Checkpoint { command }) => match command {
            CheckpointCommand::Save {
                trigger,
                session,
                json,
            } => handle_checkpoint_save(&trigger, session.as_deref(), json),
            CheckpointCommand::Show { json } => handle_checkpoint_show(json),
            CheckpointCommand::List { limit } => handle_checkpoint_list(limit),
        },
        None => {
            // Note: project detection checks cwd only — no upward traversal.
            // Running `assay` from a subdirectory of a project shows the hint.
            let root = project_root()?;
            if assay_dir(&root).is_dir() {
                show_status(&root)?;
                Ok(0)
            } else {
                eprintln!("Not an Assay project. Run `assay init` to get started.");
                if let Err(e) = Cli::command().print_help() {
                    eprintln!("Error: could not print help: {e}");
                }
                println!();
                Ok(1)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}
