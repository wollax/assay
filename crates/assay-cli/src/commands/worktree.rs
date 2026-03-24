use anyhow::bail;
use clap::Subcommand;
use std::io::IsTerminal;

use super::{assay_dir, colorize, colors_enabled, project_root};

#[derive(Subcommand)]
pub(crate) enum WorktreeCommand {
    /// Create an isolated git worktree for a spec
    #[command(after_long_help = "\
Examples:
  Create a worktree for a spec:
    assay worktree create auth-flow

  Create from a specific base branch:
    assay worktree create auth-flow --base develop

  Override the worktree directory:
    assay worktree create auth-flow --worktree-dir /tmp/worktrees

  Output as JSON:
    assay worktree create auth-flow --json")]
    Create {
        /// Spec name (filename without .toml extension)
        name: String,
        /// Base branch to create worktree from (default: auto-detected)
        #[arg(long)]
        base: Option<String>,
        /// Override worktree base directory
        #[arg(long)]
        worktree_dir: Option<String>,
        /// Output as JSON instead of human-readable
        #[arg(long)]
        json: bool,
    },
    /// List all active assay worktrees
    #[command(after_long_help = "\
Examples:
  List all worktrees:
    assay worktree list

  List as JSON:
    assay worktree list --json")]
    List {
        /// Output as JSON instead of human-readable table
        #[arg(long)]
        json: bool,
    },
    /// Check worktree status (branch, dirty state, ahead/behind)
    #[command(after_long_help = "\
Examples:
  Check status of a worktree:
    assay worktree status auth-flow

  Output as JSON:
    assay worktree status auth-flow --json")]
    Status {
        /// Spec name (filename without .toml extension)
        name: String,
        /// Output as JSON instead of human-readable
        #[arg(long)]
        json: bool,
        /// Override worktree base directory
        #[arg(long)]
        worktree_dir: Option<String>,
    },
    /// Remove a worktree and its branch
    #[command(after_long_help = "\
Examples:
  Remove a specific worktree:
    assay worktree cleanup auth-flow

  Remove without confirmation:
    assay worktree cleanup auth-flow --force

  Remove all worktrees:
    assay worktree cleanup --all

  Remove all without confirmation:
    assay worktree cleanup --all --force")]
    Cleanup {
        /// Spec name to clean up (required unless --all)
        name: Option<String>,
        /// Remove all assay worktrees
        #[arg(long)]
        all: bool,
        /// Skip confirmation prompts
        #[arg(long)]
        force: bool,
        /// Output as JSON instead of human-readable
        #[arg(long)]
        json: bool,
        /// Override worktree base directory
        #[arg(long)]
        worktree_dir: Option<String>,
    },
}

/// Handle worktree subcommands.
pub(crate) fn handle(command: WorktreeCommand) -> anyhow::Result<i32> {
    match command {
        WorktreeCommand::Create {
            name,
            base,
            worktree_dir,
            json,
        } => handle_worktree_create(&name, base.as_deref(), worktree_dir.as_deref(), json),
        WorktreeCommand::List { json } => handle_worktree_list(json),
        WorktreeCommand::Status {
            name,
            json,
            worktree_dir,
        } => handle_worktree_status(&name, worktree_dir.as_deref(), json),
        WorktreeCommand::Cleanup {
            name,
            all,
            force,
            json,
            worktree_dir,
        } => handle_worktree_cleanup(name.as_deref(), all, force, worktree_dir.as_deref(), json),
    }
}

/// Resolve worktree dir and specs dir from config, returning (root, worktree_dir, specs_dir).
fn resolve_dirs(
    worktree_dir_override: Option<&str>,
) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let root = project_root()?;
    let ad = assay_dir(&root);
    if !ad.is_dir() {
        bail!("No Assay project found. Run `assay init` first.");
    }
    let config = assay_core::config::load(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let worktree_dir =
        assay_core::worktree::resolve_worktree_dir(worktree_dir_override, &config, &root);
    let specs_dir = root.join(".assay").join(&config.specs_dir);
    Ok((root, worktree_dir, specs_dir))
}

fn handle_worktree_create(
    name: &str,
    base: Option<&str>,
    worktree_dir_override: Option<&str>,
    json: bool,
) -> anyhow::Result<i32> {
    let (root, worktree_dir, specs_dir) = resolve_dirs(worktree_dir_override)?;

    let info = assay_core::worktree::create(&root, name, base, &worktree_dir, &specs_dir, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if json {
        let output = serde_json::to_string_pretty(&info)?;
        println!("{output}");
    } else {
        let color = colors_enabled();
        println!(
            "Created worktree for '{}' at {}",
            colorize(name, "\x1b[36m", color),
            info.path.display()
        );
        println!("  Branch: {}", info.branch);
        if let Some(ref base_branch) = info.base_branch {
            println!("  Base: {base_branch}");
        }
    }

    Ok(0)
}

fn handle_worktree_list(json: bool) -> anyhow::Result<i32> {
    let (root, _worktree_dir, _specs_dir) = resolve_dirs(None)?;

    let result = assay_core::worktree::list(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    for warning in &result.warnings {
        tracing::warn!("{warning}");
    }
    let entries = result.entries;

    if json {
        let output = serde_json::to_string_pretty(&entries)?;
        println!("{output}");
        return Ok(0);
    }

    if entries.is_empty() {
        println!("No active worktrees.");
        return Ok(0);
    }

    let color = colors_enabled();

    // Compute column widths
    let spec_width = entries
        .iter()
        .map(|e| e.spec_slug.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let branch_width = entries
        .iter()
        .map(|e| e.branch.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let path_width = entries
        .iter()
        .map(|e| e.path.display().to_string().len())
        .max()
        .unwrap_or(4)
        .max(4);

    // Header
    println!(
        "  {:<sw$}  {:<bw$}  {:<pw$}",
        "Spec",
        "Branch",
        "Path",
        sw = spec_width,
        bw = branch_width,
        pw = path_width,
    );
    println!(
        "  {:<sw$}  {:<bw$}  {:<pw$}",
        "\u{2500}".repeat(spec_width),
        "\u{2500}".repeat(branch_width),
        "\u{2500}".repeat(path_width),
        sw = spec_width,
        bw = branch_width,
        pw = path_width,
    );

    for entry in &entries {
        let spec_display = if color {
            colorize(&entry.spec_slug, "\x1b[36m", true)
        } else {
            entry.spec_slug.clone()
        };
        // ANSI overhead for colored spec column
        let extra = if color { super::ANSI_COLOR_OVERHEAD } else { 0 };

        println!(
            "  {:<sw$}  {:<bw$}  {}",
            spec_display,
            entry.branch,
            entry.path.display(),
            sw = spec_width + extra,
            bw = branch_width,
        );
    }

    Ok(0)
}

fn handle_worktree_status(
    name: &str,
    worktree_dir_override: Option<&str>,
    json: bool,
) -> anyhow::Result<i32> {
    let (_root, worktree_dir, _specs_dir) = resolve_dirs(worktree_dir_override)?;
    let worktree_path = worktree_dir.join(name);

    let st =
        assay_core::worktree::status(&worktree_path, name).map_err(|e| anyhow::anyhow!("{e}"))?;

    if json {
        let output = serde_json::to_string_pretty(&st)?;
        println!("{output}");
        return Ok(0);
    }

    let color = colors_enabled();
    let status_label = if st.dirty {
        colorize("dirty", "\x1b[33m", color)
    } else {
        colorize("clean", "\x1b[32m", color)
    };

    println!("Worktree: {}", colorize(&st.spec_slug, "\x1b[36m", color));
    println!("  Branch: {}", st.branch);
    println!("  HEAD:   {}", st.head);
    println!("  Status: {status_label}");
    let ahead_str = st.ahead.map_or("n/a".to_string(), |v| v.to_string());
    let behind_str = st.behind.map_or("n/a".to_string(), |v| v.to_string());
    println!("  Ahead:  {ahead_str}  Behind: {behind_str}");

    if let Some(ref base) = st.base_branch {
        println!("  Base:   {base}");
    }

    for warning in &st.warnings {
        println!(
            "  {}",
            colorize(&format!("Warning: {warning}"), "\x1b[33m", color)
        );
    }

    Ok(0)
}

fn handle_worktree_cleanup(
    name: Option<&str>,
    all: bool,
    force: bool,
    worktree_dir_override: Option<&str>,
    json: bool,
) -> anyhow::Result<i32> {
    if name.is_none() && !all {
        bail!("Specify a spec name or use --all to remove all worktrees.");
    }

    let (root, worktree_dir, _specs_dir) = resolve_dirs(worktree_dir_override)?;

    if all {
        return handle_worktree_cleanup_all(&root, force, json);
    }

    let spec_slug = name.unwrap();
    let worktree_path = worktree_dir.join(spec_slug);

    // Check dirty state for confirmation; track whether user confirmed.
    // WorktreeNotFound is treated as clean (proceed without prompt).
    // Other status errors are propagated so the user sees the real problem.
    let effective_force = if !force {
        let is_dirty = match assay_core::worktree::status(&worktree_path, spec_slug) {
            Ok(s) => s.dirty,
            Err(assay_core::error::AssayError::WorktreeNotFound { .. }) => false,
            Err(e) => return Err(anyhow::anyhow!("{e}")),
        };

        if is_dirty {
            if !std::io::stdin().is_terminal() {
                bail!(
                    "Worktree '{spec_slug}' has uncommitted changes. \
                     Use --force to override in non-interactive mode."
                );
            }
            eprint!("Worktree '{spec_slug}' has uncommitted changes. Remove anyway? [y/N] ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !matches!(input.trim(), "y" | "Y" | "yes" | "YES") {
                println!("Aborted.");
                return Ok(1);
            }
            true // user confirmed — pass force=true to core
        } else {
            false
        }
    } else {
        true
    };

    assay_core::worktree::cleanup(&root, &worktree_path, spec_slug, effective_force)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if json {
        let output = serde_json::json!({"removed": spec_slug});
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Removed worktree for '{spec_slug}'");
    }

    Ok(0)
}

fn handle_worktree_cleanup_all(
    root: &std::path::Path,
    force: bool,
    json: bool,
) -> anyhow::Result<i32> {
    let result = assay_core::worktree::list(root).map_err(|e| anyhow::anyhow!("{e}"))?;
    for warning in &result.warnings {
        tracing::warn!("{warning}");
    }
    let entries = result.entries;

    if entries.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No active worktrees to clean up.");
        }
        return Ok(0);
    }

    // Check dirty state for each worktree to inform the user.
    // WorktreeNotFound is treated as clean; other errors are warned and treated as clean.
    let dirty_slugs: Vec<&str> = if !force {
        entries
            .iter()
            .filter(
                |e| match assay_core::worktree::status(&e.path, &e.spec_slug) {
                    Ok(s) => s.dirty,
                    Err(assay_core::error::AssayError::WorktreeNotFound { .. }) => false,
                    Err(err) => {
                        tracing::warn!(
                            spec_slug = %e.spec_slug,
                            error = %err,
                            "Could not check worktree status"
                        );
                        false
                    }
                },
            )
            .map(|e| e.spec_slug.as_str())
            .collect()
    } else {
        vec![]
    };

    if !force {
        if !std::io::stdin().is_terminal() {
            bail!(
                "Refusing to remove {} worktrees in non-interactive mode. Use --force to override.",
                entries.len()
            );
        }
        tracing::info!("The following worktrees will be removed:");
        for entry in &entries {
            let dirty_marker = if dirty_slugs.contains(&entry.spec_slug.as_str()) {
                " (dirty!)"
            } else {
                ""
            };
            tracing::info!(
                spec_slug = %entry.spec_slug,
                path = %entry.path.display(),
                "  - {}{dirty_marker}",
                entry.spec_slug,
            );
        }
        if !dirty_slugs.is_empty() {
            tracing::warn!(
                count = dirty_slugs.len(),
                "Worktree(s) have uncommitted changes"
            );
        }
        eprint!("Remove all? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !matches!(input.trim(), "y" | "Y" | "yes" | "YES") {
            println!("Aborted.");
            return Ok(1);
        }
    }

    let mut removed = Vec::new();
    let mut failed = Vec::new();
    for entry in &entries {
        // Always use the canonical path reported by `git worktree list`.
        let path = entry.path.clone();
        // User confirmed removal (or --force was passed), so force-remove dirty worktrees
        let entry_force = force || dirty_slugs.contains(&entry.spec_slug.as_str());
        match assay_core::worktree::cleanup(root, &path, &entry.spec_slug, entry_force) {
            Ok(()) => removed.push(entry.spec_slug.clone()),
            Err(e) => {
                tracing::warn!(spec_slug = %entry.spec_slug, error = %e, "Failed to remove worktree");
                failed.push(entry.spec_slug.clone());
            }
        }
    }

    if json {
        let output = serde_json::json!(removed);
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for name in &removed {
            println!("Removed worktree for '{name}'");
        }
    }

    if !failed.is_empty() {
        tracing::error!(
            failed_count = failed.len(),
            "Some worktrees could not be removed"
        );
        return Ok(1);
    }

    Ok(0)
}
