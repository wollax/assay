//! Agent session lifecycle management.
//!
//! Provides functions to create, report evaluations to, and finalize
//! agent gate evaluation sessions. Sessions manage the accumulate-then-commit
//! pattern for agent-reported criteria.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tempfile::NamedTempFile;

use chrono::Utc;

use assay_types::{
    AgentEvaluation, CriterionResult, Enforcement, EnforcementSummary, EvaluatorRole,
    GateEvalContext, GateKind, GateResult, GateRunRecord, GateRunSummary,
};

use crate::error::{AssayError, Result};
use crate::history;

/// Create a new agent session for gate evaluation.
///
/// Initializes a session with the given spec name, criteria names,
/// resolved enforcement levels, and any command-based results that
/// were evaluated synchronously before the session began.
pub fn create_session(
    spec_name: &str,
    criteria_names: HashSet<String>,
    spec_enforcement: HashMap<String, Enforcement>,
    command_results: Vec<CriterionResult>,
    diff: Option<String>,
    diff_truncated: bool,
    diff_bytes_original: Option<usize>,
) -> GateEvalContext {
    let ts = Utc::now();
    let session_id = history::generate_run_id(&ts);

    GateEvalContext {
        session_id,
        spec_name: spec_name.to_string(),
        created_at: ts,
        command_results,
        agent_evaluations: HashMap::new(),
        criteria_names,
        spec_enforcement,
        diff,
        diff_truncated,
        diff_bytes_original,
    }
}

/// Report an agent evaluation for a criterion within a session.
///
/// Validates that the criterion name exists in the session's criteria set.
/// Multiple evaluations per criterion are allowed (e.g., re-evaluation
/// after a fix, or evaluations from different roles).
pub fn report_evaluation(
    session: &mut GateEvalContext,
    criterion_name: &str,
    evaluation: AgentEvaluation,
) -> Result<()> {
    if !session.criteria_names.contains(criterion_name) {
        return Err(AssayError::InvalidCriterion {
            spec_name: session.spec_name.clone(),
            criterion_name: criterion_name.to_string(),
        });
    }

    session
        .agent_evaluations
        .entry(criterion_name.to_string())
        .or_default()
        .push(evaluation);

    Ok(())
}

/// Tally pass/fail/skip counts and enforcement summary from a finalized results list.
///
/// Returns `(passed, failed, skipped, EnforcementSummary)`.
fn count_results(results: &[CriterionResult]) -> (usize, usize, usize, EnforcementSummary) {
    results.iter().fold(
        (0usize, 0usize, 0usize, EnforcementSummary::default()),
        |(mut passed, mut failed, mut skipped, mut enforcement), cr| {
            match &cr.result {
                Some(gate_result) => {
                    if gate_result.passed {
                        passed += 1;
                        match cr.enforcement {
                            Enforcement::Required => enforcement.required_passed += 1,
                            Enforcement::Advisory => enforcement.advisory_passed += 1,
                        }
                    } else {
                        failed += 1;
                        match cr.enforcement {
                            Enforcement::Required => enforcement.required_failed += 1,
                            Enforcement::Advisory => enforcement.advisory_failed += 1,
                        }
                    }
                }
                None => {
                    skipped += 1;
                }
            }
            (passed, failed, skipped, enforcement)
        },
    )
}

/// Resolve the highest-priority evaluator from a list of evaluations.
///
/// Priority: Human > Independent > SelfEval. Returns the evaluation
/// with the highest-priority role. If multiple evaluations share the
/// same role, the last one wins (most recent).
fn resolve_evaluator_priority(evaluations: &[AgentEvaluation]) -> Option<&AgentEvaluation> {
    if evaluations.is_empty() {
        return None;
    }

    let role_priority = |role: &EvaluatorRole| -> u8 {
        match role {
            EvaluatorRole::Human => 3,
            EvaluatorRole::Independent => 2,
            EvaluatorRole::SelfEval => 1,
        }
    };

    evaluations
        .iter()
        .max_by_key(|e| (role_priority(&e.evaluator_role), e.timestamp))
}

/// Build a finalized gate run record from a session without performing I/O.
///
/// Combines command results with agent evaluation results. For each
/// agent criterion, resolves the effective evaluation using role priority
/// (human > independent > self). Returns the record directly — the caller
/// is responsible for persisting it.
///
/// Agent-reported criteria default to advisory enforcement unless
/// overridden by the spec's enforcement map.
pub fn build_finalized_record(
    session: &GateEvalContext,
    working_dir: Option<&str>,
) -> GateRunRecord {
    let start = Instant::now();

    let mut results = session.command_results.clone();

    // Merge agent evaluations into results, replacing skipped placeholders
    // that gate_run inserts for AgentReport criteria.
    for criterion_name in &session.criteria_names {
        let enforcement = session
            .spec_enforcement
            .get(criterion_name)
            .copied()
            .unwrap_or(Enforcement::Advisory);

        if let Some(evaluations) = session.agent_evaluations.get(criterion_name)
            && !evaluations.is_empty()
        {
            let best = resolve_evaluator_priority(evaluations)
                .expect("non-empty evaluations should have a best");

            let gate_result = GateResult {
                passed: best.passed,
                kind: GateKind::AgentReport,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 0,
                timestamp: best.timestamp,
                truncated: false,
                original_bytes: None,
                evidence: Some(best.evidence.clone()),
                reasoning: Some(best.reasoning.clone()),
                confidence: best.confidence,
                evaluator_role: Some(best.evaluator_role),
            };

            // Replace the skipped placeholder if one exists, otherwise append.
            if let Some(existing) = results
                .iter_mut()
                .find(|cr| cr.criterion_name == *criterion_name && cr.result.is_none())
            {
                existing.result = Some(gate_result);
                existing.enforcement = enforcement;
            } else if !results
                .iter()
                .any(|cr| cr.criterion_name == *criterion_name)
            {
                results.push(CriterionResult {
                    criterion_name: criterion_name.clone(),
                    result: Some(gate_result),
                    enforcement,
                });
            }

            continue;
        }

        // Un-evaluated agent criterion: ensure a skipped entry exists.
        if !results
            .iter()
            .any(|cr| cr.criterion_name == *criterion_name)
        {
            results.push(CriterionResult {
                criterion_name: criterion_name.clone(),
                result: None,
                enforcement,
            });
        }
    }

    let (passed, failed, skipped, enforcement_summary) = count_results(&results);
    let total_duration_ms = start.elapsed().as_millis() as u64;

    GateRunRecord {
        run_id: session.session_id.clone(),
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: session.created_at,
        working_dir: working_dir.map(String::from),
        summary: GateRunSummary {
            spec_name: session.spec_name.clone(),
            results,
            passed,
            failed,
            skipped,
            total_duration_ms,
            enforcement: enforcement_summary,
        },
        diff_truncation: None,
    }
}

/// Finalize a session, producing a complete gate run record and saving to history.
///
/// Convenience wrapper around [`build_finalized_record`] that also persists the
/// record via [`history::save`]. Keeps the original signature for backward
/// compatibility with existing callers and tests.
pub fn finalize_session(
    session: &GateEvalContext,
    assay_dir: &Path,
    working_dir: Option<&str>,
    max_history: Option<usize>,
) -> Result<GateRunRecord> {
    let record = build_finalized_record(session, working_dir);
    history::save(assay_dir, &record, max_history)?;
    Ok(record)
}

/// Finalize a session as timed out, without saving.
///
/// Any un-evaluated required agent criteria count as failures.
/// Returns the record for the caller to decide whether to persist.
pub fn finalize_as_timed_out(session: &GateEvalContext) -> GateRunRecord {
    let mut results = session.command_results.clone();

    // Build results for agent criteria not already present in command_results
    for criterion_name in &session.criteria_names {
        if results
            .iter()
            .any(|cr| &cr.criterion_name == criterion_name)
        {
            continue;
        }

        let enforcement = session
            .spec_enforcement
            .get(criterion_name)
            .copied()
            .unwrap_or(Enforcement::Advisory);

        match session.agent_evaluations.get(criterion_name) {
            Some(evaluations) if !evaluations.is_empty() => {
                let best = resolve_evaluator_priority(evaluations)
                    .expect("non-empty evaluations should have a best");

                results.push(CriterionResult {
                    criterion_name: criterion_name.clone(),
                    result: Some(GateResult {
                        passed: best.passed,
                        kind: GateKind::AgentReport,
                        stdout: String::new(),
                        stderr: String::new(),
                        exit_code: None,
                        duration_ms: 0,
                        timestamp: best.timestamp,
                        truncated: false,
                        original_bytes: None,
                        evidence: Some(best.evidence.clone()),
                        reasoning: Some(best.reasoning.clone()),
                        confidence: best.confidence,
                        evaluator_role: Some(best.evaluator_role),
                    }),
                    enforcement,
                });
            }
            _ => {
                // Un-evaluated: required => failed (timeout), advisory => skipped
                if enforcement == Enforcement::Required {
                    results.push(CriterionResult {
                        criterion_name: criterion_name.clone(),
                        result: Some(GateResult {
                            passed: false,
                            kind: GateKind::AgentReport,
                            stdout: String::new(),
                            stderr: "session timed out before evaluation".to_string(),
                            exit_code: None,
                            duration_ms: 0,
                            timestamp: Utc::now(),
                            truncated: false,
                            original_bytes: None,
                            evidence: None,
                            reasoning: None,
                            confidence: None,
                            evaluator_role: None,
                        }),
                        enforcement,
                    });
                } else {
                    results.push(CriterionResult {
                        criterion_name: criterion_name.clone(),
                        result: None,
                        enforcement,
                    });
                }
            }
        }
    }

    let (passed, failed, skipped, enforcement_summary) = count_results(&results);

    GateRunRecord {
        run_id: session.session_id.clone(),
        assay_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: session.created_at,
        working_dir: None,
        summary: GateRunSummary {
            spec_name: session.spec_name.clone(),
            results,
            passed,
            failed,
            skipped,
            total_duration_ms: 0,
            enforcement: enforcement_summary,
        },
        diff_truncation: None,
    }
}

/// Persist a gate eval context as atomic pretty-printed JSON.
///
/// Creates `.assay/gate_sessions/` if it does not exist. Uses the tempfile-then-rename
/// pattern to guarantee the file is either fully written or absent.
///
/// Returns the final path on success.
pub fn save_context(assay_dir: &Path, context: &GateEvalContext) -> Result<PathBuf> {
    let sessions_dir = assay_dir.join("gate_sessions");
    std::fs::create_dir_all(&sessions_dir)
        .map_err(|e| AssayError::io("creating gate_sessions directory", &sessions_dir, e))?;

    history::validate_path_component(&context.session_id, "session ID")?;

    let final_path = sessions_dir.join(format!("{}.json", context.session_id));

    let json = serde_json::to_string_pretty(context).map_err(|e| {
        AssayError::json(
            format!("serializing gate eval context {}", context.session_id),
            &final_path,
            e,
        )
    })?;

    let mut tmpfile = NamedTempFile::new_in(&sessions_dir).map_err(|e| {
        AssayError::io("creating temp file for gate eval context", &sessions_dir, e)
    })?;

    tmpfile
        .write_all(json.as_bytes())
        .map_err(|e| AssayError::io("writing gate eval context", &final_path, e))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::io("syncing gate eval context", &final_path, e))?;
    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting gate eval context", &final_path, e.error))?;

    // Evict old sessions to prevent unbounded directory growth.
    // Session IDs are ULID-like (chronologically sortable), so lexicographic
    // order == chronological order. Keep the 50 most recent.
    const MAX_GATE_SESSIONS: usize = 50;
    evict_old_sessions(&sessions_dir, MAX_GATE_SESSIONS);

    Ok(final_path)
}

/// Remove oldest gate session files, keeping at most `keep` entries.
fn evict_old_sessions(sessions_dir: &Path, keep: usize) {
    let entries: Vec<_> = match std::fs::read_dir(sessions_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect(),
        Err(_) => return,
    };
    if entries.len() <= keep {
        return;
    }
    let mut paths: Vec<_> = entries.into_iter().map(|e| e.path()).collect();
    paths.sort();
    let to_remove = paths.len() - keep;
    for path in paths.into_iter().take(to_remove) {
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), error = %e, "failed to evict old gate session");
        }
    }
}

/// Load a gate eval context by ID from `.assay/gate_sessions/<session_id>.json`.
///
/// Returns [`AssayError::GateEvalContextNotFound`] if the file does not exist.
/// Returns an error if `session_id` contains path traversal components.
pub fn load_context(assay_dir: &Path, session_id: &str) -> Result<GateEvalContext> {
    history::validate_path_component(session_id, "session ID")?;

    let path = assay_dir
        .join("gate_sessions")
        .join(format!("{session_id}.json"));

    let content = std::fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AssayError::GateEvalContextNotFound {
                session_id: session_id.to_string(),
            }
        } else {
            AssayError::io("reading gate eval context", &path, e)
        }
    })?;

    serde_json::from_str(&content)
        .map_err(|e| AssayError::json("deserializing gate eval context", &path, e))
}

/// List gate eval context IDs in lexicographic order.
///
/// Returns an empty vec if the `gate_sessions` directory does not exist.
pub fn list_contexts(assay_dir: &Path) -> Result<Vec<String>> {
    let sessions_dir = assay_dir.join("gate_sessions");
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut ids: Vec<String> = std::fs::read_dir(&sessions_dir)
        .map_err(|e| AssayError::io("listing gate eval contexts", &sessions_dir, e))?
        .filter_map(|entry| match entry {
            Ok(e) => Some(e),
            Err(e) => {
                tracing::warn!("skipping gate eval context entry: {e}");
                None
            }
        })
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                path.file_stem().and_then(|s| s.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();

    ids.sort();
    Ok(ids)
}

/// Find the most recent gate eval context for a given spec name.
///
/// Scans `.assay/gate_sessions/*.json` in reverse order (most recent
/// first) and returns the first [`GateEvalContext`] whose `spec_name`
/// matches the given `spec_name`.
///
/// ## Ordering invariant
///
/// Correctness relies on session IDs being timestamp-prefixed (format
/// `YYYYMMDDTHHMMSSZ-xxxxxx`) so that lexicographic ascending order
/// equals chronological order.  [`list_contexts`] returns IDs in that
/// ascending order; this function reverses the list before iterating
/// so the most recently created session is checked first.
///
/// ## Error handling
///
/// Context files that cannot be read or deserialized are logged at
/// `warn` level and skipped — they do not cause the function to fail.
/// An `Ok(None)` return means *no matching readable context was found*;
/// it does not guarantee that no context was ever persisted for the
/// spec.
///
/// Returns `Ok(None)` without error when the `gate_sessions` directory
/// does not exist (fresh project).
pub fn find_context_for_spec(assay_dir: &Path, spec_name: &str) -> Result<Option<GateEvalContext>> {
    let ids = list_contexts(assay_dir)?;
    for id in ids.into_iter().rev() {
        match load_context(assay_dir, &id) {
            Ok(ctx) if ctx.spec_name == spec_name => return Ok(Some(ctx)),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    session_id = %id,
                    error = %e,
                    "skipping unreadable context file"
                );
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::Confidence;

    fn make_evaluation(
        passed: bool,
        role: EvaluatorRole,
        confidence: Option<Confidence>,
    ) -> AgentEvaluation {
        AgentEvaluation {
            passed,
            evidence: format!("evidence for {role:?}"),
            reasoning: format!("reasoning for {role:?}"),
            confidence,
            evaluator_role: role,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn create_session_generates_unique_id() {
        let session = create_session(
            "test-spec",
            HashSet::from(["c1".to_string()]),
            HashMap::new(),
            vec![],
            None,
            false,
            None,
        );

        assert!(
            !session.session_id.is_empty(),
            "session_id should be non-empty"
        );
        assert_eq!(session.spec_name, "test-spec");
        assert!(session.agent_evaluations.is_empty());
    }

    #[test]
    fn create_session_with_diff_stores_fields() {
        let diff_content = "diff --git a/foo.rs b/foo.rs\n+hello\n".to_string();
        let session = create_session(
            "test-spec",
            HashSet::from(["c1".to_string()]),
            HashMap::new(),
            vec![],
            Some(diff_content.clone()),
            true,
            Some(65536),
        );

        assert_eq!(session.diff.as_deref(), Some(diff_content.as_str()));
        assert!(session.diff_truncated);
        assert_eq!(session.diff_bytes_original, Some(65536));
    }

    #[test]
    fn create_session_without_diff_stores_none() {
        let session = create_session(
            "test-spec",
            HashSet::from(["c1".to_string()]),
            HashMap::new(),
            vec![],
            None,
            false,
            None,
        );

        assert!(session.diff.is_none());
        assert!(!session.diff_truncated);
        assert!(session.diff_bytes_original.is_none());
    }

    #[test]
    fn report_evaluation_valid_criterion() {
        let mut session = create_session(
            "test-spec",
            HashSet::from(["code-review".to_string()]),
            HashMap::new(),
            vec![],
            None,
            false,
            None,
        );

        let eval = make_evaluation(true, EvaluatorRole::SelfEval, Some(Confidence::High));
        let result = report_evaluation(&mut session, "code-review", eval);

        assert!(result.is_ok());
        assert_eq!(session.agent_evaluations["code-review"].len(), 1);
    }

    #[test]
    fn report_evaluation_unknown_criterion_fails() {
        let mut session = create_session(
            "test-spec",
            HashSet::from(["code-review".to_string()]),
            HashMap::new(),
            vec![],
            None,
            false,
            None,
        );

        let eval = make_evaluation(true, EvaluatorRole::SelfEval, None);
        let result = report_evaluation(&mut session, "nonexistent", eval);

        assert!(result.is_err());
        let err = result.unwrap_err();
        let display = err.to_string();
        assert!(
            display.contains("nonexistent"),
            "error should mention the criterion name, got: {display}"
        );
    }

    #[test]
    fn report_evaluation_multiple_roles() {
        let mut session = create_session(
            "test-spec",
            HashSet::from(["code-review".to_string()]),
            HashMap::new(),
            vec![],
            None,
            false,
            None,
        );

        let eval1 = make_evaluation(true, EvaluatorRole::SelfEval, Some(Confidence::Medium));
        let eval2 = make_evaluation(false, EvaluatorRole::Independent, Some(Confidence::High));

        report_evaluation(&mut session, "code-review", eval1).unwrap();
        report_evaluation(&mut session, "code-review", eval2).unwrap();

        assert_eq!(session.agent_evaluations["code-review"].len(), 2);
    }

    #[test]
    fn finalize_session_produces_record() {
        let dir = tempfile::tempdir().unwrap();

        let mut session = create_session(
            "test-spec",
            HashSet::from(["agent-check".to_string()]),
            HashMap::from([("agent-check".to_string(), Enforcement::Required)]),
            vec![],
            None,
            false,
            None,
        );

        let eval = make_evaluation(true, EvaluatorRole::SelfEval, Some(Confidence::High));
        report_evaluation(&mut session, "agent-check", eval).unwrap();

        let record = finalize_session(&session, dir.path(), None, None).unwrap();

        assert_eq!(record.summary.spec_name, "test-spec");
        assert_eq!(record.summary.passed, 1);
        assert_eq!(record.summary.failed, 0);
        assert_eq!(record.summary.skipped, 0);
        assert_eq!(record.summary.results.len(), 1);

        let cr = &record.summary.results[0];
        assert_eq!(cr.criterion_name, "agent-check");
        assert!(cr.result.as_ref().unwrap().passed);
        assert!(cr.result.as_ref().unwrap().evidence.is_some());
        assert_eq!(cr.enforcement, Enforcement::Required);
    }

    #[test]
    fn finalize_resolves_highest_priority_evaluator() {
        let dir = tempfile::tempdir().unwrap();

        let mut session = create_session(
            "test-spec",
            HashSet::from(["review".to_string()]),
            HashMap::from([("review".to_string(), Enforcement::Required)]),
            vec![],
            None,
            false,
            None,
        );

        // SelfEval says pass, Human says fail — Human should win
        let self_eval = make_evaluation(true, EvaluatorRole::SelfEval, Some(Confidence::High));
        let human_eval = make_evaluation(false, EvaluatorRole::Human, Some(Confidence::High));

        report_evaluation(&mut session, "review", self_eval).unwrap();
        report_evaluation(&mut session, "review", human_eval).unwrap();

        let record = finalize_session(&session, dir.path(), None, None).unwrap();

        assert_eq!(
            record.summary.failed, 1,
            "human evaluation should win (fail)"
        );
        assert_eq!(record.summary.passed, 0);

        let cr = &record.summary.results[0];
        let result = cr.result.as_ref().unwrap();
        assert!(!result.passed, "human says fail, should be fail");
        assert_eq!(result.evaluator_role, Some(EvaluatorRole::Human));
    }

    #[test]
    fn finalize_timed_out_marks_unevaluated_as_failed() {
        let mut session = create_session(
            "test-spec",
            HashSet::from(["required-check".to_string(), "advisory-check".to_string()]),
            HashMap::from([
                ("required-check".to_string(), Enforcement::Required),
                ("advisory-check".to_string(), Enforcement::Advisory),
            ]),
            vec![],
            None,
            false,
            None,
        );

        // Only evaluate one criterion
        let eval = make_evaluation(true, EvaluatorRole::SelfEval, Some(Confidence::High));
        report_evaluation(&mut session, "advisory-check", eval).unwrap();

        let record = finalize_as_timed_out(&session);

        // required-check was not evaluated => should count as failed
        assert_eq!(record.summary.enforcement.required_failed, 1);
        // advisory-check was evaluated and passed
        assert_eq!(record.summary.enforcement.advisory_passed, 1);
        assert_eq!(record.summary.passed, 1);
        assert_eq!(record.summary.failed, 1);
    }

    #[test]
    fn resolve_evaluator_priority_human_wins() {
        let self_eval = make_evaluation(true, EvaluatorRole::SelfEval, None);
        let independent = make_evaluation(false, EvaluatorRole::Independent, None);
        let human = make_evaluation(true, EvaluatorRole::Human, None);

        let evals = vec![self_eval, independent, human];
        let best = resolve_evaluator_priority(&evals).unwrap();

        assert_eq!(best.evaluator_role, EvaluatorRole::Human);
    }

    #[test]
    fn resolve_evaluator_priority_independent_over_self() {
        let self_eval = make_evaluation(true, EvaluatorRole::SelfEval, None);
        let independent = make_evaluation(false, EvaluatorRole::Independent, None);

        let evals = vec![self_eval, independent];
        let best = resolve_evaluator_priority(&evals).unwrap();

        assert_eq!(best.evaluator_role, EvaluatorRole::Independent);
    }

    #[test]
    fn resolve_evaluator_priority_empty_returns_none() {
        let evals: Vec<AgentEvaluation> = vec![];
        assert!(resolve_evaluator_priority(&evals).is_none());
    }

    // ── save / load / list contexts ──────────────────────────────

    fn make_test_context(spec_name: &str) -> GateEvalContext {
        create_session(
            spec_name,
            HashSet::from(["c1".to_string()]),
            HashMap::new(),
            vec![],
            None,
            false,
            None,
        )
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = make_test_context("round-trip");
        save_context(dir.path(), &ctx).unwrap();
        let loaded = load_context(dir.path(), &ctx.session_id).unwrap();
        assert_eq!(ctx, loaded);
    }

    #[test]
    fn save_creates_directory() {
        let dir = tempfile::tempdir().unwrap();
        let gate_dir = dir.path().join("gate_sessions");
        assert!(!gate_dir.exists());

        let ctx = make_test_context("dir-create");
        save_context(dir.path(), &ctx).unwrap();
        assert!(gate_dir.is_dir());
    }

    #[test]
    fn load_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_context(dir.path(), "01NONEXISTENT0000000000000");
        assert!(result.is_err());
        assert!(
            matches!(
                result.unwrap_err(),
                AssayError::GateEvalContextNotFound { .. }
            ),
            "expected GateEvalContextNotFound"
        );
    }

    #[test]
    fn list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let ids = list_contexts(dir.path()).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn list_returns_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let c1 = make_test_context("spec");
        let c2 = make_test_context("spec");
        let c3 = make_test_context("spec");

        // Save in non-sorted order.
        save_context(dir.path(), &c3).unwrap();
        save_context(dir.path(), &c1).unwrap();
        save_context(dir.path(), &c2).unwrap();

        let ids = list_contexts(dir.path()).unwrap();
        assert_eq!(ids.len(), 3);

        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "list_contexts should return sorted IDs");

        assert!(ids.contains(&c1.session_id));
        assert!(ids.contains(&c2.session_id));
        assert!(ids.contains(&c3.session_id));
    }

    #[test]
    fn save_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = make_test_context("spec");
        ctx.session_id = "../evil".to_string();
        let result = save_context(dir.path(), &ctx);
        assert!(result.is_err(), "should reject path-traversal session ID");
    }

    #[test]
    fn load_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_context(dir.path(), "../evil");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid session ID"),
            "should reject via path validation, got: {msg}"
        );
    }

    // ── find_context_for_spec ─────────────────────────────────

    #[test]
    fn test_find_context_for_spec_returns_match() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = make_test_context("target-spec");
        let expected_id = ctx.session_id.clone();
        save_context(dir.path(), &ctx).unwrap();

        let found = find_context_for_spec(dir.path(), "target-spec").unwrap();
        assert!(found.is_some(), "should find a matching context");
        assert_eq!(found.unwrap().session_id, expected_id);
    }

    #[test]
    fn test_find_context_for_spec_returns_none_on_miss() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = make_test_context("other-spec");
        save_context(dir.path(), &ctx).unwrap();

        let found = find_context_for_spec(dir.path(), "target-spec").unwrap();
        assert!(
            found.is_none(),
            "should not find a context for a different spec"
        );
    }

    #[test]
    fn test_find_context_for_spec_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let found = find_context_for_spec(dir.path(), "target-spec").unwrap();
        assert!(found.is_none(), "should return None for empty/missing dir");
    }

    #[test]
    fn test_find_context_for_spec_returns_most_recent() {
        let dir = tempfile::tempdir().unwrap();

        // Use manually-assigned IDs to guarantee lexicographic ordering.
        let mut ctx1 = make_test_context("target-spec");
        ctx1.session_id = "20260101T000000Z-aaaaaa".to_string();
        save_context(dir.path(), &ctx1).unwrap();

        let mut ctx2 = make_test_context("target-spec");
        ctx2.session_id = "20260102T000000Z-bbbbbb".to_string();
        save_context(dir.path(), &ctx2).unwrap();

        let found = find_context_for_spec(dir.path(), "target-spec").unwrap();
        assert!(found.is_some(), "should find a matching context");
        assert_eq!(
            found.unwrap().session_id,
            "20260102T000000Z-bbbbbb",
            "should return the most recent (lexicographically last) context"
        );
    }
}
