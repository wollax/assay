//! `assay criteria` CLI: list and new subcommands.

use std::io::Write;

use anyhow::{Context, Result};
use assay_types::{CriteriaLibrary, CriteriaWizardInput, CriterionInput};

#[derive(clap::Subcommand, Debug)]
pub enum CriteriaCommand {
    /// List all criteria libraries under `.assay/criteria/`.
    List(ListArgs),
    /// Interactively create a new criteria library.
    New,
}

#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Include description, version, and tags for each library.
    #[arg(long)]
    pub verbose: bool,
    /// Emit the full Vec<CriteriaLibrary> as JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

pub(crate) fn handle(command: CriteriaCommand) -> anyhow::Result<i32> {
    match command {
        CriteriaCommand::List(args) => handle_list(args),
        CriteriaCommand::New => handle_new(),
    }
}

pub(crate) fn handle_list(args: ListArgs) -> anyhow::Result<i32> {
    let root = crate::commands::project_root()?;
    let assay_dir = crate::commands::assay_dir(&root);
    let libs = assay_core::spec::compose::scan_libraries(&assay_dir)?;
    render_list(&libs, &args, &mut std::io::stdout())?;
    Ok(0)
}

fn render_list<W: Write>(libs: &[CriteriaLibrary], args: &ListArgs, out: &mut W) -> Result<()> {
    if args.json {
        let payload = serde_json::to_string_pretty(libs).context("serializing libraries")?;
        writeln!(out, "{payload}")?;
        return Ok(());
    }
    if libs.is_empty() {
        writeln!(out, "No criteria libraries found.")?;
        return Ok(());
    }
    for lib in libs {
        writeln!(out, "{:<32}  {} criteria", lib.name, lib.criteria.len())?;
        if args.verbose {
            if !lib.description.is_empty() {
                writeln!(out, "    description: {}", lib.description)?;
            }
            if let Some(ref v) = lib.version {
                writeln!(out, "    version:     {v}")?;
            }
            if !lib.tags.is_empty() {
                writeln!(out, "    tags:        {}", lib.tags.join(", "))?;
            }
        }
    }
    Ok(())
}

pub(crate) fn handle_new() -> anyhow::Result<i32> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        tracing::error!("assay criteria new requires an interactive terminal.");
        return Ok(1);
    }

    let root = crate::commands::project_root()?;
    let assay_dir = crate::commands::assay_dir(&root);

    // 1. Slug (inline validation via shared helper)
    let name = crate::commands::wizard_helpers::prompt_slug("Library slug", None)?;

    // 2. Criteria (shared loop; start empty)
    let criteria = crate::commands::wizard_helpers::prompt_criteria_loop(&[])?;

    // 3. Metadata opt-in
    let add_meta = dialoguer::Confirm::new()
        .with_prompt("Add metadata (description, version, tags)?")
        .default(false)
        .interact()?;
    let (description, version, tags) = if add_meta {
        let description: String = dialoguer::Input::new()
            .with_prompt("Description")
            .allow_empty(true)
            .interact_text()?;
        let version_raw: String = dialoguer::Input::new()
            .with_prompt("Version (Enter to skip)")
            .allow_empty(true)
            .interact_text()?;
        let version = if version_raw.trim().is_empty() {
            None
        } else {
            Some(version_raw.trim().to_string())
        };
        let tags_raw: String = dialoguer::Input::new()
            .with_prompt("Tags (comma-separated; Enter to skip)")
            .allow_empty(true)
            .interact_text()?;
        let tags: Vec<String> = tags_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        (description, version, tags)
    } else {
        (String::new(), None, Vec::new())
    };

    let input = build_input(name.clone(), description, version, tags, criteria, false);
    let output = assay_core::wizard::apply_criteria_wizard(&input, &assay_dir)?;
    println!("  Created criteria library '{name}'");
    println!("    written {}", output.path.display());
    println!("    {} criteria", output.library.criteria.len());
    Ok(0)
}

/// Pure input builder; extracted so unit tests can verify field mapping without dialoguer.
fn build_input(
    name: String,
    description: String,
    version: Option<String>,
    tags: Vec<String>,
    criteria: Vec<CriterionInput>,
    overwrite: bool,
) -> CriteriaWizardInput {
    CriteriaWizardInput {
        name,
        description,
        version,
        tags,
        criteria,
        overwrite,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::Criterion;
    use assay_types::criterion::When;

    fn make_criterion(name: &str) -> Criterion {
        Criterion {
            name: name.to_string(),
            description: format!("description for {name}"),
            cmd: Some("echo ok".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: When::default(),
        }
    }

    fn make_library(name: &str, n: usize) -> CriteriaLibrary {
        CriteriaLibrary {
            name: name.to_string(),
            description: format!("desc for {name}"),
            version: Some("0.1.0".to_string()),
            tags: vec!["rust".to_string()],
            criteria: (0..n).map(|i| make_criterion(&format!("c{i}"))).collect(),
        }
    }

    #[test]
    fn criteria_list_format_default() {
        let libs = vec![make_library("lib-a", 3), make_library("lib-b", 0)];
        let mut out: Vec<u8> = Vec::new();
        render_list(
            &libs,
            &ListArgs {
                verbose: false,
                json: false,
            },
            &mut out,
        )
        .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("lib-a"), "output should contain 'lib-a': {s}");
        assert!(
            s.contains("3 criteria"),
            "output should contain '3 criteria': {s}"
        );
        assert!(s.contains("lib-b"), "output should contain 'lib-b': {s}");
        assert!(
            s.contains("0 criteria"),
            "output should contain '0 criteria': {s}"
        );
    }

    #[test]
    fn criteria_list_format_json() {
        let libs = vec![make_library("lib-a", 2)];
        let mut out: Vec<u8> = Vec::new();
        render_list(
            &libs,
            &ListArgs {
                verbose: false,
                json: true,
            },
            &mut out,
        )
        .unwrap();
        let parsed: Vec<CriteriaLibrary> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "lib-a");
        assert_eq!(parsed[0].criteria.len(), 2);
    }

    #[test]
    fn criteria_list_format_verbose() {
        let libs = vec![make_library("lib-a", 1)];
        let mut out: Vec<u8> = Vec::new();
        render_list(
            &libs,
            &ListArgs {
                verbose: true,
                json: false,
            },
            &mut out,
        )
        .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(
            s.contains("description: desc for lib-a"),
            "missing description: {s}"
        );
        assert!(s.contains("version:     0.1.0"), "missing version: {s}");
    }

    #[test]
    fn criteria_list_empty() {
        let libs: Vec<CriteriaLibrary> = vec![];
        let mut out: Vec<u8> = Vec::new();
        render_list(
            &libs,
            &ListArgs {
                verbose: false,
                json: false,
            },
            &mut out,
        )
        .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(
            s.contains("No criteria libraries"),
            "expected friendly message: {s}"
        );
    }

    #[test]
    fn handle_new_non_tty() {
        // In cargo test context, stdin is not a TTY — guard fires and returns Ok(1).
        let rc = handle_new().unwrap();
        assert_eq!(rc, 1);
    }

    #[test]
    fn handle_new_builds_input() {
        let input = build_input(
            "lib".to_string(),
            "desc".to_string(),
            Some("0.1.0".to_string()),
            vec!["t".to_string()],
            vec![CriterionInput {
                name: "c".to_string(),
                description: "d".to_string(),
                cmd: None,
            }],
            false,
        );
        assert_eq!(input.name, "lib");
        assert_eq!(input.description, "desc");
        assert_eq!(input.version, Some("0.1.0".to_string()));
        assert_eq!(input.tags, vec!["t".to_string()]);
        assert_eq!(input.criteria.len(), 1);
        assert!(!input.overwrite);
    }
}
