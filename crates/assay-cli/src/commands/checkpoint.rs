use anyhow::{Context, bail};
use clap::Subcommand;

use super::{assay_dir, project_root};

#[derive(Subcommand)]
pub(crate) enum CheckpointCommand {
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

/// Handle checkpoint subcommands.
pub(crate) fn handle(command: CheckpointCommand) -> anyhow::Result<i32> {
    match command {
        CheckpointCommand::Save {
            trigger,
            session,
            json,
        } => handle_checkpoint_save(&trigger, session.as_deref(), json),
        CheckpointCommand::Show { json } => handle_checkpoint_show(json),
        CheckpointCommand::List { limit } => handle_checkpoint_list(limit),
    }
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
