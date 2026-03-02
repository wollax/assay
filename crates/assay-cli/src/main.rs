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
        None => {
            println!("assay {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
