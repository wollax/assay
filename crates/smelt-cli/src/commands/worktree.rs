//! `smelt worktree` (alias: `smelt wt`) command handlers.

use std::path::PathBuf;

use clap::Subcommand;
use smelt_core::{CreateWorktreeOpts, GitCli, SmeltError, WorktreeManager};

/// Resolve a path to an absolute display string, collapsing `..` segments.
fn display_path(path: &std::path::Path) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

/// Worktree subcommands for managing agent session worktrees.
#[derive(Subcommand)]
pub enum WorktreeCommands {
    /// Create a new worktree for an agent session
    Create {
        /// Session name (used for branch and directory naming)
        name: String,

        /// Base branch or commit (defaults to HEAD)
        #[arg(long, default_value = "HEAD")]
        base: String,

        /// Custom worktree directory name
        #[arg(long)]
        dir_name: Option<String>,

        /// Task description for the session
        #[arg(long)]
        task: Option<String>,
    },

    /// List all tracked worktrees
    List {
        /// Show detailed output including base ref and timestamps
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Execute the `worktree create` subcommand.
pub async fn execute_create(
    git: GitCli,
    repo_root: PathBuf,
    name: &str,
    base: &str,
    dir_name: Option<String>,
    task: Option<String>,
) -> anyhow::Result<i32> {
    let manager = WorktreeManager::new(git, repo_root);

    let opts = CreateWorktreeOpts {
        session_name: name.to_string(),
        base: base.to_string(),
        dir_name,
        task_description: task,
        file_scope: None,
    };

    match manager.create(opts).await {
        Ok(info) => {
            let abs_path = std::path::absolute(&info.worktree_path)
                .unwrap_or_else(|_| info.worktree_path.clone());
            println!("Created worktree '{}'", info.session_name);
            println!("  Branch: {}", info.branch_name);
            println!("  Path:   {}", abs_path.display());
            Ok(0)
        }
        Err(SmeltError::NotInitialized) => {
            eprintln!("Error: not a Smelt project (run `smelt init` first)");
            Ok(1)
        }
        Err(SmeltError::WorktreeExists { name }) => {
            eprintln!("Error: worktree '{name}' already exists");
            Ok(1)
        }
        Err(SmeltError::BranchExists { branch }) => {
            eprintln!("Error: branch '{branch}' already exists");
            Ok(1)
        }
        Err(e) => Err(e.into()),
    }
}

/// Execute the `worktree list` subcommand.
pub async fn execute_list(
    git: GitCli,
    repo_root: PathBuf,
    verbose: bool,
) -> anyhow::Result<i32> {
    let manager = WorktreeManager::new(git, repo_root);

    let worktrees = manager.list().await?;

    if worktrees.is_empty() {
        println!("No worktrees tracked.");
        return Ok(0);
    }

    if verbose {
        // Verbose: NAME | BRANCH | STATUS | BASE | CREATED | PATH
        println!(
            "{:<20} {:<30} {:<12} {:<10} {:<22} PATH",
            "NAME", "BRANCH", "STATUS", "BASE", "CREATED"
        );
        println!("{}", "-".repeat(110));
        for wt in &worktrees {
            let status = format!("{:?}", wt.status).to_lowercase();
            let created = wt.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            println!(
                "{:<20} {:<30} {:<12} {:<10} {:<22} {}",
                wt.session_name,
                wt.branch_name,
                status,
                wt.base_ref,
                created,
                display_path(&wt.worktree_path),
            );
        }
    } else {
        // Compact: NAME | BRANCH | STATUS | PATH
        println!(
            "{:<20} {:<30} {:<12} PATH",
            "NAME", "BRANCH", "STATUS"
        );
        println!("{}", "-".repeat(80));
        for wt in &worktrees {
            let status = format!("{:?}", wt.status).to_lowercase();
            println!(
                "{:<20} {:<30} {:<12} {}",
                wt.session_name,
                wt.branch_name,
                status,
                display_path(&wt.worktree_path),
            );
        }
    }

    Ok(0)
}
