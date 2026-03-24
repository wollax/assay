use anyhow::{Context, bail};
use clap::Subcommand;

use super::{
    ANSI_COLOR_OVERHEAD, assay_dir, colorize, colors_enabled, format_number, format_relative_time,
    format_size, project_root,
};

#[derive(Subcommand)]
pub(crate) enum ContextCommand {
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
    /// Background context protection daemon
    #[command(after_long_help = "\
Examples:
  Start the guard daemon:
    assay context guard start

  Start with a specific session:
    assay context guard start --session /path/to/session.jsonl

  Check daemon status:
    assay context guard status

  Stop the daemon:
    assay context guard stop

  View daemon logs:
    assay context guard logs
    assay context guard logs --level warn")]
    Guard {
        #[command(subcommand)]
        command: GuardCommand,
    },
}

#[derive(Subcommand)]
pub(crate) enum GuardCommand {
    /// Start the guard daemon (runs in foreground)
    Start {
        /// Session file path (auto-discovers if omitted)
        #[arg(long)]
        session: Option<String>,
    },
    /// Stop the running guard daemon
    Stop,
    /// Show guard daemon status
    Status,
    /// View guard daemon logs
    Logs {
        /// Filter by log level (trace, debug, info, warn, error)
        #[arg(long, default_value = "info")]
        level: String,
    },
}

/// Handle context subcommands.
pub(crate) fn handle(command: ContextCommand) -> anyhow::Result<i32> {
    match command {
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
        ContextCommand::Guard { command } => match command {
            GuardCommand::Start { session } => handle_guard_start(session.as_deref()),
            GuardCommand::Stop => handle_guard_stop(),
            GuardCommand::Status => handle_guard_status(),
            GuardCommand::Logs { level } => handle_guard_logs(&level),
        },
    }
}

// ── Context handlers ──────────────────────────────────────────────

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

// ── Guard daemon handlers ─────────────────────────────────────────

/// Minimum severity rank for log level filtering.
fn log_level_rank(level: &str) -> u8 {
    match level.to_uppercase().as_str() {
        "TRACE" => 0,
        "DEBUG" => 1,
        "INFO" => 2,
        "WARN" => 3,
        "ERROR" => 4,
        _ => 2, // default to INFO
    }
}

/// All log level names at or above the given rank.
fn levels_at_or_above(min_rank: u8) -> Vec<&'static str> {
    ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]
        .into_iter()
        .filter(|l| log_level_rank(l) >= min_rank)
        .collect()
}

/// Handle `assay context guard start [--session <path>]`.
#[cfg(unix)]
fn handle_guard_start(session: Option<&str>) -> anyhow::Result<i32> {
    let root = project_root()?;
    let assay = assay_dir(&root);

    // Load config
    let config = assay_core::config::load(&root)?;
    let guard_config = config
        .guard
        .unwrap_or_else(|| serde_json::from_str("{}").expect("default GuardConfig should parse"));

    // Validate guard config
    let errors = assay_core::guard::config::validate(&guard_config);
    if !errors.is_empty() {
        tracing::error!(error_count = errors.len(), "Guard configuration is invalid");
        for e in &errors {
            tracing::error!(error = %e, "Guard configuration error");
        }
        return Ok(1);
    }

    // Resolve session path
    let session_path = match session {
        Some(s) => std::path::PathBuf::from(s),
        None => {
            let session_dir = assay_core::context::find_session_dir(&root)?;
            assay_core::context::resolve_session(&session_dir, None)?
        }
    };

    // Set up guard log directory and tracing
    let guard_dir = assay.join("guard");
    std::fs::create_dir_all(&guard_dir)
        .with_context(|| format!("creating guard log directory: {}", guard_dir.display()))?;

    // Note: the guard directory is created here for S04, which will add
    // file-based trace logging. Until then, the centralized stderr subscriber
    // initialized in main() is sufficient.

    tracing::info!(path = %session_path.display(), "[guard] Starting — watching");
    tracing::info!(
        soft_threshold = guard_config.soft_threshold * 100.0,
        hard_threshold = guard_config.hard_threshold * 100.0,
        poll_interval_secs = guard_config.poll_interval_secs,
        "[guard] Configuration"
    );

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(assay_core::guard::start_guard(
        &session_path,
        &assay,
        &root,
        guard_config,
    ));

    match result {
        Ok(()) => {
            tracing::info!("[guard] Stopped cleanly.");
            Ok(0)
        }
        Err(ref e) if matches!(e, assay_core::AssayError::GuardCircuitBreakerTripped { .. }) => {
            tracing::error!(error = %e, "Guard circuit breaker tripped");
            Ok(2)
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(not(unix))]
fn handle_guard_start(_session: Option<&str>) -> anyhow::Result<i32> {
    tracing::error!("Guard daemon is only supported on Unix platforms.");
    Ok(1)
}

/// Handle `assay context guard stop`.
#[cfg(unix)]
fn handle_guard_stop() -> anyhow::Result<i32> {
    let root = project_root()?;
    let assay = assay_dir(&root);

    match assay_core::guard::stop_guard(&assay) {
        Ok(()) => {
            println!("[guard] Daemon stopped.");
            Ok(0)
        }
        Err(assay_core::AssayError::GuardNotRunning) => {
            println!("Guard daemon is not running.");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(not(unix))]
fn handle_guard_stop() -> anyhow::Result<i32> {
    tracing::error!("Guard daemon is only supported on Unix platforms.");
    Ok(1)
}

/// Handle `assay context guard status`.
fn handle_guard_status() -> anyhow::Result<i32> {
    let root = project_root()?;
    let assay = assay_dir(&root);

    match assay_core::guard::guard_status(&assay) {
        assay_core::guard::GuardStatus::Running { pid } => {
            println!("Guard daemon running (PID {pid})");
            Ok(0)
        }
        assay_core::guard::GuardStatus::Stopped => {
            println!("Guard daemon is not running");
            Ok(1)
        }
    }
}

/// Handle `assay context guard logs [--level <level>]`.
fn handle_guard_logs(level: &str) -> anyhow::Result<i32> {
    let root = project_root()?;
    let assay = assay_dir(&root);
    let log_path = assay.join("guard").join("guard.log");

    if !log_path.is_file() {
        println!("No guard logs found.");
        return Ok(1);
    }

    let contents = std::fs::read_to_string(&log_path)
        .with_context(|| format!("reading guard log: {}", log_path.display()))?;

    let min_rank = log_level_rank(level);
    let visible = levels_at_or_above(min_rank);

    for line in contents.lines() {
        if visible.iter().any(|l| line.contains(l)) {
            println!("{line}");
        }
    }

    Ok(0)
}
