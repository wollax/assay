use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new Assay project in the current directory
    Init {
        /// Override the inferred project name
        #[arg(long)]
        name: Option<String>,
    },
    /// MCP server operations
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Manage spec files
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
    /// Run quality gates for a spec
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
}

#[derive(Subcommand)]
enum McpCommand {
    /// Start the MCP server (stdio transport)
    Serve,
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Display a spec's criteria in detail
    Show {
        /// Spec name (filename without .toml extension)
        name: String,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },
    /// List all available specs
    List,
}

#[derive(Subcommand)]
enum GateCommand {
    /// Run all executable criteria for a spec
    Run {
        /// Spec name (filename without .toml extension)
        name: String,
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
    std::env::var("NO_COLOR").is_err()
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
                // ANSI codes add 9 chars (\x1b[32m = 5 + \x1b[0m = 4) that don't display
                type_w = type_width + 9,
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
            println!("  {slug}");
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

/// Handle `assay gate run <name> [--timeout N] [--verbose] [--json]`.
fn handle_gate_run(name: &str, cli_timeout: Option<u64>, verbose: bool, json: bool) {
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

    // Resolve working directory: config gates.working_dir > project root
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

    // Extract config timeout
    let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);

    // JSON output path: evaluate all at once, serialize, print
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

    // Streaming display path: iterate criteria manually for live progress
    let color = colors_enabled();

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let executable_count = spec.criteria.iter().filter(|c| c.cmd.is_some()).count();

    if executable_count == 0 {
        println!("No executable criteria found");
        return;
    }

    for criterion in &spec.criteria {
        // Descriptive-only criteria: skip silently during streaming
        if criterion.cmd.is_none() {
            skipped += 1;
            continue;
        }

        // Show "running" state (overwritable line on stderr)
        if color {
            eprint!("\r\x1b[K  {} ... running", criterion.name);
        } else {
            eprint!("\r  {} ... running", criterion.name);
        }

        let timeout =
            assay_core::gate::resolve_timeout(cli_timeout, criterion.timeout, config_timeout);

        match assay_core::gate::evaluate(criterion, &working_dir, timeout) {
            Ok(result) => {
                let status_label = if result.passed {
                    passed += 1;
                    format_pass(color)
                } else {
                    failed += 1;
                    format_fail(color)
                };

                // Replace the running line with final status
                if color {
                    eprintln!("\r\x1b[K  {} ... {}", criterion.name, status_label);
                } else {
                    eprintln!("\r  {} ... {}", criterion.name, status_label);
                }

                // Show evidence for failures (always) or all criteria (--verbose)
                if !result.passed || verbose {
                    print_evidence(&result.stdout, &result.stderr, result.truncated, color);
                }
            }
            Err(err) => {
                failed += 1;

                // Replace the running line with FAILED
                if color {
                    eprintln!("\r\x1b[K  {} ... {}", criterion.name, format_fail(color));
                } else {
                    eprintln!("\r  {} ... {}", criterion.name, format_fail(color));
                }
                eprintln!("    error: {err}");
            }
        }
    }

    // Summary line (stdout for capturability)
    let total = passed + failed + skipped;
    let passed_str = format_count(passed, "\x1b[32m", color);
    let failed_str = format_count(failed, "\x1b[31m", color);
    let skipped_str = format_count(skipped, "\x1b[33m", color);

    println!();
    println!(
        "Results: {passed_str} passed, {failed_str} failed, {skipped_str} skipped (of {total} total)"
    );

    if failed > 0 {
        std::process::exit(1);
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
            let root = std::env::current_dir().unwrap_or_else(|e| {
                eprintln!("Error: could not determine current directory: {e}");
                std::process::exit(1);
            });
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
                    eprintln!("Error: {e:?}");
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
                name,
                timeout,
                verbose,
                json,
            } => {
                handle_gate_run(&name, timeout, verbose, json);
            }
        },
        None => {
            println!("assay {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
