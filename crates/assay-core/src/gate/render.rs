//! Surface-adapted gate evidence rendering.
//!
//! Transforms [`GateRunRecord`] into format-appropriate output for
//! different surfaces: terminal, in-agent markdown, PR body, and PR check.
//!
//! All renderers are pure functions with no side effects.

use std::fmt::Write;

use assay_types::{Enforcement, GateRunRecord};

/// Render a 1-line terminal summary with pass/fail counts and duration.
///
/// Example: `PASS: auth-flow — 3/4 passed in 1.2s`
pub fn render_terminal(record: &GateRunRecord) -> String {
    let s = &record.summary;
    let total = s.passed + s.failed;
    let duration = format!("{:.1}s", s.total_duration_ms as f64 / 1000.0);

    let advisory_note = if s.enforcement.advisory_failed > 0 {
        format!(" ({} advisory failed)", s.enforcement.advisory_failed)
    } else {
        String::new()
    };

    let icon = if s.enforcement.required_failed == 0 {
        "PASS"
    } else {
        "FAIL"
    };

    format!(
        "{icon}: {name} — {passed}/{total} passed{advisory_note} in {duration}",
        name = s.spec_name,
        passed = s.passed,
    )
}

/// Render a collapsed markdown block for in-agent context (minimal token use).
pub fn render_markdown_collapsed(record: &GateRunRecord) -> String {
    let s = &record.summary;
    let icon = if s.enforcement.required_failed == 0 {
        "PASS"
    } else {
        "FAIL"
    };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "**Gate: {}** — {icon} ({}/{} passed)",
        s.spec_name,
        s.passed,
        s.passed + s.failed
    );

    if !s.results.is_empty() {
        let _ = writeln!(out, "\n<details><summary>Criteria details</summary>\n");
        for cr in &s.results {
            let status = match &cr.result {
                Some(r) if r.passed => "PASS",
                Some(_) => "FAIL",
                None => "SKIP",
            };
            let enf = match cr.enforcement {
                Enforcement::Advisory => " (advisory)",
                Enforcement::Required => "",
            };
            let _ = writeln!(out, "- **{status}** {}{enf}", cr.criterion_name);
        }
        let _ = writeln!(out, "\n</details>");
    }

    out
}

/// Render a summary suitable for a PR body.
pub fn render_pr_body(record: &GateRunRecord) -> String {
    let s = &record.summary;
    let icon = if s.enforcement.required_failed == 0 {
        "PASS"
    } else {
        "FAIL"
    };

    format!(
        "**Gate: {}** — {icon} | {}/{} passed | Run: `{}`",
        s.spec_name,
        s.passed,
        s.passed + s.failed,
        record.run_id,
    )
}

/// Render a full per-criterion table with collapsible evidence blocks.
pub fn render_pr_check(record: &GateRunRecord) -> String {
    let s = &record.summary;
    let mut out = String::new();

    let icon = if s.enforcement.required_failed == 0 {
        "PASS"
    } else {
        "FAIL"
    };

    let _ = writeln!(out, "## Gate: {} — {icon}\n", s.spec_name);
    let _ = writeln!(out, "| Criterion | Status | Enforcement |");
    let _ = writeln!(out, "|-----------|--------|-------------|");

    for cr in &s.results {
        let status = match &cr.result {
            Some(r) if r.passed => "PASS",
            Some(_) => "FAIL",
            None => "SKIP",
        };
        let _ = writeln!(
            out,
            "| {} | {status} | {} |",
            cr.criterion_name, cr.enforcement
        );
    }

    let failures: Vec<_> = s
        .results
        .iter()
        .filter(|cr| cr.result.as_ref().is_some_and(|r| !r.passed))
        .collect();

    if !failures.is_empty() {
        let _ = writeln!(out, "\n### Failed Criteria\n");
        for cr in failures {
            if let Some(ref result) = cr.result {
                let _ = writeln!(out, "<details><summary>{}</summary>\n", cr.criterion_name);
                if !result.stdout.is_empty() {
                    let _ = writeln!(out, "```\n{}\n```", result.stdout);
                }
                if !result.stderr.is_empty() {
                    let _ = writeln!(out, "**stderr:**\n```\n{}\n```", result.stderr);
                }
                let _ = writeln!(out, "</details>\n");
            }
        }
    }

    let _ = writeln!(
        out,
        "\n*Run ID: `{}` | Duration: {:.1}s*",
        record.run_id,
        s.total_duration_ms as f64 / 1000.0,
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::*;
    use chrono::Utc;

    fn make_record(all_pass: bool) -> GateRunRecord {
        let now = Utc::now();
        GateRunRecord {
            run_id: "20260414T120000Z-abc123".to_string(),
            assay_version: "0.8.0".to_string(),
            timestamp: now,
            working_dir: Some("/tmp/test".to_string()),
            diff_truncation: None,
            precondition_blocked: None,
            summary: GateRunSummary {
                spec_name: "auth-flow".to_string(),
                passed: if all_pass { 2 } else { 1 },
                failed: if all_pass { 0 } else { 1 },
                skipped: 0,
                results: vec![
                    CriterionResult {
                        criterion_name: "compiles".to_string(),
                        result: Some(GateResult {
                            passed: true,
                            kind: GateKind::Command {
                                cmd: "cargo build".to_string(),
                            },
                            stdout: "ok".to_string(),
                            stderr: String::new(),
                            exit_code: Some(0),
                            duration_ms: 500,
                            timestamp: now,
                            truncated: false,
                            original_bytes: None,
                            evidence: None,
                            reasoning: None,
                            confidence: None,
                            evaluator_role: None,
                        }),
                        enforcement: Enforcement::Required,
                        source: None,
                    },
                    CriterionResult {
                        criterion_name: "tests".to_string(),
                        result: Some(GateResult {
                            passed: all_pass,
                            kind: GateKind::Command {
                                cmd: "cargo test".to_string(),
                            },
                            stdout: "test result".to_string(),
                            stderr: if all_pass {
                                String::new()
                            } else {
                                "test failed".to_string()
                            },
                            exit_code: Some(if all_pass { 0 } else { 1 }),
                            duration_ms: 1000,
                            timestamp: now,
                            truncated: false,
                            original_bytes: None,
                            evidence: None,
                            reasoning: None,
                            confidence: None,
                            evaluator_role: None,
                        }),
                        enforcement: Enforcement::Required,
                        source: None,
                    },
                ],
                total_duration_ms: 1500,
                enforcement: EnforcementSummary {
                    required_passed: if all_pass { 2 } else { 1 },
                    required_failed: if all_pass { 0 } else { 1 },
                    advisory_passed: 0,
                    advisory_failed: 0,
                },
            },
        }
    }

    #[test]
    fn terminal_all_pass() {
        let record = make_record(true);
        let output = render_terminal(&record);
        assert!(output.contains("PASS"), "should contain PASS: {output}");
        assert!(output.contains("2/2"), "should show 2/2: {output}");
        assert!(output.contains("auth-flow"), "should contain spec name");
    }

    #[test]
    fn terminal_with_failure() {
        let record = make_record(false);
        let output = render_terminal(&record);
        assert!(output.contains("FAIL"), "should contain FAIL: {output}");
        assert!(output.contains("1/2"), "should show 1/2: {output}");
    }

    #[test]
    fn pr_check_all_pass() {
        let record = make_record(true);
        let output = render_pr_check(&record);
        assert!(output.contains("PASS"), "should contain PASS");
        assert!(output.contains("compiles"), "should list criteria");
        assert!(!output.contains("Failed Criteria"), "no failure section");
    }

    #[test]
    fn pr_check_with_failure() {
        let record = make_record(false);
        let output = render_pr_check(&record);
        assert!(output.contains("FAIL"), "should contain FAIL");
        assert!(
            output.contains("Failed Criteria"),
            "should have failure section"
        );
        assert!(
            output.contains("<details>"),
            "should have collapsible blocks"
        );
        assert!(output.contains("test failed"), "should include stderr");
    }

    #[test]
    fn pr_body_includes_run_id() {
        let record = make_record(true);
        let output = render_pr_body(&record);
        assert!(
            output.contains("20260414T120000Z-abc123"),
            "should include run ID: {output}"
        );
    }

    #[test]
    fn markdown_collapsed_format() {
        let record = make_record(true);
        let output = render_markdown_collapsed(&record);
        assert!(output.contains("<details>"), "should have details tag");
        assert!(output.contains("compiles"), "should list criteria");
    }
}
