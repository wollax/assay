//! Milestone subcommands for the `assay milestone` CLI group.

use clap::Subcommand;

use super::{assay_dir, project_root};

#[derive(Subcommand)]
pub(crate) enum MilestoneCommand {
    /// List all milestones in the current project
    List,
    /// Show progress for active (in_progress) milestones
    Status {
        /// Output current cycle state as JSON to stdout
        #[arg(long)]
        json: bool,
    },
    /// Advance the development cycle: evaluate gates for the active chunk and mark it complete
    Advance {
        /// Slug of the milestone to advance. Defaults to the first in_progress milestone.
        #[arg(long)]
        milestone: Option<String>,
    },
}

/// Handle milestone subcommands.
pub(crate) fn handle(command: MilestoneCommand) -> anyhow::Result<i32> {
    match command {
        MilestoneCommand::List => milestone_list_cmd(),
        MilestoneCommand::Status { json } => milestone_status_cmd(json),
        MilestoneCommand::Advance { milestone } => milestone_advance_cmd(milestone),
    }
}

/// Handle `assay milestone list`.
fn milestone_list_cmd() -> anyhow::Result<i32> {
    let root = project_root()?;
    let dir = assay_dir(&root);

    let milestones = assay_core::milestone::milestone_scan(&dir)?;

    if milestones.is_empty() {
        println!("No milestones found.");
        return Ok(0);
    }

    // Print table header
    println!("{:<24}  {:<32}  STATUS", "SLUG", "NAME");
    println!("{}", "-".repeat(68));

    for m in &milestones {
        println!(
            "{:<24}  {:<32}  {}",
            m.slug,
            m.name,
            format!("{:?}", m.status).to_lowercase()
        );
    }

    Ok(0)
}

/// Handle `assay milestone status`.
fn milestone_status_cmd(json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let dir = assay_dir(&root);

    if json {
        match assay_core::milestone::cycle_status(&dir) {
            Ok(Some(status)) => match serde_json::to_string(&status) {
                Ok(s) => println!("{s}"),
                Err(e) => {
                    eprintln!("Error serializing cycle status: {e}");
                    return Ok(1);
                }
            },
            Ok(None) => println!(r#"{{"active":false}}"#),
            Err(e) => {
                eprintln!("Error: {e}");
                return Ok(1);
            }
        }
        return Ok(0);
    }

    let milestones = assay_core::milestone::milestone_scan(&dir)?;

    let active: Vec<_> = milestones
        .iter()
        .filter(|m| m.status == assay_types::MilestoneStatus::InProgress)
        .collect();

    if active.is_empty() {
        println!("No active milestones.");
        return Ok(0);
    }

    for m in &active {
        println!(
            "MILESTONE: {} ({})",
            m.slug,
            format!("{:?}", m.status).to_lowercase()
        );

        // Sort chunks by order ascending
        let mut chunks = m.chunks.clone();
        chunks.sort_by_key(|c| c.order);

        for chunk in &chunks {
            if m.completed_chunks.contains(&chunk.slug) {
                println!("  [x] {}  (complete)", chunk.slug);
            } else {
                println!("  [ ] {}  (active)", chunk.slug);
            }
        }
    }

    Ok(0)
}

/// Handle `assay milestone advance`.
fn milestone_advance_cmd(milestone_slug: Option<String>) -> anyhow::Result<i32> {
    let root = project_root()?;
    let assay_dir = assay_dir(&root);
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            return Ok(1);
        }
    };
    let specs_dir = assay_dir.join(&config.specs_dir);
    let working_dir = root.clone();

    match assay_core::milestone::cycle_advance(
        &assay_dir,
        &specs_dir,
        &working_dir,
        milestone_slug.as_deref(),
    ) {
        Ok(status) => {
            println!(
                "Advanced: {} ({}/{} chunks complete, phase: {:?})",
                status.milestone_slug, status.completed_count, status.total_count, status.phase
            );
            Ok(0)
        }
        Err(e) => {
            eprintln!("Error: {e}");
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_list_subcommand_no_milestones() {
        let dir = tempfile::tempdir().unwrap();
        // Create the .assay directory so project_root resolves correctly,
        // but skip creating .assay/milestones — milestone_scan returns Ok(vec![]) for missing dir.
        std::fs::create_dir_all(dir.path().join(".assay")).unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = handle(MilestoneCommand::List);
        assert!(result.is_ok(), "milestone list should succeed: {result:?}");
        assert_eq!(result.unwrap(), 0, "exit code should be 0");
    }

    #[test]
    fn milestone_status_no_milestones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".assay")).unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = handle(MilestoneCommand::Status { json: false });
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn milestone_status_json_no_active() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".assay")).unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = handle(MilestoneCommand::Status { json: true });
        assert!(
            result.is_ok(),
            "milestone status --json should succeed: {result:?}"
        );
        assert_eq!(result.unwrap(), 0, "exit code should be 0");
    }

    #[test]
    fn milestone_advance_no_active_milestone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".assay")).unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = handle(MilestoneCommand::Advance { milestone: None });
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            1,
            "advance with no milestones should exit 1"
        );
    }
}
