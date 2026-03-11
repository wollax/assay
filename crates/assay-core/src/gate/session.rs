//! Agent session lifecycle management.
//!
//! Provides functions to create, report evaluations to, and finalize
//! agent gate evaluation sessions. Sessions manage the accumulate-then-commit
//! pattern for agent-reported criteria.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chrono::Utc;

use assay_types::{
    AgentEvaluation, AgentSession, CriterionResult, Enforcement, EnforcementSummary, EvaluatorRole,
    GateKind, GateResult, GateRunRecord, GateRunSummary,
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
) -> AgentSession {
    let ts = Utc::now();
    let session_id = history::generate_run_id(&ts);

    AgentSession {
        session_id,
        spec_name: spec_name.to_string(),
        created_at: ts,
        command_results,
        agent_evaluations: HashMap::new(),
        criteria_names,
        spec_enforcement,
    }
}

/// Report an agent evaluation for a criterion within a session.
///
/// Validates that the criterion name exists in the session's criteria set.
/// Multiple evaluations per criterion are allowed (e.g., re-evaluation
/// after a fix, or evaluations from different roles).
pub fn report_evaluation(
    session: &mut AgentSession,
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
pub fn build_finalized_record(session: &AgentSession, working_dir: Option<&str>) -> GateRunRecord {
    let start = Instant::now();

    let mut results = session.command_results.clone();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut enforcement_summary = EnforcementSummary::default();

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

    // Count all results
    for cr in &results {
        let enforcement = cr.enforcement;
        match &cr.result {
            Some(gate_result) => {
                if gate_result.passed {
                    passed += 1;
                    match enforcement {
                        Enforcement::Required => enforcement_summary.required_passed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_passed += 1,
                    }
                } else {
                    failed += 1;
                    match enforcement {
                        Enforcement::Required => enforcement_summary.required_failed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                    }
                }
            }
            None => {
                skipped += 1;
            }
        }
    }

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
    }
}

/// Finalize a session, producing a complete gate run record and saving to history.
///
/// Convenience wrapper around [`build_finalized_record`] that also persists the
/// record via [`history::save`]. Keeps the original signature for backward
/// compatibility with existing callers and tests.
pub fn finalize_session(
    session: &AgentSession,
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
pub fn finalize_as_timed_out(session: &AgentSession) -> GateRunRecord {
    let mut results = session.command_results.clone();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut enforcement_summary = EnforcementSummary::default();

    // Count command results
    for cr in &results {
        let enforcement = cr.enforcement;
        match &cr.result {
            Some(gate_result) => {
                if gate_result.passed {
                    passed += 1;
                    match enforcement {
                        Enforcement::Required => enforcement_summary.required_passed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_passed += 1,
                    }
                } else {
                    failed += 1;
                    match enforcement {
                        Enforcement::Required => enforcement_summary.required_failed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                    }
                }
            }
            None => {
                skipped += 1;
            }
        }
    }

    // Build results for agent criteria
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

                if best.passed {
                    passed += 1;
                    match enforcement {
                        Enforcement::Required => enforcement_summary.required_passed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_passed += 1,
                    }
                } else {
                    failed += 1;
                    match enforcement {
                        Enforcement::Required => enforcement_summary.required_failed += 1,
                        Enforcement::Advisory => enforcement_summary.advisory_failed += 1,
                    }
                }

                results.push(CriterionResult {
                    criterion_name: criterion_name.clone(),
                    result: Some(gate_result),
                    enforcement,
                });
            }
            _ => {
                // Un-evaluated: required => failed, advisory => skipped
                if enforcement == Enforcement::Required {
                    failed += 1;
                    enforcement_summary.required_failed += 1;
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
                    skipped += 1;
                    results.push(CriterionResult {
                        criterion_name: criterion_name.clone(),
                        result: None,
                        enforcement,
                    });
                }
            }
        }
    }

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
    }
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
        );

        assert!(
            !session.session_id.is_empty(),
            "session_id should be non-empty"
        );
        assert_eq!(session.spec_name, "test-spec");
        assert!(session.agent_evaluations.is_empty());
    }

    #[test]
    fn report_evaluation_valid_criterion() {
        let mut session = create_session(
            "test-spec",
            HashSet::from(["code-review".to_string()]),
            HashMap::new(),
            vec![],
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
}
