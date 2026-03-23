//! Slash command parsing, dispatch, overlay state, and stubs.
//!
//! `parse_slash_cmd` and `execute_slash_cmd` are fully implemented.
//! `handle_slash_event` and `draw_slash_overlay` are stubs for T02.

use std::path::Path;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;

// ── SlashCmd ──────────────────────────────────────────────────────────────────

/// Known slash commands the user can invoke from the overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCmd {
    GateCheck,
    Status,
    NextChunk,
    SpecShow,
    PrCreate,
}

/// Lookup table used for parsing and tab completion.
pub const COMMANDS: &[(&str, SlashCmd)] = &[
    ("gate-check", SlashCmd::GateCheck),
    ("next-chunk", SlashCmd::NextChunk),
    ("pr-create", SlashCmd::PrCreate),
    ("spec-show", SlashCmd::SpecShow),
    ("status", SlashCmd::Status),
];

// ── SlashState ────────────────────────────────────────────────────────────────

/// Mutable state for the slash command overlay.
#[derive(Debug, Default, Clone)]
pub struct SlashState {
    /// Current text in the input buffer (without leading `/`).
    pub input: String,
    /// Tab-completion suggestion (full command name with `/` prefix).
    pub suggestion: Option<String>,
    /// Result string from the last executed command.
    pub result: Option<String>,
    /// Error string from the last executed command.
    pub error: Option<String>,
}

// ── SlashAction ───────────────────────────────────────────────────────────────

/// Actions returned by the event handler to drive the outer app state machine.
#[derive(Debug, PartialEq, Eq)]
pub enum SlashAction {
    /// Keep the overlay open, no side-effects.
    Continue,
    /// Close the overlay without executing.
    Close,
    /// Execute the given command.
    Execute(SlashCmd),
}

// ── parse_slash_cmd ───────────────────────────────────────────────────────────

/// Parse a user-entered string into a `SlashCmd`.
///
/// Strips a leading `/` if present, trims whitespace, and matches
/// case-insensitively against known command names.
pub fn parse_slash_cmd(input: &str) -> Option<SlashCmd> {
    let trimmed = input.trim().strip_prefix('/').unwrap_or(input.trim());
    let lower = trimmed.trim().to_lowercase();
    COMMANDS
        .iter()
        .find(|(name, _)| *name == lower)
        .map(|(_, cmd)| cmd.clone())
}

// ── tab_complete ──────────────────────────────────────────────────────────────

/// Prefix-match against known commands and return a suggestion.
///
/// The `input` should be the raw text *without* the leading `/`.
/// Returns the full command name with `/` prefix if a unique or first
/// alphabetical match is found; `None` if no commands match.
pub fn tab_complete(input: &str) -> Option<String> {
    let lower = input.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    let matches: Vec<&str> = COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(&lower))
        .map(|(name, _)| *name)
        .collect();
    // COMMANDS is already sorted alphabetically, so first match = first alphabetical.
    matches.first().map(|name| format!("/{name}"))
}

// ── execute_slash_cmd ─────────────────────────────────────────────────────────

/// Dispatch a parsed command against assay-core functions.
///
/// All calls are synchronous. Errors are caught and returned as human-readable
/// strings — this function never panics.
pub fn execute_slash_cmd(cmd: SlashCmd, project_root: &Path) -> String {
    let assay_dir = project_root.join(".assay");
    let specs_dir = assay_dir.join("specs");

    match cmd {
        SlashCmd::Status => match assay_core::milestone::cycle_status(&assay_dir) {
            Ok(Some(cs)) => {
                let chunk_info = cs
                    .active_chunk_slug
                    .as_deref()
                    .unwrap_or("none (all complete)");
                format!(
                    "Milestone: {} ({})\nActive chunk: {}\nProgress: {}/{}",
                    cs.milestone_name,
                    cs.milestone_slug,
                    chunk_info,
                    cs.completed_count,
                    cs.total_count,
                )
            }
            Ok(None) => "No milestone currently in progress.".to_string(),
            Err(e) => format!("Error loading status: {e}"),
        },

        SlashCmd::NextChunk => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            let chunk_slug = match &status.active_chunk_slug {
                Some(slug) => slug.clone(),
                None => return "All chunks complete — no next chunk.".to_string(),
            };
            match assay_core::spec::load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir) {
                Ok(entry) => {
                    let spec_name = match &entry {
                        assay_core::spec::SpecEntry::Directory { gates, .. } => {
                            gates.name.clone()
                        }
                        assay_core::spec::SpecEntry::Legacy { slug, .. } => slug.clone(),
                    };
                    format!(
                        "Next chunk: {} (spec: {})\nMilestone: {} ({}/{})",
                        chunk_slug,
                        spec_name,
                        status.milestone_slug,
                        status.completed_count,
                        status.total_count,
                    )
                }
                Err(e) => format!("Next chunk: {chunk_slug}\nError loading spec: {e}"),
            }
        }

        SlashCmd::GateCheck => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            match assay_core::pr::pr_check_milestone_gates(
                &assay_dir,
                &specs_dir,
                project_root,
                &status.milestone_slug,
            ) {
                Ok(failures) if failures.is_empty() => {
                    "All gates pass for current milestone.".to_string()
                }
                Ok(failures) => {
                    let mut out = String::from("Gate failures:\n");
                    for f in &failures {
                        out.push_str(&format!(
                            "  {} — {} required criteria failed\n",
                            f.chunk_slug, f.required_failed
                        ));
                    }
                    out
                }
                Err(e) => format!("Error evaluating gates: {e}"),
            }
        }

        SlashCmd::SpecShow => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            let chunk_slug = match &status.active_chunk_slug {
                Some(slug) => slug.clone(),
                None => return "All chunks complete — no active spec to show.".to_string(),
            };
            match assay_core::spec::load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir) {
                Ok(assay_core::spec::SpecEntry::Directory { gates, .. }) => {
                    let mut out = format!("Spec: {}\nCriteria:\n", gates.name);
                    for c in &gates.criteria {
                        out.push_str(&format!("  • {} — {}\n", c.name, c.description));
                    }
                    out
                }
                Ok(assay_core::spec::SpecEntry::Legacy { slug, .. }) => {
                    format!("Legacy spec: {slug} (criteria not available in directory format)")
                }
                Err(e) => format!("Error loading spec for '{chunk_slug}': {e}"),
            }
        }

        SlashCmd::PrCreate => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            match assay_core::pr::pr_check_milestone_gates(
                &assay_dir,
                &specs_dir,
                project_root,
                &status.milestone_slug,
            ) {
                Ok(failures) if failures.is_empty() => {
                    format!(
                        "All gates pass for '{}'. Ready to create PR.",
                        status.milestone_slug
                    )
                }
                Ok(failures) => {
                    let slugs: Vec<&str> =
                        failures.iter().map(|f| f.chunk_slug.as_str()).collect();
                    format!(
                        "Cannot create PR — gate failures in: {}",
                        slugs.join(", ")
                    )
                }
                Err(e) => format!("Error checking gates for PR: {e}"),
            }
        }
    }
}

// ── handle_slash_event (stub) ─────────────────────────────────────────────────

/// Handle a key event within the slash overlay.
///
/// **Stub**: always returns `SlashAction::Continue`. Real implementation in T02.
#[allow(unused_variables)]
pub fn handle_slash_event(state: &mut SlashState, key: KeyEvent) -> SlashAction {
    SlashAction::Continue
}

// ── draw_slash_overlay (stub) ─────────────────────────────────────────────────

/// Draw the slash command overlay at the bottom of the screen.
///
/// **Stub**: no-op. Real implementation in T02.
#[allow(unused_variables)]
pub fn draw_slash_overlay(frame: &mut ratatui::Frame, area: Rect, state: &SlashState) {}
