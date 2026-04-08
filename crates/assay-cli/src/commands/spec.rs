use anyhow::{Context, bail};
use assay_core::spec::SpecEntry;
use assay_types::Severity;
use assay_types::feature_spec::SpecStatus;
use clap::Subcommand;

use super::{
    ANSI_COLOR_OVERHEAD, COLUMN_GAP, assay_dir, colors_enabled, format_criteria_type, project_root,
};

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
    /// Validate a spec and display diagnostics
    #[command(after_long_help = "\
Examples:
  Validate a spec and show diagnostics table:
    assay spec validate auth-flow

  Output diagnostics as JSON:
    assay spec validate auth-flow --json

  Also check that command binaries exist on PATH:
    assay spec validate auth-flow --check-commands")]
    Validate {
        /// Spec name (filename without .toml extension, or directory name)
        name: String,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
        /// Also check that command binaries exist on PATH
        #[arg(long)]
        check_commands: bool,
    },
    /// Show requirements coverage for a spec
    #[command(after_long_help = "\
Examples:
  Show coverage table:
    assay spec coverage auth-flow

  Output as JSON:
    assay spec coverage auth-flow --json")]
    Coverage {
        /// Spec name (directory name or filename without .toml extension)
        name: String,
        /// Output as JSON instead of table
        #[arg(long)]
        json: bool,
    },
    /// Promote a spec's lifecycle status
    #[command(after_long_help = "\
Examples:
  Advance to the next status:
    assay spec promote auth-flow

  Jump directly to a specific status:
    assay spec promote auth-flow --to planned

Valid statuses: draft, proposed, planned, in-progress, verified, deprecated")]
    Promote {
        /// Spec name (directory name)
        name: String,
        /// Target status to set directly (e.g. planned, in-progress)
        #[arg(long)]
        to: Option<String>,
    },
    /// Run structural review checks on a spec
    #[command(after_long_help = "\
Examples:
  Run structural review:
    assay spec review auth-flow

  Output review report as JSON:
    assay spec review auth-flow --json

  List past reviews:
    assay spec review auth-flow --list

  Also run agent quality pass (S05, currently no-op):
    assay spec review auth-flow --agent")]
    Review {
        /// Spec name (directory name or filename without .toml extension)
        name: String,
        /// Output ReviewReport as JSON to stdout
        #[arg(long)]
        json: bool,
        /// List past review reports instead of running a new review
        #[arg(long)]
        list: bool,
        /// Also run agent quality pass (no-op until S05)
        #[arg(long)]
        agent: bool,
    },
}

/// Handle spec subcommands.
pub(crate) fn handle(command: SpecCommand) -> anyhow::Result<i32> {
    match command {
        SpecCommand::Show { name, json } => handle_spec_show(&name, json),
        SpecCommand::List => handle_spec_list(),
        SpecCommand::New { name } => handle_spec_new(&name),
        SpecCommand::Validate {
            name,
            json,
            check_commands,
        } => handle_spec_validate(&name, json, check_commands),
        SpecCommand::Coverage { name, json } => handle_spec_coverage(&name, json),
        SpecCommand::Promote { name, to } => handle_spec_promote(&name, to.as_deref()),
        SpecCommand::Review {
            name,
            json,
            list,
            agent: _,
        } => handle_spec_review(&name, json, list),
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

            println!(
                "Spec: {} {}",
                gates.name,
                assay_types::DIRECTORY_SPEC_INDICATOR
            );

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
        "  {:<num_w$}{gap}{:<name_w$}{gap}{:<type_w$}{gap}Command",
        "#",
        "Criterion",
        "Type",
        num_w = num_width,
        name_w = name_width,
        type_w = type_width,
        gap = COLUMN_GAP,
    );
    println!(
        "  {:<num_w$}{gap}{:<name_w$}{gap}{:<type_w$}{gap}{}",
        "\u{2500}".repeat(num_width),
        "\u{2500}".repeat(name_width),
        "\u{2500}".repeat(type_width),
        "\u{2500}".repeat(7),
        num_w = num_width,
        name_w = name_width,
        type_w = type_width,
        gap = COLUMN_GAP,
    );

    for (i, criterion) in criteria.iter().enumerate() {
        let type_label =
            format_criteria_type(criterion.cmd.is_some() || criterion.path.is_some(), color);
        let cmd_display = criterion
            .cmd
            .as_deref()
            .or(criterion.path.as_deref())
            .unwrap_or("");

        let tw = if color {
            type_width + ANSI_COLOR_OVERHEAD
        } else {
            type_width
        };
        println!(
            "  {:<num_w$}{gap}{:<name_w$}{gap}{:<type_w$}{gap}{cmd_display}",
            i + 1,
            criterion.name,
            type_label,
            num_w = num_width,
            name_w = name_width,
            type_w = tw,
            gap = COLUMN_GAP,
        );
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
        tracing::warn!("{err}");
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
                        "  {:<width$}{gap}{}",
                        slug,
                        spec.description,
                        width = name_width,
                        gap = COLUMN_GAP,
                    );
                }
            }
            SpecEntry::Directory { slug, gates, .. } => {
                let indicator = assay_types::DIRECTORY_SPEC_INDICATOR;
                let criteria_count = gates.criteria.len();
                println!(
                    "  {:<width$}{gap}{indicator} {criteria_count} criteria",
                    slug,
                    width = name_width,
                    gap = COLUMN_GAP,
                );
            }
        }
    }
    Ok(0)
}

/// Handle `assay spec validate <name> [--json] [--check-commands]`.
fn handle_spec_validate(name: &str, json: bool, check_commands: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;
    let result = assay_core::spec::validate::validate_spec_with_dependencies(
        &entry,
        check_commands,
        &specs_dir,
    );

    if json {
        let output = serde_json::to_string_pretty(&result)
            .context("failed to serialize ValidationResult")?;
        println!("{output}");
    } else {
        let color = colors_enabled();
        if result.diagnostics.is_empty() {
            println!("✓ {} — no diagnostics", result.spec);
        } else {
            // Compute column widths.
            // sev_width must accommodate the header "Severity" (8 chars) as well as
            // the widest data value "warning" (7 chars). Use 8 so the header fits.
            let sev_width = "Severity".len(); // 8
            let loc_width = result
                .diagnostics
                .iter()
                .map(|d| d.location.len())
                .max()
                .unwrap_or(8)
                .max(8);

            println!(
                "  {:<sev_w$}{gap}{:<loc_w$}{gap}Message",
                "Severity",
                "Location",
                sev_w = sev_width,
                loc_w = loc_width,
                gap = COLUMN_GAP,
            );
            println!(
                "  {:<sev_w$}{gap}{:<loc_w$}{gap}{}",
                "\u{2500}".repeat(sev_width),
                "\u{2500}".repeat(loc_width),
                "\u{2500}".repeat(7),
                sev_w = sev_width,
                loc_w = loc_width,
                gap = COLUMN_GAP,
            );

            for diag in &result.diagnostics {
                // Each label must be exactly `sev_width` (8) visible chars wide so
                // the Location column starts at the same offset as the header/separator.
                // ANSI codes are invisible bytes — they do not consume display width.
                let sev_label = match diag.severity {
                    Severity::Error => {
                        if color {
                            "\x1b[31merror\x1b[0m   " // 5 + 3 spaces = 8 visible
                        } else {
                            "error   " // 5 + 3 spaces = 8
                        }
                    }
                    Severity::Warning => {
                        if color {
                            "\x1b[33mwarning\x1b[0m " // 7 + 1 space = 8 visible
                        } else {
                            "warning " // 7 + 1 space = 8
                        }
                    }
                    Severity::Info => {
                        if color {
                            "\x1b[34minfo\x1b[0m    " // 4 + 4 spaces = 8 visible
                        } else {
                            "info    " // 4 + 4 spaces = 8
                        }
                    }
                };
                println!(
                    "  {sev_label}{gap}{:<loc_w$}{gap}{}",
                    diag.location,
                    diag.message,
                    loc_w = loc_width,
                    gap = COLUMN_GAP,
                );
            }
            println!();
        }

        // Summary line
        let s = &result.summary;
        println!(
            "{}: {} error(s), {} warning(s), {} info(s)",
            result.spec, s.errors, s.warnings, s.infos,
        );
    }

    Ok(if result.valid { 0 } else { 1 })
}

/// Handle `assay spec coverage <name> [--json]`.
fn handle_spec_coverage(name: &str, json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;

    let (gates, feature) = match &entry {
        SpecEntry::Legacy { .. } => {
            // Legacy flat specs have no FeatureSpec — no requirements to report.
            println!("No spec.toml found for '{name}' — no requirements to report");
            return Ok(0);
        }
        SpecEntry::Directory {
            gates, spec_path, ..
        } => {
            let feature = spec_path
                .as_ref()
                .and_then(|p| assay_core::spec::load_feature_spec(p).ok());
            (gates, feature)
        }
    };

    let Some(feature) = feature else {
        println!("No spec.toml found for '{name}' — no requirements to report");
        return Ok(0);
    };

    let report = assay_core::spec::coverage::compute_coverage(name, gates, Some(&feature));

    if json {
        let output =
            serde_json::to_string_pretty(&report).context("failed to serialize CoverageReport")?;
        println!("{output}");
        return Ok(0);
    }

    // Print headline
    println!(
        "{}/{} requirements covered ({:.1}%)",
        report.covered.len(),
        report.total_requirements,
        report.coverage_pct,
    );
    println!();

    if report.covered.is_empty() && report.uncovered.is_empty() && report.orphaned.is_empty() {
        println!("No requirements declared");
        return Ok(0);
    }

    if !report.covered.is_empty() {
        println!("Covered:");
        for id in &report.covered {
            println!("  ✓ {id}");
        }
        println!();
    }

    if !report.uncovered.is_empty() {
        println!("Uncovered:");
        for id in &report.uncovered {
            println!("  ✗ {id}");
        }
        println!();
    }

    if !report.orphaned.is_empty() {
        println!("Orphaned (criterion references unknown REQ-ID):");
        for id in &report.orphaned {
            println!("  ? {id}");
        }
        println!();
    }

    Ok(0)
}

/// Parse a lifecycle status string (kebab-case) into a `SpecStatus`.
///
/// Uses serde's JSON deserialization since `SpecStatus` derives
/// `serde(rename_all = "kebab-case")`. The string is wrapped in JSON quotes
/// so serde sees a valid JSON string value.
fn parse_spec_status(s: &str) -> Option<SpecStatus> {
    // Escape any JSON metacharacters so the format produces valid JSON.
    // serde_json::to_string wraps the str in quotes and escapes as needed.
    let json = serde_json::to_string(s).ok()?;
    serde_json::from_str(&json).ok()
}

/// Handle `assay spec promote <name> [--to <status>]`.
fn handle_spec_promote(name: &str, to: Option<&str>) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);

    let target = match to {
        Some(s) => match parse_spec_status(s) {
            Some(status) => Some(status),
            None => bail!(
                "invalid status '{}'. Valid values: draft, proposed, planned, in-progress, verified, deprecated",
                s
            ),
        },
        None => None,
    };

    let (old, new) = assay_core::spec::promote::promote_spec(&specs_dir, name, target)?;
    println!("{name}: {old} → {new}");
    Ok(0)
}

/// Handle `assay spec review <name> [--json] [--list] [--agent]`.
fn handle_spec_review(name: &str, json: bool, list: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let config = assay_core::config::load(&root)?;
    let ad = assay_dir(&root);

    if list {
        let reviews = assay_core::review::list_reviews(&ad, name)?;
        if reviews.is_empty() {
            println!("No past reviews for '{name}'");
            return Ok(0);
        }
        // Print table header.
        println!(
            "  {:<26}  {:>6}  {:>6}  {:>7}",
            "Timestamp", "Passed", "Failed", "Skipped"
        );
        println!(
            "  {}  {}  {}  {}",
            "\u{2500}".repeat(26),
            "\u{2500}".repeat(6),
            "\u{2500}".repeat(6),
            "\u{2500}".repeat(7),
        );
        for r in &reviews {
            println!(
                "  {:<26}  {:>6}  {:>6}  {:>7}",
                r.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                r.passed,
                r.failed,
                r.skipped,
            );
        }
        return Ok(0);
    }

    let specs_dir = ad.join(&config.specs_dir);
    let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, &specs_dir)?;

    let report = match &entry {
        SpecEntry::Legacy { spec, .. } => {
            // Build a GatesSpec-like wrapper from the legacy Spec.
            let gates_spec = assay_types::GatesSpec {
                name: spec.name.clone(),
                description: spec.description.clone(),
                gate: None,
                depends: vec![],
                milestone: None,
                order: None,
                criteria: spec.criteria.clone(),
            };
            assay_core::review::run_structural_review(name, &gates_spec, None)
        }
        SpecEntry::Directory {
            gates, spec_path, ..
        } => {
            let feature = spec_path.as_ref().and_then(|p| {
                match assay_core::spec::load_feature_spec(p) {
                    Ok(f) => Some(f),
                    Err(ref e)
                        if matches!(
                            e,
                            assay_core::error::AssayError::Io { .. }
                        ) && p.exists() =>
                    {
                        tracing::warn!(
                            path = %p.display(),
                            error = %e,
                            "spec.toml exists but could not be loaded — structural checks that require FeatureSpec will be skipped"
                        );
                        None
                    }
                    Err(_) => None,
                }
            });
            assay_core::review::run_structural_review(name, gates, feature.as_ref())
        }
    };

    // Save the review.
    let path = assay_core::review::save_review(&ad, &report)?;
    eprintln!("Review saved: {}", path.display());

    if json {
        // Read the saved file (it has the run_id populated).
        let content = std::fs::read_to_string(&path).context("failed to read saved review")?;
        println!("{content}");
    } else {
        // Print pass/fail table.
        let color = colors_enabled();
        println!("Review: {name}");
        println!();
        for check in &report.checks {
            let symbol = if check.skipped {
                if color { "\x1b[33m⊘\x1b[0m" } else { "⊘" }
            } else if check.passed {
                if color { "\x1b[32m✓\x1b[0m" } else { "✓" }
            } else if color {
                "\x1b[31m✗\x1b[0m"
            } else {
                "✗"
            };
            println!("  {} {}: {}", symbol, check.name, check.message);
            if let Some(details) = &check.details {
                println!("    {details}");
            }
        }
        println!();
        println!(
            "Passed: {}  Failed: {}  Skipped: {}",
            report.passed, report.failed, report.skipped
        );

        // Show gate diagnostics from the most recent run (if any).
        let diagnostics = assay_core::review::list_gate_diagnostics(&ad, name)?;
        if let Some(diag) = diagnostics.first() {
            println!();
            println!("Gate Diagnostics (most recent run):");
            println!(
                "  Run: {} ({})",
                diag.run_id,
                diag.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!("  Passed: {}  Failed: {}", diag.passed, diag.failed);
            for fc in &diag.failed_criteria {
                println!("  ✗ {}", fc.criterion_name);
                if let Some(cmd) = &fc.command {
                    println!("    cmd: {cmd}");
                }
                if let Some(code) = fc.exit_code {
                    println!("    exit: {code}");
                }
                if !fc.stderr_snippet.is_empty() {
                    println!("    stderr: {}", fc.stderr_snippet);
                }
            }
        }

        // Show checkpoint metadata from diagnostics (S04).
        let checkpoint_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.checkpoint_index.is_some())
            .collect();
        if !checkpoint_diags.is_empty() {
            println!();
            println!("Checkpoints:");
            for d in &checkpoint_diags {
                let idx = d.checkpoint_index.unwrap();
                let phase_str = match &d.session_phase {
                    assay_types::review::SessionPhase::AtToolCall { n } => {
                        format!("at_tool_call({n})")
                    }
                    assay_types::review::SessionPhase::AtEvent { event_type } => {
                        format!("at_event({event_type})")
                    }
                    assay_types::review::SessionPhase::SessionEnd => "session_end".to_string(),
                };
                println!(
                    "  [{idx}] phase: {phase_str}  passed: {passed}  failed: {failed}",
                    passed = d.passed,
                    failed = d.failed,
                );
                for fc in &d.failed_criteria {
                    println!("      ✗ {}", fc.criterion_name);
                }
            }
        }

        // Show auto-promotion from the most recent work session (S04).
        if let Ok(session_ids) = assay_core::work_session::list_sessions(&ad) {
            let mut recent_session: Option<assay_types::WorkSession> = None;
            for sid in &session_ids {
                if let Ok(s) = assay_core::work_session::load_session(&ad, sid)
                    && s.spec_name == name
                    && recent_session
                        .as_ref()
                        .is_none_or(|prev| s.created_at > prev.created_at)
                {
                    recent_session = Some(s);
                }
            }
            if let Some(session) = recent_session
                && session.auto_promoted
            {
                let target = session
                    .promoted_to
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                println!();
                println!("Auto-promotion: in-progress → {target}");
            }
        }
    }

    // Exit 0 if all pass, 1 if any fail.
    if report.failed > 0 { Ok(1) } else { Ok(0) }
}

/// Testable core of `handle_spec_validate` that takes an explicit specs_dir.
#[cfg(test)]
fn validate_and_exit_code(
    name: &str,
    json: bool,
    check_commands: bool,
    specs_dir: &std::path::Path,
) -> anyhow::Result<(i32, assay_types::ValidationResult)> {
    let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, specs_dir)?;
    let result = assay_core::spec::validate::validate_spec_with_dependencies(
        &entry,
        check_commands,
        specs_dir,
    );
    let exit_code = if result.valid { 0 } else { 1 };

    if json {
        let _output = serde_json::to_string_pretty(&result)
            .context("failed to serialize ValidationResult")?;
    }

    Ok((exit_code, result))
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

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::CoverageReport;

    /// Testable core of `handle_spec_coverage` that accepts an explicit specs_dir
    /// (bypasses project_root discovery).
    fn coverage_report_from(
        name: &str,
        json: bool,
        specs_dir: &std::path::Path,
    ) -> anyhow::Result<(i32, Option<CoverageReport>)> {
        let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, specs_dir)?;

        let (gates, feature) = match &entry {
            SpecEntry::Legacy { .. } => return Ok((0, None)),
            SpecEntry::Directory {
                gates, spec_path, ..
            } => {
                let feature = spec_path
                    .as_ref()
                    .and_then(|p| assay_core::spec::load_feature_spec(p).ok());
                (gates, feature)
            }
        };

        let Some(feature) = feature else {
            return Ok((0, None));
        };

        let report = assay_core::spec::coverage::compute_coverage(name, gates, Some(&feature));

        if json {
            // Exercise the serialization path and verify round-trip.
            let json_str = serde_json::to_string_pretty(&report)
                .context("failed to serialize CoverageReport")?;
            let _: CoverageReport = serde_json::from_str(&json_str)
                .context("CoverageReport JSON did not round-trip")?;
        }

        Ok((0, Some(report)))
    }

    /// Create a valid flat spec file in a tempdir and return the specs_dir path.
    fn write_valid_flat_spec(dir: &std::path::Path, name: &str) {
        let content = format!(
            r#"name = "{name}"
description = "a valid spec"

[[criteria]]
name = "check"
description = "always passes"
cmd = "echo ok"
"#
        );
        std::fs::write(dir.join(format!("{name}.toml")), content).unwrap();
    }

    /// Create a spec with a validation error (empty depends entry).
    fn write_invalid_flat_spec(dir: &std::path::Path, name: &str) {
        let content = format!(
            r#"name = "{name}"
description = "has an empty depends entry"
depends = [""]

[[criteria]]
name = "check"
description = "always passes"
cmd = "echo ok"
"#
        );
        std::fs::write(dir.join(format!("{name}.toml")), content).unwrap();
    }

    /// Create a valid directory spec with a warning (AgentReport criterion without prompt).
    fn write_warning_dir_spec(dir: &std::path::Path, name: &str) {
        let spec_dir = dir.join(name);
        std::fs::create_dir_all(&spec_dir).unwrap();
        let gates = format!(
            r#"name = "{name}"

[gate]
enforcement = "required"

[[criteria]]
name = "agent-check"
description = "agent report"
kind = "AgentReport"
"#
        );
        std::fs::write(spec_dir.join("gates.toml"), gates).unwrap();
    }

    #[test]
    fn test_spec_validate_valid_spec_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_valid_flat_spec(specs_dir, "good-spec");

        let (exit_code, result) =
            validate_and_exit_code("good-spec", false, false, specs_dir).unwrap();
        assert_eq!(exit_code, 0, "valid spec should exit 0");
        assert!(result.valid);
        assert_eq!(result.summary.errors, 0);
    }

    #[test]
    fn test_spec_validate_invalid_spec_returns_one() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_invalid_flat_spec(specs_dir, "bad-spec");

        let (exit_code, result) =
            validate_and_exit_code("bad-spec", false, false, specs_dir).unwrap();
        assert_eq!(exit_code, 1, "invalid spec should exit 1");
        assert!(!result.valid);
        assert!(result.summary.errors > 0);
    }

    #[test]
    fn test_spec_validate_warnings_only_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_warning_dir_spec(specs_dir, "warn-spec");

        let (exit_code, result) =
            validate_and_exit_code("warn-spec", false, false, specs_dir).unwrap();
        assert_eq!(exit_code, 0, "warnings-only should exit 0");
        assert!(result.valid);
        assert_eq!(result.summary.errors, 0);
        assert!(
            result.summary.warnings > 0,
            "should have warnings: {result:?}"
        );
    }

    #[test]
    fn test_spec_validate_json_output_parses() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_valid_flat_spec(specs_dir, "json-spec");

        let (exit_code, result) =
            validate_and_exit_code("json-spec", true, false, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        // Verify JSON round-trip
        let json_str = serde_json::to_string_pretty(&result).unwrap();
        let parsed: assay_types::ValidationResult = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.spec, result.spec);
        assert_eq!(parsed.valid, result.valid);
    }

    #[test]
    fn test_spec_validate_unknown_spec_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        // Empty specs dir — no spec file
        let result = validate_and_exit_code("nonexistent", false, false, specs_dir);
        assert!(result.is_err(), "unknown spec should return an error");
    }

    // --- Coverage tests ---

    /// Create a directory spec with spec.toml and gates.toml for coverage testing.
    /// Requirements: REQ-AUTH-001 (covered), REQ-AUTH-002 (uncovered).
    /// Criteria: c1 references REQ-AUTH-001 + REQ-ORPHAN-001 (orphaned).
    fn write_coverage_dir_spec(dir: &std::path::Path, name: &str) {
        let spec_dir = dir.join(name);
        std::fs::create_dir_all(&spec_dir).unwrap();
        let spec_toml = format!(
            r#"name = "{name}"
status = "draft"
version = "0.1"

[overview]
description = "Test spec for coverage"
functions = []

[[requirements]]
id = "REQ-AUTH-001"
title = "First requirement"
statement = "The system shall authenticate users."
obligation = "shall"
priority = "must"
verification = "test"
status = "draft"

[[requirements]]
id = "REQ-AUTH-002"
title = "Second requirement"
statement = "The system shall authorize access."
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
name = "auth-check"
description = "Verifies authentication"
cmd = "echo ok"
requirements = ["REQ-AUTH-001", "REQ-ORPHAN-001"]
"#
        );
        std::fs::write(spec_dir.join("spec.toml"), spec_toml).unwrap();
        std::fs::write(spec_dir.join("gates.toml"), gates_toml).unwrap();
    }

    /// Create a directory spec with only gates.toml (no spec.toml).
    fn write_gates_only_dir_spec(dir: &std::path::Path, name: &str) {
        let spec_dir = dir.join(name);
        std::fs::create_dir_all(&spec_dir).unwrap();
        let gates_toml = format!(
            r#"name = "{name}"

[gate]
enforcement = "required"

[[criteria]]
name = "check"
description = "Basic check"
cmd = "echo ok"
"#
        );
        std::fs::write(spec_dir.join("gates.toml"), gates_toml).unwrap();
    }

    #[test]
    fn test_spec_coverage_mixed_coverage() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_coverage_dir_spec(specs_dir, "auth-flow");

        let (exit_code, report) = coverage_report_from("auth-flow", false, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        let report = report.expect("should have a coverage report");
        assert_eq!(report.total_requirements, 2);
        assert_eq!(report.covered, vec!["REQ-AUTH-001"]);
        assert_eq!(report.uncovered, vec!["REQ-AUTH-002"]);
        assert_eq!(report.orphaned, vec!["REQ-ORPHAN-001"]);
        assert!((report.coverage_pct - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_spec_coverage_no_spec_toml_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_gates_only_dir_spec(specs_dir, "gates-only");

        let (exit_code, report) = coverage_report_from("gates-only", false, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        assert!(report.is_none(), "gates-only spec should return None");
    }

    #[test]
    fn test_spec_coverage_legacy_spec_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_valid_flat_spec(specs_dir, "legacy");

        let (exit_code, report) = coverage_report_from("legacy", false, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        assert!(report.is_none(), "legacy spec should return None");
    }

    #[test]
    fn test_spec_coverage_zero_coverage() {
        // A spec with requirements but NO criteria that reference them.
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();

        let spec_dir = specs_dir.join("zero-cov");
        std::fs::create_dir_all(&spec_dir).unwrap();
        let spec_toml = r#"name = "zero-cov"
status = "draft"
version = "0.1"

[overview]
description = "Zero coverage spec"
functions = []

[[requirements]]
id = "REQ-ZERO-001"
title = "Uncovered requirement"
statement = "No criterion references this."
obligation = "shall"
priority = "must"
verification = "test"
status = "draft"
"#;
        let gates_toml = r#"name = "zero-cov"

[[criteria]]
name = "no-req-ref"
description = "Criterion with no requirements references"
cmd = "echo ok"
"#;
        std::fs::write(spec_dir.join("spec.toml"), spec_toml).unwrap();
        std::fs::write(spec_dir.join("gates.toml"), gates_toml).unwrap();

        let (exit_code, report) = coverage_report_from("zero-cov", false, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        let report = report.expect("should have a coverage report");
        assert_eq!(report.total_requirements, 1);
        assert_eq!(report.coverage_pct, 0.0);
        assert!(report.covered.is_empty());
        assert_eq!(report.uncovered, vec!["REQ-ZERO-001"]);
        assert!(report.orphaned.is_empty());
    }

    #[test]
    fn test_spec_coverage_json_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_coverage_dir_spec(specs_dir, "auth-flow");

        let (exit_code, report) = coverage_report_from("auth-flow", true, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        let report = report.expect("should have a coverage report");
        // Verify JSON round-trip
        let json_str = serde_json::to_string_pretty(&report).unwrap();
        let parsed: CoverageReport = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.spec, report.spec);
        assert_eq!(parsed.total_requirements, report.total_requirements);
        assert_eq!(parsed.covered, report.covered);
        assert_eq!(parsed.uncovered, report.uncovered);
        assert_eq!(parsed.orphaned, report.orphaned);
    }

    // --- Promote tests ---

    /// Testable core of handle_spec_promote that takes explicit specs_dir.
    fn promote_and_result(
        name: &str,
        to: Option<&str>,
        specs_dir: &std::path::Path,
    ) -> anyhow::Result<(i32, SpecStatus, SpecStatus)> {
        let target = match to {
            Some(s) => match parse_spec_status(s) {
                Some(status) => Some(status),
                None => anyhow::bail!(
                    "invalid status '{}'. Valid values: draft, proposed, planned, in-progress, verified, deprecated",
                    s
                ),
            },
            None => None,
        };

        let (old, new) = assay_core::spec::promote::promote_spec(specs_dir, name, target)?;
        Ok((0, old, new))
    }

    /// Create a directory spec with spec.toml for promote testing.
    fn write_promote_dir_spec(dir: &std::path::Path, name: &str, status: &str) {
        let spec_dir = dir.join(name);
        std::fs::create_dir_all(&spec_dir).unwrap();
        let spec_toml = format!(
            r#"name = "{name}"
status = "{status}"
version = "0.1"

[overview]
description = "Test spec for promote"
functions = []

[[requirements]]
id = "REQ-TEST-001"
title = "Test requirement"
statement = "The system shall do something"
"#
        );
        let gates_toml = format!(
            r#"name = "{name}"

[[criteria]]
name = "check"
description = "Basic check"
cmd = "echo ok"
"#
        );
        std::fs::write(spec_dir.join("spec.toml"), spec_toml).unwrap();
        std::fs::write(spec_dir.join("gates.toml"), gates_toml).unwrap();
    }

    #[test]
    fn test_spec_promote_advance() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_promote_dir_spec(specs_dir, "auth-flow", "draft");

        let (exit_code, old, new) = promote_and_result("auth-flow", None, specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        assert_eq!(old, SpecStatus::Draft);
        assert_eq!(new, SpecStatus::Proposed);
    }

    #[test]
    fn test_spec_promote_to_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_promote_dir_spec(specs_dir, "auth-flow", "draft");

        let (exit_code, old, new) =
            promote_and_result("auth-flow", Some("planned"), specs_dir).unwrap();
        assert_eq!(exit_code, 0);
        assert_eq!(old, SpecStatus::Draft);
        assert_eq!(new, SpecStatus::Planned);
    }

    #[test]
    fn test_spec_promote_invalid_to() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_promote_dir_spec(specs_dir, "auth-flow", "draft");

        let err = promote_and_result("auth-flow", Some("bogus"), specs_dir).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid status") && msg.contains("Valid values"),
            "Expected invalid status error with valid values list, got: {msg}"
        );
    }

    #[test]
    fn test_spec_promote_unsupported_spec() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        write_gates_only_dir_spec(specs_dir, "gates-only");

        let err = promote_and_result("gates-only", None, specs_dir).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("spec.toml"),
            "Expected spec.toml error, got: {msg}"
        );
    }

    // ── Review tests ─────────────────────────────────────────────────

    /// Testable core for `handle_spec_review` that bypasses project_root discovery.
    fn review_and_result(
        name: &str,
        specs_dir: &std::path::Path,
        assay_dir: &std::path::Path,
    ) -> anyhow::Result<(i32, assay_types::ReviewReport)> {
        let entry = assay_core::spec::load_spec_entry_with_diagnostics(name, specs_dir)?;

        let report = match &entry {
            SpecEntry::Legacy { spec, .. } => {
                let gates_spec = assay_types::GatesSpec {
                    name: spec.name.clone(),
                    description: spec.description.clone(),
                    gate: None,
                    depends: vec![],
                    milestone: None,
                    order: None,
                    criteria: spec.criteria.clone(),
                };
                assay_core::review::run_structural_review(name, &gates_spec, None)
            }
            SpecEntry::Directory {
                gates, spec_path, ..
            } => {
                let feature = spec_path
                    .as_ref()
                    .and_then(|p| assay_core::spec::load_feature_spec(p).ok());
                assay_core::review::run_structural_review(name, gates, feature.as_ref())
            }
        };

        let _path = assay_core::review::save_review(assay_dir, &report)?;
        let exit_code = if report.failed > 0 { 1 } else { 0 };
        Ok((exit_code, report))
    }

    fn write_dir_spec_with_feature(
        dir: &std::path::Path,
        name: &str,
        reqs: &[&str],
        criteria_reqs: &[(&str, &[&str])],
    ) {
        let spec_dir = dir.join(name);
        std::fs::create_dir_all(&spec_dir).unwrap();

        // Write gates.toml.
        let mut gates = format!("name = \"{name}\"\n\n");
        for (cname, creqs) in criteria_reqs {
            gates.push_str(&format!(
                "[[criteria]]\nname = \"{cname}\"\ndescription = \"test\"\ncmd = \"echo ok\"\n"
            ));
            if !creqs.is_empty() {
                let reqs_str: Vec<String> = creqs.iter().map(|r| format!("\"{r}\"")).collect();
                gates.push_str(&format!("requirements = [{}]\n", reqs_str.join(", ")));
            }
            gates.push('\n');
        }
        std::fs::write(spec_dir.join("gates.toml"), gates).unwrap();

        // Write spec.toml with requirements.
        let mut spec = format!("name = \"{name}\"\nstatus = \"draft\"\nversion = \"0.1\"\n\n");
        for req_id in reqs {
            spec.push_str(&format!(
                "[[requirements]]\nid = \"{req_id}\"\ntitle = \"Test\"\nstatement = \"Must work\"\nobligation = \"shall\"\npriority = \"must\"\nverification = \"test\"\nstatus = \"draft\"\n\n[[requirements.acceptance_criteria]]\ncriterion = \"Given X, when Y, then Z\"\n\n"
            ));
        }
        std::fs::write(spec_dir.join("spec.toml"), spec).unwrap();
    }

    #[test]
    fn test_spec_review_all_pass_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        write_dir_spec_with_feature(
            specs_dir,
            "review-pass",
            &["REQ-AUTH-001"],
            &[("c1", &["REQ-AUTH-001"])],
        );

        let (exit_code, report) = review_and_result("review-pass", specs_dir, &assay_dir).unwrap();
        assert_eq!(exit_code, 0, "all-pass review should exit 0");
        assert_eq!(report.failed, 0);
        assert_eq!(report.checks.len(), 6);
    }

    #[test]
    fn test_spec_review_failure_returns_one() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        // REQ-AUTH-002 is uncovered → req-coverage fails.
        write_dir_spec_with_feature(
            specs_dir,
            "review-fail",
            &["REQ-AUTH-001", "REQ-AUTH-002"],
            &[("c1", &["REQ-AUTH-001"])],
        );

        let (exit_code, report) = review_and_result("review-fail", specs_dir, &assay_dir).unwrap();
        assert_eq!(exit_code, 1, "review with failures should exit 1");
        assert!(report.failed > 0);
    }

    #[test]
    fn test_spec_review_json_output_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        write_dir_spec_with_feature(
            specs_dir,
            "review-json",
            &["REQ-AUTH-001"],
            &[("c1", &["REQ-AUTH-001"])],
        );

        let (_exit_code, report) = review_and_result("review-json", specs_dir, &assay_dir).unwrap();

        let json_str =
            serde_json::to_string_pretty(&report).expect("ReviewReport should serialize");
        let deserialized: assay_types::ReviewReport =
            serde_json::from_str(&json_str).expect("ReviewReport JSON should round-trip");
        assert_eq!(deserialized.spec, "review-json");
        assert_eq!(deserialized.checks.len(), report.checks.len());
    }

    #[test]
    fn test_spec_review_list_returns_saved_reviews() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        write_dir_spec_with_feature(
            specs_dir,
            "review-list",
            &["REQ-AUTH-001"],
            &[("c1", &["REQ-AUTH-001"])],
        );

        // Save two reviews.
        review_and_result("review-list", specs_dir, &assay_dir).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        review_and_result("review-list", specs_dir, &assay_dir).unwrap();

        let reviews = assay_core::review::list_reviews(&assay_dir, "review-list").unwrap();
        assert_eq!(reviews.len(), 2);
        // Most recent first.
        assert!(
            reviews[0].timestamp > reviews[1].timestamp,
            "expected most-recent review first"
        );
    }

    #[test]
    fn test_spec_review_no_spec_toml_skips_gracefully() {
        let tmp = tempfile::tempdir().unwrap();
        let specs_dir = tmp.path();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        // Directory spec with gates only, no spec.toml.
        write_gates_only_dir_spec(specs_dir, "no-feature");

        let (_exit_code, report) = review_and_result("no-feature", specs_dir, &assay_dir).unwrap();
        // 5 checks skip (those needing FeatureSpec), 1 runs (criterion-traceability).
        assert_eq!(report.skipped, 5);
        // criterion-traceability runs; exit code depends on whether it passes.
        // The gates-only spec has 1 criterion with no requirements → 100% → fails.
        assert_eq!(report.failed, 1);
        assert_eq!(report.checks.len(), 6);
    }

    #[test]
    fn test_spec_review_auto_promotion_session_data() {
        // Verify that a work session with auto_promoted=true and promoted_to=Verified
        // can be loaded for spec review display (S04 data path).
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        let session = assay_core::work_session::start_session(
            &assay_dir,
            "auto-promo",
            std::path::PathBuf::from("/tmp/wt"),
            "claude",
            None,
        )
        .unwrap();

        let session_id = session.id.clone();

        // Set auto-promote fields.
        assay_core::work_session::with_session(&assay_dir, &session_id, |s| {
            s.auto_promoted = true;
            s.promoted_to = Some(assay_types::feature_spec::SpecStatus::Verified);
            Ok(())
        })
        .unwrap();

        // Verify the session can be found by listing and filtering.
        let ids = assay_core::work_session::list_sessions(&assay_dir).unwrap();
        assert!(!ids.is_empty());

        let loaded = assay_core::work_session::load_session(&assay_dir, &session_id).unwrap();
        assert!(loaded.auto_promoted);
        assert_eq!(
            loaded.promoted_to,
            Some(assay_types::feature_spec::SpecStatus::Verified)
        );
        assert_eq!(loaded.spec_name, "auto-promo");
    }

    #[test]
    fn test_spec_review_checkpoint_diagnostic_data() {
        // Verify that a GateDiagnostic with checkpoint metadata can be
        // saved and loaded for spec review display (S04 data path).
        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        let diag = assay_types::GateDiagnostic {
            spec: "ckpt-spec".to_string(),
            run_id: "test-run-001".to_string(),
            timestamp: chrono::Utc::now(),
            failed_criteria: vec![assay_types::FailedCriterionSummary {
                criterion_name: "too-many-errors".to_string(),
                command: Some("check errors".to_string()),
                exit_code: Some(1),
                stderr_snippet: "threshold exceeded".to_string(),
            }],
            passed: 2,
            failed: 1,
            checkpoint_index: Some(0),
            session_phase: assay_types::review::SessionPhase::AtToolCall { n: 5 },
        };

        let path =
            assay_core::review::save_gate_diagnostic(&assay_dir, "ckpt-spec", &diag).unwrap();
        assert!(path.exists());

        let diagnostics =
            assay_core::review::list_gate_diagnostics(&assay_dir, "ckpt-spec").unwrap();
        assert_eq!(diagnostics.len(), 1);
        let loaded = &diagnostics[0];
        assert_eq!(loaded.checkpoint_index, Some(0));
        assert!(matches!(
            loaded.session_phase,
            assay_types::review::SessionPhase::AtToolCall { n: 5 }
        ));
        assert_eq!(loaded.failed_criteria.len(), 1);
        assert_eq!(loaded.failed_criteria[0].criterion_name, "too-many-errors");
    }
}
