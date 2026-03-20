//! Gate-gated PR creation workflow.
//!
//! Provides [`pr_check_milestone_gates`] and [`pr_create_if_gates_pass`] for
//! evaluating all milestone chunk gates before opening a GitHub PR via `gh`.
//! PR number and URL are persisted to the milestone TOML after creation.
//!
//! All errors use [`AssayError::Io`] — no new variants (consistent with D065,
//! D008, S01/S02 patterns).

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use chrono::Utc;

use assay_types::MilestoneStatus;

use crate::error::{AssayError, Result};
use crate::gate::evaluate_all_gates;
use crate::milestone::{milestone_load, milestone_phase_transition, milestone_save};
use crate::spec::{SpecEntry, load_spec_entry_with_diagnostics};

// ── Public result types ───────────────────────────────────────────────────────

/// A chunk whose required gates did not all pass.
#[derive(Debug)]
pub struct ChunkGateFailure {
    /// Slug of the chunk that has failing required gates.
    pub chunk_slug: String,
    /// Number of required criteria that failed for this chunk.
    pub required_failed: usize,
}

/// Result of a successful PR creation via `gh`.
#[derive(Debug)]
pub struct PrCreateResult {
    /// The GitHub PR number assigned to the new pull request.
    pub pr_number: u64,
    /// The HTML URL of the new pull request.
    pub pr_url: String,
}

// ── pr_check_milestone_gates ──────────────────────────────────────────────────

/// Evaluate gates for every chunk in the milestone and collect failures.
///
/// Chunks are processed in ascending [`assay_types::ChunkRef::order`] order.
/// For each chunk, the directory spec is loaded from `specs_dir` and all gates
/// are evaluated synchronously.  Chunks with one or more failing *required*
/// criteria produce a [`ChunkGateFailure`] entry in the returned `Vec`.
///
/// Returns `Ok(vec![])` when every chunk passes all required gates.
/// Returns `Ok(failures)` (non-empty) when at least one chunk has failures.
/// Returns `Err(AssayError::Io)` only for I/O or spec-loading errors.
pub fn pr_check_milestone_gates(
    assay_dir: &Path,
    specs_dir: &Path,
    working_dir: &Path,
    milestone_slug: &str,
) -> Result<Vec<ChunkGateFailure>> {
    let milestone = milestone_load(assay_dir, milestone_slug)?;

    let mut ordered_chunks: Vec<_> = milestone.chunks.iter().collect();
    ordered_chunks.sort_by_key(|c| c.order);

    let mut failures = Vec::new();

    for chunk in ordered_chunks {
        let spec_entry = load_spec_entry_with_diagnostics(&chunk.slug, specs_dir)?;

        let gates = match spec_entry {
            SpecEntry::Directory { gates, .. } => gates,
            SpecEntry::Legacy { slug, .. } => {
                return Err(AssayError::Io {
                    operation: "pr_check_milestone_gates".to_string(),
                    path: specs_dir.join(format!("{slug}.toml")),
                    source: io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "chunk '{slug}' is a legacy spec; directory specs required for \
                             milestone chunks (create specs/{slug}/gates.toml instead)"
                        ),
                    ),
                });
            }
        };

        let summary = evaluate_all_gates(&gates, working_dir, None, None);

        if summary.enforcement.required_failed > 0 {
            failures.push(ChunkGateFailure {
                chunk_slug: chunk.slug.clone(),
                required_failed: summary.enforcement.required_failed,
            });
        }
    }

    Ok(failures)
}

// ── pr_create_if_gates_pass ───────────────────────────────────────────────────

/// Create a GitHub PR via `gh` only when all milestone chunk gates pass.
///
/// ## Algorithm
///
/// 1. Load the milestone; return `AssayError::Io` if `pr_number` is already
///    set (idempotency guard — prevents duplicate PRs).
/// 2. Call [`pr_check_milestone_gates`]; if failures are non-empty, format a
///    structured list and return `AssayError::Io`.
/// 3. Determine the base branch from `milestone.pr_base` (default: `"main"`).
/// 4. Run `gh pr create --title <title> --base <base> --json number,url
///    [--body <body>]` in `working_dir`.
/// 5. Parse the JSON response and persist `pr_number` / `pr_url` to the
///    milestone TOML.
/// 6. If the milestone was in `Verify` status, transition it to `Complete`.
///
/// # Errors
///
/// Returns [`AssayError::Io`] for every failure path (consistent with D065):
/// - `"PR already created: #N — url"` — PR already recorded on the milestone
/// - Gate-failure list — one or more chunks have failing required criteria
/// - `"gh CLI not found — install from https://cli.github.com"` — `gh` binary
///   not found in `PATH`
/// - `gh` non-zero exit — stderr is forwarded as the error message
/// - JSON parse failure — `gh` stdout could not be decoded
pub fn pr_create_if_gates_pass(
    assay_dir: &Path,
    specs_dir: &Path,
    working_dir: &Path,
    milestone_slug: &str,
    title: &str,
    body: Option<&str>,
) -> Result<PrCreateResult> {
    // ── Step 1: Load milestone + idempotency guard ────────────────────────
    let milestone_check = milestone_load(assay_dir, milestone_slug)?;

    if let Some(pr_number) = milestone_check.pr_number {
        let pr_url = milestone_check
            .pr_url
            .as_deref()
            .unwrap_or("<no url recorded>");
        return Err(AssayError::Io {
            operation: "pr_create_if_gates_pass".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("PR already created: #{pr_number} — {pr_url}"),
            ),
        });
    }

    // ── Step 2: Pre-flight — verify gh is available ───────────────────────
    // Check before gate evaluation so "gh not found" is always actionable.
    match Command::new("gh")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(AssayError::Io {
                operation: "pr_create_if_gates_pass".to_string(),
                path: std::path::PathBuf::from(milestone_slug),
                source: io::Error::new(
                    io::ErrorKind::NotFound,
                    "gh CLI not found — install from https://cli.github.com",
                ),
            });
        }
        Err(e) => {
            return Err(AssayError::Io {
                operation: "pr_create_if_gates_pass".to_string(),
                path: std::path::PathBuf::from(milestone_slug),
                source: io::Error::new(e.kind(), format!("failed to spawn gh: {e}")),
            });
        }
        Ok(mut child) => {
            // Reap the spawned process — we only needed the spawn to succeed.
            let _ = child.wait();
        }
    }

    // ── Step 3: Check gates ───────────────────────────────────────────────
    let failures = pr_check_milestone_gates(assay_dir, specs_dir, working_dir, milestone_slug)?;

    if !failures.is_empty() {
        let detail = failures
            .iter()
            .map(|f| {
                format!(
                    "  - {}: {} required criteria failed",
                    f.chunk_slug, f.required_failed
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(AssayError::Io {
            operation: "pr_create_if_gates_pass".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::other(format!(
                "gates failed for {} chunk(s):\n{}",
                failures.len(),
                detail
            )),
        });
    }

    // ── Step 4: Determine base branch ─────────────────────────────────────
    let base_branch = milestone_check
        .pr_base
        .as_deref()
        .unwrap_or("main")
        .to_string();

    // ── Step 5: Build gh args ─────────────────────────────────────────────
    let mut args: Vec<String> = vec![
        "pr".to_string(),
        "create".to_string(),
        "--title".to_string(),
        title.to_string(),
        "--base".to_string(),
        base_branch,
        "--json".to_string(),
        "number,url".to_string(),
    ];

    if let Some(b) = body {
        args.push("--body".to_string());
        args.push(b.to_string());
    }

    // ── Step 6: Run gh ────────────────────────────────────────────────────
    let output = Command::new("gh")
        .args(&args)
        .current_dir(working_dir)
        .output()
        .map_err(|e| AssayError::Io {
            operation: "pr_create_if_gates_pass".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::new(e.kind(), format!("failed to spawn gh: {e}")),
        })?;

    // ── Step 7: Check exit status ─────────────────────────────────────────
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout_str.is_empty() {
            format!("(stderr empty; stdout: {stdout_str})")
        } else {
            format!("gh exited with status {}", output.status)
        };
        return Err(AssayError::Io {
            operation: "gh pr create".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::other(format!("exit status {}: {detail}", output.status)),
        });
    }

    // ── Step 8: Parse JSON response ───────────────────────────────────────
    let (pr_number, pr_url) = parse_gh_output(&output.stdout, milestone_slug)?;

    // ── Steps 9-11: Reload, mutate, and save milestone ───────────────────
    // If any step fails after the PR is created, include the PR details so
    // the user can recover by manually updating the milestone TOML.
    let save_result = (|| -> Result<()> {
        // ── Step 9: Reload and mutate milestone ───────────────────────────
        let mut milestone = milestone_load(assay_dir, milestone_slug)?;
        milestone.pr_number = Some(pr_number);
        milestone.pr_url = Some(pr_url.clone());
        milestone.updated_at = Utc::now();

        // ── Step 10: Transition Verify → Complete if applicable ───────────
        if milestone.status == MilestoneStatus::Verify {
            milestone_phase_transition(&mut milestone, MilestoneStatus::Complete)?;
        }

        // ── Step 11: Save ─────────────────────────────────────────────────
        milestone_save(assay_dir, &milestone)
    })();

    if let Err(e) = save_result {
        return Err(AssayError::Io {
            operation: "pr_create_if_gates_pass".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::other(format!(
                "PR #{pr_number} was created at {pr_url} but milestone TOML update failed: {e}. \
                 Add pr_number = {pr_number} and pr_url = \"{pr_url}\" to the milestone TOML manually."
            )),
        });
    }

    Ok(PrCreateResult { pr_number, pr_url })
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Parse the JSON output from `gh pr create --json number,url`.
///
/// Returns `Err(AssayError::Io)` for all missing or invalid fields — no
/// silent defaults.  Both `number` (must be present and > 0) and `url`
/// (must be a non-empty string) are required; absence of either is a hard
/// error.
fn parse_gh_output(stdout: &[u8], milestone_slug: &str) -> Result<(u64, String)> {
    let parsed: serde_json::Value = serde_json::from_slice(stdout).map_err(|e| AssayError::Io {
        operation: "pr_create_if_gates_pass".to_string(),
        path: std::path::PathBuf::from(milestone_slug),
        source: io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to parse gh JSON output: {e}"),
        ),
    })?;

    let pr_number = parsed["number"]
        .as_u64()
        .filter(|&n| n > 0)
        .ok_or_else(|| AssayError::Io {
            operation: "pr_create_if_gates_pass".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::new(
                io::ErrorKind::InvalidData,
                "gh JSON response missing or invalid 'number' field",
            ),
        })?;

    let pr_url = parsed["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AssayError::Io {
            operation: "pr_create_if_gates_pass".to_string(),
            path: std::path::PathBuf::from(milestone_slug),
            source: io::Error::new(
                io::ErrorKind::InvalidData,
                "gh JSON response missing or invalid 'url' field",
            ),
        })?;

    Ok((pr_number, pr_url))
}
