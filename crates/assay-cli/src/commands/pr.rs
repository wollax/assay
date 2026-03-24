//! PR subcommands for the `assay pr` CLI group.

use clap::Subcommand;

use super::{assay_dir, project_root};

#[derive(Subcommand)]
pub(crate) enum PrCommand {
    /// Create a GitHub PR for a milestone after all chunk gates pass
    Create {
        /// Slug of the milestone
        milestone: String,
        /// PR title (defaults to "feat: <milestone-slug>" if omitted)
        #[arg(long)]
        title: Option<String>,
        /// PR body text
        #[arg(long)]
        body: Option<String>,
        /// Additional label to apply to the PR (repeatable)
        #[arg(long = "label")]
        labels: Vec<String>,
        /// Additional reviewer to request on the PR (repeatable)
        #[arg(long = "reviewer")]
        reviewers: Vec<String>,
    },
}

/// Handle pr subcommands.
pub(crate) fn handle(command: PrCommand) -> anyhow::Result<i32> {
    match command {
        PrCommand::Create {
            milestone,
            title,
            body,
            labels,
            reviewers,
        } => pr_create_cmd(milestone, title, body, labels, reviewers),
    }
}

/// Handle `assay pr create <milestone>`.
fn pr_create_cmd(
    milestone: String,
    title: Option<String>,
    body: Option<String>,
    extra_labels: Vec<String>,
    extra_reviewers: Vec<String>,
) -> anyhow::Result<i32> {
    let root = project_root()?;
    let assay_dir = assay_dir(&root);
    let config = match assay_core::config::load(&root) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load config");
            return Ok(1);
        }
    };
    let specs_dir = assay_dir.join(&config.specs_dir);
    let working_dir = root.clone();

    let effective_title = title.unwrap_or_else(|| format!("feat: {milestone}"));

    match assay_core::pr::pr_create_if_gates_pass(
        &assay_dir,
        &specs_dir,
        &working_dir,
        &milestone,
        &effective_title,
        body.as_deref(),
        &extra_labels,
        &extra_reviewers,
    ) {
        Ok(result) => {
            println!("PR created: #{} — {}", result.pr_number, result.pr_url);
            Ok(0)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create PR");
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use serial_test::serial;

    /// Wrapper to test PrCommand parsing via clap.
    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: PrCommand,
    }

    #[test]
    #[serial]
    fn pr_create_cmd_exits_1_no_assay_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = handle(PrCommand::Create {
            milestone: "x".to_string(),
            title: None,
            body: None,
            labels: vec![],
            reviewers: vec![],
        });
        assert!(result.is_ok(), "handle should not return Err: {result:?}");
        assert_eq!(result.unwrap(), 1, "exit code should be 1");
    }

    #[test]
    #[serial]
    fn pr_create_cmd_exits_1_already_created() {
        let dir = tempfile::tempdir().unwrap();
        let assay_milestones = dir.path().join(".assay").join("milestones");
        std::fs::create_dir_all(&assay_milestones).unwrap();

        // Write a milestone TOML with pr_number already set
        let toml_content = r#"slug = "my-feature"
name = "My Feature"
status = "in_progress"
created_at = "2024-01-01T00:00:00Z"
updated_at = "2024-01-01T00:00:00Z"
pr_number = 42
pr_url = "https://github.com/o/r/pull/42"
"#;
        std::fs::write(assay_milestones.join("my-feature.toml"), toml_content).unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = handle(PrCommand::Create {
            milestone: "my-feature".to_string(),
            title: None,
            body: None,
            labels: vec![],
            reviewers: vec![],
        });
        assert!(result.is_ok(), "handle should not return Err: {result:?}");
        assert_eq!(
            result.unwrap(),
            1,
            "exit code should be 1 (PR already created)"
        );
    }

    #[test]
    fn pr_create_parses_label_and_reviewer_flags() {
        let cli = TestCli::try_parse_from([
            "test",
            "create",
            "my-feature",
            "--label",
            "bug",
            "--label",
            "priority",
            "--reviewer",
            "alice",
            "--reviewer",
            "bob",
        ])
        .expect("should parse");

        match cli.command {
            PrCommand::Create {
                milestone,
                labels,
                reviewers,
                ..
            } => {
                assert_eq!(milestone, "my-feature");
                assert_eq!(labels, vec!["bug", "priority"]);
                assert_eq!(reviewers, vec!["alice", "bob"]);
            }
        }
    }

    #[test]
    fn pr_create_label_and_reviewer_default_empty() {
        let cli = TestCli::try_parse_from(["test", "create", "my-feature"]).expect("should parse");

        match cli.command {
            PrCommand::Create {
                labels, reviewers, ..
            } => {
                assert!(labels.is_empty());
                assert!(reviewers.is_empty());
            }
        }
    }
}
