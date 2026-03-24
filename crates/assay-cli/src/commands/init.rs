use super::{COLUMN_GAP, assay_dir, project_root};

/// Display project status for bare `assay` invocation inside an initialized project.
///
/// Shows the binary version, project name, and a spec inventory with criteria counts.
/// Returns `Err` on config load failure so the caller controls the exit.
///
/// Unlike `handle_spec_list`, scan errors are soft warnings here — bare invocation
/// should degrade gracefully since the user didn't explicitly ask for spec data.
pub(crate) fn show_status(root: &std::path::Path) -> anyhow::Result<()> {
    use assay_core::spec::SpecEntry;

    let config = assay_core::config::load(root)?;

    println!(
        "assay {} -- {}",
        env!("CARGO_PKG_VERSION"),
        config.project_name
    );
    println!();

    let specs_dir = assay_dir(root).join(&config.specs_dir);
    let result = match assay_core::spec::scan(&specs_dir) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "Could not scan specs");
            return Ok(());
        }
    };

    for err in &result.errors {
        tracing::warn!("{err}");
    }

    if result.entries.is_empty() {
        println!("No specs found in {}", config.specs_dir);
        return Ok(());
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
                let total = spec.criteria.len();
                let executable = spec
                    .criteria
                    .iter()
                    .filter(|c| c.cmd.is_some() || c.path.is_some())
                    .count();
                println!(
                    "  {:<width$}{gap}{total} criteria ({executable} executable)",
                    slug,
                    width = name_width,
                    gap = COLUMN_GAP,
                );
            }
            SpecEntry::Directory { slug, gates, .. } => {
                let total = gates.criteria.len();
                let executable = gates
                    .criteria
                    .iter()
                    .filter(|c| c.cmd.is_some() || c.path.is_some())
                    .count();
                println!(
                    "  {:<width$}{gap}{} {total} criteria ({executable} executable)",
                    slug,
                    assay_types::DIRECTORY_SPEC_INDICATOR,
                    width = name_width,
                    gap = COLUMN_GAP,
                );
            }
        }
    }

    Ok(())
}

/// Handle `assay init [--name <name>]`.
pub(crate) fn handle_init(name: Option<String>) -> anyhow::Result<i32> {
    let root = project_root()?;
    let options = assay_core::init::InitOptions { name };
    let result = assay_core::init::init(&root, &options)?;
    println!("  Created assay project `{}`", result.project_name);
    for path in &result.created_files {
        let display = path.strip_prefix(&root).unwrap_or(path);
        println!("    created {}", display.display());
    }
    Ok(0)
}
