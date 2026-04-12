//! Shared dialoguer prompt helpers for the gate and criteria wizards.
//!
//! These helpers are `pub(crate)` — Plan 04 (`criteria new`) imports them
//! directly from `crate::commands::wizard_helpers` without needing changes.

use anyhow::{Context, Result};
use assay_types::CriterionInput;

/// Prompt for a slug via `Input::validate_with`, rejecting invalid values inline.
///
/// If `initial` is `Some(text)`, pre-fills the prompt with that value
/// (used in edit mode). The user cannot advance past a bad slug.
pub(crate) fn prompt_slug(prompt: &str, initial: Option<&str>) -> Result<String> {
    let mut builder = dialoguer::Input::<String>::new().with_prompt(prompt);
    if let Some(init) = initial {
        builder = builder.with_initial_text(init);
    }
    builder
        .validate_with(|input: &String| -> std::result::Result<(), String> {
            assay_core::spec::compose::validate_slug(input.as_str()).map_err(|e| e.to_string())
        })
        .interact_text()
        .context("reading slug")
}

/// Inline add-another criterion loop.
///
/// If `existing` is non-empty (edit mode), the loop starts with those
/// criteria pre-populated and prompts the user to add more. In create mode
/// pass an empty slice.
///
/// This helper is shared by both the gate wizard (Plan 03) and the criteria
/// new command (Plan 04).
pub(crate) fn prompt_criteria_loop(existing: &[CriterionInput]) -> Result<Vec<CriterionInput>> {
    let mut criteria = existing.to_vec();
    loop {
        let add = dialoguer::Confirm::new()
            .with_prompt("  Add a criterion?")
            .default(criteria.is_empty())
            .interact()
            .context("confirm add criterion")?;
        if !add {
            break;
        }
        let name: String = dialoguer::Input::new()
            .with_prompt("    Criterion name")
            .interact_text()
            .context("reading criterion name")?;
        let description: String = dialoguer::Input::new()
            .with_prompt("    Description")
            .allow_empty(true)
            .interact_text()
            .context("reading criterion description")?;
        let cmd_raw: String = dialoguer::Input::new()
            .with_prompt("    Command (Enter to skip)")
            .allow_empty(true)
            .interact_text()
            .context("reading criterion command")?;
        let cmd = if cmd_raw.trim().is_empty() {
            None
        } else {
            Some(cmd_raw.trim().to_string())
        };
        criteria.push(CriterionInput {
            name,
            description,
            cmd,
        });
    }
    Ok(criteria)
}

/// Generic `Select` wrapper that returns the chosen item index.
pub(crate) fn select_from_list(
    prompt: &str,
    items: &[String],
    default_idx: usize,
) -> Result<usize> {
    dialoguer::Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(default_idx)
        .interact()
        .context("selecting from list")
}

/// Generic `MultiSelect` wrapper that returns the chosen item indices.
pub(crate) fn multi_select_from_list(
    prompt: &str,
    items: &[String],
    preselected: &[usize],
) -> Result<Vec<usize>> {
    let defaults: Vec<bool> = (0..items.len()).map(|i| preselected.contains(&i)).collect();
    dialoguer::MultiSelect::new()
        .with_prompt(prompt)
        .items(items)
        .defaults(&defaults)
        .interact()
        .context("selecting multiple items")
}

#[cfg(test)]
mod tests {
    // These helpers require a TTY for end-to-end interaction. Unit tests here
    // focus on type signatures and non-interactive branches. Visual flow is
    // on the manual-only verification list in VALIDATION.md.

    use super::*;

    /// Verify that `prompt_criteria_loop` has the expected public signature
    /// (callable with a slice and returning `Result<Vec<CriterionInput>>`).
    /// In non-TTY test environments the dialoguer prompt is not invoked — this
    /// test simply confirms the function compiles and is accessible.
    #[test]
    fn prompt_criteria_loop_stub_exists() {
        // We cannot call the function in a non-TTY context because dialoguer
        // would panic. The test asserts the type signature compiles correctly
        // by constructing a call-site with the expected types.
        let _: fn(&[CriterionInput]) -> Result<Vec<CriterionInput>> = prompt_criteria_loop;
    }
}
