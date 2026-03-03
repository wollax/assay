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
/// "executable" (has a `cmd`) renders green; "descriptive" renders yellow.
fn format_criteria_type(has_cmd: bool, color: bool) -> String {
    if has_cmd {
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
    let spec_path = specs_dir.join(format!("{name}.toml"));

    let spec = match assay_core::spec::load(&spec_path) {
        Ok(s) => s,
        Err(assay_core::AssayError::Io { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            eprintln!("Error: spec '{name}' not found in {}", config.specs_dir);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if json {
        let output = serde_json::to_string_pretty(&spec).unwrap_or_else(|e| {
            eprintln!("Error: failed to serialize spec: {e}");
            std::process::exit(1);
        });
        println!("{output}");
        return;
    }

    // Table output
    let color = colors_enabled();

    println!("Spec: {}", spec.name);
    if !spec.description.is_empty() {
        println!("Description: {}", spec.description);
    }
    println!();

    // Column widths: compute from data
    let num_width = spec.criteria.len().to_string().len().max(1);
    let name_width = spec
        .criteria
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(9)
        .max(9); // "Criterion" header
    let type_width = 11; // "descriptive" is longest at 11

    // Header
    println!(
        "  {:<num_w$}  {:<name_w$}  {:<type_w$}  Command",
        "#",
        "Criterion",
        "Type",
        num_w = num_width,
        name_w = name_width,
        type_w = type_width,
    );
    // Separator
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

    for (i, criterion) in spec.criteria.iter().enumerate() {
        let type_label = format_criteria_type(criterion.cmd.is_some(), color);
        let cmd_display = criterion.cmd.as_deref().unwrap_or("");

        // When color is enabled the ANSI codes add non-printing characters,
        // so we need to pad the plain text width manually.
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

    // Print warnings for scan errors
    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.specs.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return;
    }

    // Compute name column width for alignment
    let name_width = result
        .specs
        .iter()
        .map(|(slug, _)| slug.len())
        .max()
        .unwrap_or(0);

    println!("Specs:");
    for (slug, spec) in &result.specs {
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

/// Stream a single criterion's evaluation with live "running" -> "PASS/FAIL" display.
fn stream_criterion(
    criterion: &assay_types::Criterion,
    working_dir: &std::path::Path,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
    verbose: bool,
    color: bool,
    counters: &mut StreamCounters,
) {
    if criterion.cmd.is_none() {
        counters.skipped += 1;
        return;
    }

    let cr = if color { "\r\x1b[K" } else { "\r" };
    eprint!("{cr}  {} ... running", criterion.name);

    let timeout = assay_core::gate::resolve_timeout(cli_timeout, criterion.timeout, config_timeout);

    match assay_core::gate::evaluate(criterion, working_dir, timeout) {
        Ok(result) => {
            let status_label = if result.passed {
                counters.passed += 1;
                format_pass(color)
            } else {
                counters.failed += 1;
                format_fail(color)
            };

            eprintln!("{cr}  {} ... {}", criterion.name, status_label);

            if !result.passed || verbose {
                print_evidence(&result.stdout, &result.stderr, result.truncated, color);
            }
        }
        Err(err) => {
            counters.failed += 1;
            eprintln!("{cr}  {} ... {}", criterion.name, format_fail(color));
            eprintln!("    error: {err}");
        }
    }
}

/// Print a gate summary line and exit with code 1 if any criteria failed.
fn print_gate_summary(counters: &StreamCounters, color: bool, label: &str) {
    let total = counters.passed + counters.failed + counters.skipped;
    let passed_str = format_count(counters.passed, "\x1b[32m", color);
    let failed_str = format_count(counters.failed, "\x1b[31m", color);
    let skipped_str = format_count(counters.skipped, "\x1b[33m", color);

    println!();
    println!(
        "{label}: {passed_str} passed, {failed_str} failed, {skipped_str} skipped (of {total} total)"
    );

    if counters.failed > 0 {
        std::process::exit(1);
    }
}

/// Handle `assay gate run --all [--timeout N] [--verbose] [--json]`.
///
/// Scans all specs and runs gates for each, printing results per-spec.
/// Exits 0 if all specs pass, exits 1 if any spec has failures.
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

    if result.specs.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No specs found in {}", config.specs_dir);
        }
        return;
    }

    if json {
        let summaries: Vec<_> = result
            .specs
            .iter()
            .map(|(_slug, spec)| {
                assay_core::gate::evaluate_all(spec, &working_dir, cli_timeout, config_timeout)
            })
            .collect();

        let any_failed = summaries.iter().any(|s| s.failed > 0);

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
    let mut counters = StreamCounters {
        passed: 0,
        failed: 0,
        skipped: 0,
    };
    let spec_count = result.specs.len();

    for (i, (slug, spec)) in result.specs.iter().enumerate() {
        if i > 0 {
            eprintln!();
        }
        eprintln!("--- {} ---", slug);

        let executable_count = spec.criteria.iter().filter(|c| c.cmd.is_some()).count();
        if executable_count == 0 {
            eprintln!("  No executable criteria");
            counters.skipped += spec.criteria.len();
            continue;
        }

        for criterion in &spec.criteria {
            stream_criterion(
                criterion,
                &working_dir,
                cli_timeout,
                config_timeout,
                verbose,
                color,
                &mut counters,
            );
        }
    }

    print_gate_summary(&counters, color, &format!("Results ({spec_count} specs)"));
}

/// Handle `assay gate run <name> [--timeout N] [--verbose] [--json]`.
fn handle_gate_run(name: &str, cli_timeout: Option<u64>, verbose: bool, json: bool) {
    let (root, config, working_dir, config_timeout) = load_gate_context();
    let specs_dir = root.join(".assay").join(&config.specs_dir);
    let spec_path = specs_dir.join(format!("{name}.toml"));

    let spec = match assay_core::spec::load(&spec_path) {
        Ok(s) => s,
        Err(assay_core::AssayError::Io { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            eprintln!("Error: spec '{name}' not found in {}", config.specs_dir);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    if json {
        let summary =
            assay_core::gate::evaluate_all(&spec, &working_dir, cli_timeout, config_timeout);
        let output = serde_json::to_string_pretty(&summary).unwrap_or_else(|e| {
            eprintln!("Error: failed to serialize gate results: {e}");
            std::process::exit(1);
        });
        println!("{output}");
        if summary.failed > 0 {
            std::process::exit(1);
        }
        return;
    }

    let color = colors_enabled();
    let mut counters = StreamCounters {
        passed: 0,
        failed: 0,
        skipped: 0,
    };
    let executable_count = spec.criteria.iter().filter(|c| c.cmd.is_some()).count();

    if executable_count == 0 {
        println!("No executable criteria found");
        return;
    }

    for criterion in &spec.criteria {
        stream_criterion(
            criterion,
            &working_dir,
            cli_timeout,
            config_timeout,
            verbose,
            color,
            &mut counters,
        );
    }

    print_gate_summary(&counters, color, "Results");
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

    if result.specs.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return Ok(());
    }

    // Compute column width for alignment
    let name_width = result
        .specs
        .iter()
        .map(|(slug, _)| slug.len())
        .max()
        .unwrap_or(0);

    println!("Specs:");
    for (slug, spec) in &result.specs {
        let total = spec.criteria.len();
        let executable = spec.criteria.iter().filter(|c| c.cmd.is_some()).count();
        println!(
            "  {:<width$}  {total} criteria ({executable} executable)",
            slug,
            width = name_width,
        );
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
