use anyhow::{Context, bail};
use assay_core::spec::SpecEntry;
use clap::Subcommand;

use super::{ANSI_COLOR_OVERHEAD, assay_dir, colors_enabled, format_criteria_type, project_root};

#[derive(Subcommand)]
pub(crate) enum SpecCommand {
    /// Display a spec's criteria in detail
    #[command(after_long_help = "\
Examples:
  Show criteria as a formatted table:
    assay spec show auth-flow

  Show as JSON (for scripting or agent use):
    assay spec show auth-flow --json")]
    Show {
        /// Spec name (filename without .toml extension)
        name: String,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },
    /// List all available specs
    #[command(after_long_help = "\
Examples:
  List all specs in the project:
    assay spec list")]
    List,
    /// Create a new directory-based spec with template files
    #[command(after_long_help = "\
Examples:
  Create a new feature spec:
    assay spec new auth-flow")]
    New {
        /// Name for the new spec (used as directory name)
        name: String,
    },
}

/// Handle spec subcommands.
pub(crate) fn handle(command: SpecCommand) -> anyhow::Result<i32> {
    match command {
        SpecCommand::Show { name, json } => handle_spec_show(&name, json),
        SpecCommand::List => handle_spec_list(),
        SpecCommand::New { name } => handle_spec_new(&name),
    }
}

/// Handle `assay spec show <name> [--json]`.
fn handle_spec_show(name: &str, json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;

    match entry {
        SpecEntry::Legacy { spec, .. } => {
            if json {
                let output =
                    serde_json::to_string_pretty(&spec).context("failed to serialize spec")?;
                println!("{output}");
                return Ok(0);
            }
            print_spec_table(&spec.name, &spec.description, &spec.criteria);
        }
        SpecEntry::Directory {
            gates, spec_path, ..
        } => {
            if json {
                let mut map = serde_json::Map::new();
                map.insert("format".into(), "directory".into());
                map.insert(
                    "gates".into(),
                    serde_json::to_value(&gates).unwrap_or_default(),
                );
                if let Some(ref sp) = spec_path
                    && let Ok(feature_spec) = assay_core::spec::load_feature_spec(sp)
                {
                    map.insert(
                        "feature_spec".into(),
                        serde_json::to_value(&feature_spec).unwrap_or_default(),
                    );
                }
                let output = serde_json::to_string_pretty(&serde_json::Value::Object(map))
                    .context("failed to serialize spec")?;
                println!("{output}");
                return Ok(0);
            }

            println!("Spec: {} [srs]", gates.name);

            // Show feature spec summary if available
            if let Some(ref sp) = spec_path
                && let Ok(feature_spec) = assay_core::spec::load_feature_spec(sp)
            {
                if let Some(ref overview) = feature_spec.overview
                    && !overview.description.is_empty()
                {
                    println!("Description: {}", overview.description);
                }
                if !feature_spec.requirements.is_empty() {
                    println!(
                        "Requirements: {} ({})",
                        feature_spec.requirements.len(),
                        feature_spec
                            .requirements
                            .iter()
                            .map(|r| r.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
            println!();

            // Convert GateCriteria to Criteria for table display
            let criteria: Vec<_> = gates
                .criteria
                .iter()
                .map(assay_core::gate::to_criterion)
                .collect();
            print_criteria_table(&criteria);

            // Show requirement traceability
            let traced: Vec<_> = gates
                .criteria
                .iter()
                .filter(|c| !c.requirements.is_empty())
                .collect();
            if !traced.is_empty() {
                println!();
                println!("Traceability:");
                for c in &traced {
                    println!("  {} → {}", c.name, c.requirements.join(", "));
                }
            }
        }
    }
    Ok(0)
}

/// Print a spec table header and rows for a list of criteria.
fn print_criteria_table(criteria: &[assay_types::Criterion]) {
    let color = colors_enabled();
    let num_width = criteria.len().to_string().len().max(1);
    let name_width = criteria
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(9)
        .max(9);
    let type_width = 11;

    println!(
        "  {:<num_w$}  {:<name_w$}  {:<type_w$}  Command",
        "#",
        "Criterion",
        "Type",
        num_w = num_width,
        name_w = name_width,
        type_w = type_width,
    );
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

    for (i, criterion) in criteria.iter().enumerate() {
        let type_label =
            format_criteria_type(criterion.cmd.is_some() || criterion.path.is_some(), color);
        let cmd_display = criterion
            .cmd
            .as_deref()
            .or(criterion.path.as_deref())
            .unwrap_or("");

        if color {
            println!(
                "  {:<num_w$}  {:<name_w$}  {:<type_w$}  {cmd_display}",
                i + 1,
                criterion.name,
                type_label,
                num_w = num_width,
                name_w = name_width,
                type_w = type_width + ANSI_COLOR_OVERHEAD,
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

/// Print a full spec table (name + description + criteria).
fn print_spec_table(name: &str, description: &str, criteria: &[assay_types::Criterion]) {
    println!("Spec: {name}");
    if !description.is_empty() {
        println!("Description: {description}");
    }
    println!();
    print_criteria_table(criteria);
}

/// Handle `assay spec list`.
fn handle_spec_list() -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let result = assay_core::spec::scan(&specs_dir)?;

    for err in &result.errors {
        eprintln!("Warning: {err}");
    }

    if result.entries.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return Ok(0);
    }

    let name_width = result
        .entries
        .iter()
        .map(|e| e.slug().len())
        .max()
        .unwrap_or(0);

    println!("Specs:");
    for entry in &result.entries {
        match entry {
            SpecEntry::Legacy { slug, spec } => {
                if spec.description.is_empty() {
                    println!("  {:<width$}", slug, width = name_width);
                } else {
                    println!(
                        "  {:<width$}  {}",
                        slug,
                        spec.description,
                        width = name_width
                    );
                }
            }
            SpecEntry::Directory { slug, gates, .. } => {
                let indicator = "[srs]";
                let criteria_count = gates.criteria.len();
                println!(
                    "  {:<width$}  {indicator} {criteria_count} criteria",
                    slug,
                    width = name_width
                );
            }
        }
    }
    Ok(0)
}

/// Handle `assay spec new <name>`.
///
/// Creates a directory-based spec with template `spec.toml` and `gates.toml`.
fn handle_spec_new(name: &str) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);
    let spec_dir = specs_dir.join(name);

    if spec_dir.exists() {
        bail!("spec directory '{}' already exists", spec_dir.display());
    }

    // Also check for a flat file with the same name
    let flat_path = specs_dir.join(format!("{name}.toml"));
    if flat_path.exists() {
        bail!("spec '{name}.toml' already exists as a flat file");
    }

    std::fs::create_dir_all(&spec_dir).context("could not create spec directory")?;

    let spec_toml = format!(
        r#"name = "{name}"
status = "draft"
version = "0.1"

[overview]
description = ""
functions = []

[[requirements]]
id = "REQ-FUNC-001"
title = ""
statement = ""
obligation = "shall"
priority = "must"
verification = "test"
status = "draft"
"#
    );

    let gates_toml = format!(
        r#"name = "{name}"

[gate]
enforcement = "required"

[[criteria]]
name = "compiles"
description = "{name} compiles without errors"
cmd = "echo 'TODO: add compile check'"
"#
    );

    let spec_path = spec_dir.join("spec.toml");
    let gates_path = spec_dir.join("gates.toml");

    std::fs::write(&spec_path, &spec_toml).context("failed to write spec.toml")?;
    std::fs::write(&gates_path, &gates_toml).context("failed to write gates.toml")?;

    let rel_spec = spec_path.strip_prefix(&root).unwrap_or(&spec_path);
    let rel_gates = gates_path.strip_prefix(&root).unwrap_or(&gates_path);
    println!("  Created spec `{name}`");
    println!("    created {}", rel_spec.display());
    println!("    created {}", rel_gates.display());
    Ok(0)
}
