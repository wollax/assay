//! Milestone subcommands for the `assay milestone` CLI group.

use clap::Subcommand;

use super::{assay_dir, project_root};

#[derive(Subcommand)]
pub(crate) enum MilestoneCommand {
    /// List all milestones in the current project
    List,
}

/// Handle milestone subcommands.
pub(crate) fn handle(command: MilestoneCommand) -> anyhow::Result<i32> {
    match command {
        MilestoneCommand::List => milestone_list_cmd(),
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
}
