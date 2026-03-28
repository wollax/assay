//! `assay plan` — guided authoring wizard for creating milestones and chunk specs.
//!
//! This command is TTY-only. In non-interactive environments (CI, piped scripts,
//! MCP tool calls) it exits with code 1 and points callers at the `milestone_create`
//! MCP tool instead.

use std::io::IsTerminal as _;

use anyhow::Context as _;
use assay_core::wizard::{
    CriterionInput, WizardChunkInput, WizardInputs, create_from_inputs, slugify,
};

use super::{assay_dir, project_root};

/// Entry point for `assay plan`.
///
/// Returns `Ok(1)` when stdin is not a TTY (non-interactive), so the caller can
/// surface a helpful message without panicking.
pub(crate) fn handle() -> anyhow::Result<i32> {
    // ── TTY guard ────────────────────────────────────────────────────────────
    if !std::io::stdin().is_terminal() {
        tracing::error!(
            "assay plan requires an interactive terminal. \
             For non-interactive authoring, use the milestone_create MCP tool."
        );
        return Ok(1);
    }

    // ── Milestone name ───────────────────────────────────────────────────────
    let milestone_name: String = dialoguer::Input::new()
        .with_prompt("Milestone name")
        .interact_text()?;

    let milestone_slug = slugify(&milestone_name);

    // ── Optional description ─────────────────────────────────────────────────
    let has_description: bool = dialoguer::Confirm::new()
        .with_prompt("Add a description?")
        .default(false)
        .interact()?;

    let description: Option<String> = if has_description {
        let desc: String = dialoguer::Input::new()
            .with_prompt("Description")
            .allow_empty(true)
            .interact_text()?;
        if desc.is_empty() { None } else { Some(desc) }
    } else {
        None
    };

    // ── Chunk count ──────────────────────────────────────────────────────────
    let chunk_index = dialoguer::Select::new()
        .with_prompt("Number of chunks (1-7)")
        .items(["1", "2", "3", "4", "5", "6", "7"])
        .default(1) // 0-based → default 2 chunks
        .interact()?;
    let chunk_count = chunk_index + 1;

    // ── Per-chunk inputs ─────────────────────────────────────────────────────
    let mut chunks: Vec<WizardChunkInput> = Vec::with_capacity(chunk_count);

    for i in 1..=chunk_count {
        println!("\n  Chunk {i} of {chunk_count}");

        let chunk_name: String = dialoguer::Input::new()
            .with_prompt("  Chunk name")
            .interact_text()?;

        let chunk_slug = slugify(&chunk_name);

        // Collect criteria for this chunk.
        let mut criteria: Vec<CriterionInput> = Vec::new();
        loop {
            let add_more = dialoguer::Confirm::new()
                .with_prompt("  Add a criterion?")
                .default(criteria.is_empty()) // default yes on first prompt
                .interact()?;

            if !add_more {
                break;
            }

            let criterion_name: String = dialoguer::Input::new()
                .with_prompt("    Criterion name")
                .interact_text()?;

            let cmd_raw: String = dialoguer::Input::new()
                .with_prompt("    Command (Enter to skip)")
                .allow_empty(true)
                .interact_text()?;

            let cmd_trimmed = cmd_raw.trim().to_string();
            let cmd = if cmd_trimmed.is_empty() {
                None
            } else {
                Some(cmd_trimmed)
            };

            criteria.push(CriterionInput {
                name: criterion_name,
                description: String::new(),
                cmd,
            });
        }

        chunks.push(WizardChunkInput {
            slug: chunk_slug,
            name: chunk_name,
            criteria,
        });
    }

    // ── Build inputs and delegate to wizard core ─────────────────────────────
    let inputs = WizardInputs {
        slug: milestone_slug.clone(),
        name: milestone_name,
        description,
        chunks,
    };

    let root = project_root()?;
    let assay = assay_dir(&root);
    let specs = assay.join("specs");

    let result =
        create_from_inputs(&inputs, &assay, &specs).context("failed to create milestone")?;

    // ── Print summary ────────────────────────────────────────────────────────
    println!("\n  Created milestone '{milestone_slug}'");
    for path in &result.spec_paths {
        println!("    created {}", path.display());
    }
    println!("    created {}", result.milestone_path.display());
    println!(
        "\n  Milestone created as Draft. Use 'assay milestone list' to view, \
         or run 'assay gate run <chunk>' to test a chunk."
    );

    Ok(0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// In test environments stdin is not a TTY, so `handle()` must return `Ok(1)`
    /// immediately without attempting to start the interactive flow.
    #[test]
    fn plan_non_tty_returns_1() {
        let result = handle().expect("handle() should not return Err in non-TTY path");
        assert_eq!(result, 1, "non-TTY handle() must return exit code 1");
    }
}
