//! Workflow state machine for solo developer flow.
//!
//! The [`next_action`] function reads milestone state, spec status, and gate
//! history to determine the next recommended action. It is a pure function
//! (no side effects) that can be called from any surface (CLI, TUI, MCP skill).
//!
//! The [`NextAction`] enum represents the possible workflow states. Surfaces
//! act on the returned variant to guide the developer.

use std::path::Path;

use assay_types::{GateSpecStatus, MilestoneStatus};

use crate::error::Result;
use crate::milestone::cycle::active_chunk;
use crate::milestone::milestone_scan;
use crate::spec;

/// The next recommended action in the solo workflow loop.
///
/// Returned by [`next_action`]. Each variant carries enough context for
/// the caller to act without re-reading state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextAction {
    /// No active work — suggest explore or plan.
    Idle,
    /// Spec is draft — needs review/approval.
    ReviewSpec { spec_name: String },
    /// Spec is approved — ready for execution.
    Execute {
        spec_name: String,
        chunk_slug: Option<String>,
    },
    /// Gates failed — show failures, suggest fixes.
    FixAndRecheck {
        spec_name: String,
        failed_criteria: Vec<String>,
    },
    /// Gates + UAT passed, more chunks remain.
    AdvanceChunk {
        milestone_slug: String,
        next_chunk: String,
    },
    /// All chunks done — prompt for PR.
    PromptShip { milestone_slug: String },
}

/// Determine the next workflow action from current project state.
///
/// This is a pure function: it reads milestones, specs, and gate history
/// from disk and returns a recommendation. No writes or side effects.
///
/// Logic:
/// 1. If no milestones exist or none are `InProgress` → `Idle`
/// 2. Find the first `InProgress` milestone
/// 3. Find the active chunk (first incomplete)
/// 4. If no active chunk (all done) → `PromptShip`
/// 5. Load the spec for the active chunk
/// 6. Check spec status and gate history to determine action
pub fn next_action(assay_dir: &Path) -> Result<NextAction> {
    let milestones = milestone_scan(assay_dir)?;

    // Find first InProgress milestone
    let milestone = match milestones
        .into_iter()
        .find(|m| m.status == MilestoneStatus::InProgress)
    {
        Some(m) => m,
        None => return Ok(NextAction::Idle),
    };

    // Find active chunk
    let chunk = match active_chunk(&milestone) {
        Some(c) => c.clone(),
        None => {
            // All chunks complete — ready to ship
            return Ok(NextAction::PromptShip {
                milestone_slug: milestone.slug.clone(),
            });
        }
    };

    let spec_name = chunk.slug.clone();
    let specs_dir = assay_dir.join("specs");

    // Try to load the spec entry to check status
    let status = match spec::load_spec_entry(&spec_name, &specs_dir) {
        Ok(spec::SpecEntry::Directory { gates, .. }) => spec::effective_status(&gates),
        Ok(spec::SpecEntry::Legacy { .. }) => GateSpecStatus::Draft,
        Err(_) => GateSpecStatus::Draft,
    };

    // Check gate history for this spec
    let run_ids = crate::history::list(assay_dir, &spec_name).unwrap_or_default();

    if let Some(latest_run_id) = run_ids.last() {
        // We have gate history — check the latest result
        if let Ok(record) = crate::history::load(assay_dir, &spec_name, latest_run_id) {
            if record.summary.enforcement.required_failed > 0 {
                // Gates failed
                let failed: Vec<String> = record
                    .summary
                    .results
                    .iter()
                    .filter(|r| {
                        r.enforcement == assay_types::Enforcement::Required
                            && r.result.as_ref().is_some_and(|g| !g.passed)
                    })
                    .map(|r| r.criterion_name.clone())
                    .collect();
                return Ok(NextAction::FixAndRecheck {
                    spec_name,
                    failed_criteria: failed,
                });
            }

            // Gates passed — check if there are more chunks
            let next = milestone
                .chunks
                .iter()
                .find(|c| !milestone.completed_chunks.contains(&c.slug) && c.slug != spec_name);

            if let Some(next_chunk_ref) = next {
                // More chunks remain after current
                return Ok(NextAction::AdvanceChunk {
                    milestone_slug: milestone.slug.clone(),
                    next_chunk: next_chunk_ref.slug.clone(),
                });
            }

            // Last chunk, gates passed
            return Ok(NextAction::PromptShip {
                milestone_slug: milestone.slug.clone(),
            });
        }
    }

    // No gate history — check spec status
    match status {
        GateSpecStatus::Draft | GateSpecStatus::Ready => Ok(NextAction::ReviewSpec { spec_name }),
        GateSpecStatus::Approved => Ok(NextAction::Execute {
            spec_name,
            chunk_slug: Some(chunk.slug),
        }),
        GateSpecStatus::Verified => {
            // Already verified but chunk not marked complete — advance
            let next_slug = milestone
                .chunks
                .iter()
                .find(|c| !milestone.completed_chunks.contains(&c.slug) && c.slug != spec_name)
                .map(|c| c.slug.clone())
                .unwrap_or(spec_name);
            Ok(NextAction::AdvanceChunk {
                milestone_slug: milestone.slug.clone(),
                next_chunk: next_slug,
            })
        }
    }
}

// ── Branch isolation ─────────────────────────────────────────────────────────

/// Decision from the branch isolation heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolationDecision {
    /// Create a worktree/branch automatically.
    Yes,
    /// Proceed without isolation.
    No,
    /// Ask the user — they're on a protected branch.
    Ask { branch: String },
}

/// Default protected branch names.
const DEFAULT_PROTECTED: &[&str] = &["main", "master", "develop"];

/// Determine whether to isolate work in a worktree.
///
/// Uses the config's `auto_isolate` setting and protected branch list.
/// When `auto_isolate` is `Ask`, checks the current branch against the
/// protected list (config override or defaults + dynamic detection).
pub fn should_isolate(
    config: &assay_types::WorkflowConfig,
    current_branch: &str,
) -> IsolationDecision {
    match config.auto_isolate {
        assay_types::AutoIsolate::Always => IsolationDecision::Yes,
        assay_types::AutoIsolate::Never => IsolationDecision::No,
        assay_types::AutoIsolate::Ask => {
            let protected: Vec<&str> = if let Some(ref custom) = config.protected_branches {
                custom.iter().map(String::as_str).collect()
            } else {
                let list: Vec<&str> = DEFAULT_PROTECTED.to_vec();
                // Dynamic detection: try to find the default branch
                if let Ok(detected) = detect_default_branch()
                    && !list.contains(&detected.as_str())
                    && current_branch == detected
                {
                    return IsolationDecision::Ask {
                        branch: current_branch.to_string(),
                    };
                }
                list
            };

            if protected.contains(&current_branch) {
                IsolationDecision::Ask {
                    branch: current_branch.to_string(),
                }
            } else {
                IsolationDecision::No
            }
        }
    }
}

/// Detect the default branch from `git symbolic-ref refs/remotes/origin/HEAD`.
fn detect_default_branch() -> std::result::Result<String, ()> {
    let output = std::process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()
        .map_err(|_| ())?;

    if !output.status.success() {
        return Err(());
    }

    let full_ref = String::from_utf8_lossy(&output.stdout);
    let branch = full_ref
        .trim()
        .strip_prefix("refs/remotes/origin/")
        .unwrap_or(full_ref.trim());
    Ok(branch.to_string())
}

// ── Strict status ───────────────────────────────────────────────────────────

/// Check whether a spec meets the strict_status requirement for gate evaluation.
///
/// When `strict_status` is `true`, the spec must have status >= `Approved`
/// before gates can be run. Returns `Ok(())` if allowed, or an error with
/// guidance if the spec status is too low.
pub fn check_strict_status(gates: &assay_types::GatesSpec, strict: bool) -> Result<()> {
    if !strict {
        return Ok(());
    }
    let status = spec::effective_status(gates);
    match status {
        GateSpecStatus::Approved | GateSpecStatus::Verified => Ok(()),
        other => Err(crate::error::AssayError::WorkflowViolation {
            message: format!(
                "strict_status is enabled: spec status is '{}' but must be 'approved' or 'verified' before running gates. \
                 Use `spec_set_status` to advance the spec, or set `[workflow] strict_status = false` in config.",
                other
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn setup_project(dir: &Path) {
        std::fs::create_dir_all(dir.join("milestones")).unwrap();
        std::fs::create_dir_all(dir.join("specs")).unwrap();
        std::fs::create_dir_all(dir.join("results")).unwrap();
    }

    fn write_milestone(dir: &Path, milestone: &Milestone) {
        let path = dir
            .join("milestones")
            .join(format!("{}.toml", milestone.slug));
        let content = toml::to_string_pretty(milestone).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn write_gates_spec(dir: &Path, slug: &str, gates: &GatesSpec) {
        let spec_dir = dir.join("specs").join(slug);
        std::fs::create_dir_all(&spec_dir).unwrap();
        let content = toml::to_string_pretty(gates).unwrap();
        std::fs::write(spec_dir.join("gates.toml"), content).unwrap();
    }

    fn make_milestone(slug: &str, status: MilestoneStatus, chunks: Vec<ChunkRef>) -> Milestone {
        let now = Utc::now();
        Milestone {
            slug: slug.to_string(),
            name: slug.to_string(),
            description: None,
            status,
            quick: false,
            chunks,
            completed_chunks: vec![],
            depends_on: vec![],
            pr_branch: None,
            pr_base: None,
            pr_number: None,
            pr_url: None,
            pr_labels: None,
            pr_reviewers: None,
            pr_body_template: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_gates(name: &str) -> GatesSpec {
        GatesSpec {
            name: name.to_string(),
            description: String::new(),
            status: None,
            uat: None,
            gate: None,
            depends: vec![],
            milestone: None,
            order: None,
            extends: None,
            include: vec![],
            preconditions: None,
            criteria: vec![Criterion {
                name: "compiles".to_string(),
                description: "Code compiles".to_string(),
                cmd: Some("true".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: criterion::When::default(),
            }],
        }
    }

    #[test]
    fn no_milestones_returns_idle() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());
        let result = next_action(dir.path()).unwrap();
        assert_eq!(result, NextAction::Idle);
    }

    #[test]
    fn no_in_progress_milestone_returns_idle() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());
        let m = make_milestone("draft-ms", MilestoneStatus::Draft, vec![]);
        write_milestone(dir.path(), &m);
        let result = next_action(dir.path()).unwrap();
        assert_eq!(result, NextAction::Idle);
    }

    #[test]
    fn all_chunks_complete_returns_prompt_ship() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());

        let mut m = make_milestone(
            "ready-ms",
            MilestoneStatus::InProgress,
            vec![ChunkRef {
                slug: "auth".to_string(),
                order: 1,
                depends_on: vec![],
            }],
        );
        m.completed_chunks = vec!["auth".to_string()];
        write_milestone(dir.path(), &m);

        let result = next_action(dir.path()).unwrap();
        assert_eq!(
            result,
            NextAction::PromptShip {
                milestone_slug: "ready-ms".to_string()
            }
        );
    }

    #[test]
    fn draft_spec_returns_review_spec() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());

        let m = make_milestone(
            "ms",
            MilestoneStatus::InProgress,
            vec![ChunkRef {
                slug: "auth".to_string(),
                order: 1,
                depends_on: vec![],
            }],
        );
        write_milestone(dir.path(), &m);

        let gates = make_gates("auth");
        write_gates_spec(dir.path(), "auth", &gates);

        let result = next_action(dir.path()).unwrap();
        assert_eq!(
            result,
            NextAction::ReviewSpec {
                spec_name: "auth".to_string()
            }
        );
    }

    #[test]
    fn approved_spec_returns_execute() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());

        let m = make_milestone(
            "ms",
            MilestoneStatus::InProgress,
            vec![ChunkRef {
                slug: "auth".to_string(),
                order: 1,
                depends_on: vec![],
            }],
        );
        write_milestone(dir.path(), &m);

        let mut gates = make_gates("auth");
        gates.status = Some(GateSpecStatus::Approved);
        write_gates_spec(dir.path(), "auth", &gates);

        let result = next_action(dir.path()).unwrap();
        assert_eq!(
            result,
            NextAction::Execute {
                spec_name: "auth".to_string(),
                chunk_slug: Some("auth".to_string()),
            }
        );
    }

    #[test]
    fn next_action_is_idempotent() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());

        let m = make_milestone(
            "ms",
            MilestoneStatus::InProgress,
            vec![ChunkRef {
                slug: "auth".to_string(),
                order: 1,
                depends_on: vec![],
            }],
        );
        write_milestone(dir.path(), &m);

        let gates = make_gates("auth");
        write_gates_spec(dir.path(), "auth", &gates);

        let r1 = next_action(dir.path()).unwrap();
        let r2 = next_action(dir.path()).unwrap();
        assert_eq!(r1, r2, "next_action should be idempotent (pure function)");
    }

    #[test]
    fn strict_status_rejects_draft() {
        let gates = make_gates("test");
        let err = check_strict_status(&gates, true).unwrap_err();
        assert!(
            err.to_string().contains("strict_status"),
            "error should mention strict_status: {err}"
        );
    }

    #[test]
    fn strict_status_allows_approved() {
        let mut gates = make_gates("test");
        gates.status = Some(GateSpecStatus::Approved);
        assert!(check_strict_status(&gates, true).is_ok());
    }

    #[test]
    fn strict_status_disabled_allows_all() {
        let gates = make_gates("test");
        assert!(check_strict_status(&gates, false).is_ok());
    }

    #[test]
    fn quick_milestone_returns_review_spec() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());

        // Quick milestone: single chunk with same slug as milestone
        let m = make_milestone(
            "quick-task",
            MilestoneStatus::InProgress,
            vec![ChunkRef {
                slug: "quick-task".to_string(),
                order: 1,
                depends_on: vec![],
            }],
        );
        write_milestone(dir.path(), &m);

        let gates = make_gates("quick-task");
        write_gates_spec(dir.path(), "quick-task", &gates);

        let result = next_action(dir.path()).unwrap();
        assert_eq!(
            result,
            NextAction::ReviewSpec {
                spec_name: "quick-task".to_string()
            }
        );
    }
}
