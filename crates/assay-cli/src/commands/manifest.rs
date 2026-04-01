//! Manifest generation subcommands for the `assay manifest` CLI group.

use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::Subcommand;

use assay_core::manifest_gen::{ManifestGenConfig, ManifestSource};

use super::{assay_dir, project_root};

#[derive(Subcommand)]
pub(crate) enum ManifestCommand {
    /// Generate a run manifest from a milestone or all specs
    #[command(after_long_help = "\
Examples:
  Generate from a milestone:
    assay manifest generate --from-milestone my-milestone

  Generate from all specs:
    assay manifest generate --from-specs

  Write to a custom path:
    assay manifest generate --from-milestone my-milestone --output run.toml")]
    Generate {
        /// Generate sessions from a named milestone's chunks
        #[arg(long)]
        from_milestone: Option<String>,

        /// Generate sessions from all specs in .assay/specs/ (fully parallel)
        #[arg(long)]
        from_specs: bool,

        /// Output file path (default: manifest.toml)
        #[arg(long, short, default_value = "manifest.toml")]
        output: String,
    },
}

/// Handle manifest subcommands.
pub(crate) fn handle(command: ManifestCommand) -> anyhow::Result<i32> {
    match command {
        ManifestCommand::Generate {
            from_milestone,
            from_specs,
            output,
        } => manifest_generate_cmd(from_milestone, from_specs, output),
    }
}

/// Handle `assay manifest generate`.
fn manifest_generate_cmd(
    from_milestone: Option<String>,
    from_specs: bool,
    output: String,
) -> anyhow::Result<i32> {
    // Validate mutually exclusive flags.
    let source = match (from_milestone, from_specs) {
        (Some(slug), false) => ManifestSource::Milestone(slug),
        (None, true) => ManifestSource::AllSpecs,
        (Some(_), true) => {
            bail!("cannot use both --from-milestone and --from-specs; choose one");
        }
        (None, false) => {
            bail!("specify either --from-milestone <slug> or --from-specs");
        }
    };

    let root = project_root()?;
    let dir = assay_dir(&root);
    let output_path = PathBuf::from(&output);

    let config = ManifestGenConfig {
        assay_dir: dir.clone(),
    };

    let manifest = assay_core::manifest_gen::generate_manifest(source, &config)
        .context("manifest generation failed")?;

    let session_count = manifest.sessions.len();

    assay_core::manifest_gen::write_manifest(&manifest, &output_path)
        .context("writing manifest failed")?;

    println!(
        "Written {} ({} sessions)",
        output_path.display(),
        session_count
    );

    Ok(0)
}
