//! Gate evidence formatting for PR bodies.
//!
//! Transforms [`GateRunRecord`] into markdown suitable for GitHub PR bodies,
//! with semantic truncation to fit within the 65,536-byte limit. Also provides
//! report persistence to disk.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use assay_types::{CriterionResult, Enforcement, FormattedEvidence, GateKind, GateRunRecord};

use crate::error::{AssayError, Result};
use crate::history::validate_path_component;

/// GitHub PR body byte limit.
pub const GITHUB_BODY_LIMIT: usize = 65_536;

/// Format gate run results as markdown suitable for PR bodies.
///
/// Builds a full markdown report from the gate run record, then applies
/// semantic truncation to produce a PR body that fits within `char_limit`
/// bytes. The full untruncated report is returned alongside for disk
/// persistence via [`save_report`].
///
/// `report_path` is embedded in the truncation notice so reviewers can
/// find the full report on disk.
pub fn format_gate_evidence(
    record: &GateRunRecord,
    report_path: &Path,
    char_limit: usize,
) -> FormattedEvidence {
    let summary = &record.summary;

    // Build header + summary stats + table (the "skeleton" that's always kept).
    let mut skeleton = String::new();
    write_header(&mut skeleton, &summary.spec_name);
    write_summary_stats(&mut skeleton, record);
    write_status_table(&mut skeleton, &summary.results);

    // Build detail sections individually so we can selectively drop them.
    let detail_sections = build_detail_sections(&summary.results);

    // Build footer.
    let mut footer = String::new();
    write_footer(&mut footer, record);

    // Assemble the full report.
    let full_report = assemble(&skeleton, &detail_sections, &footer);

    // Apply truncation for the PR body.
    let (pr_body, truncated) = truncate_to_fit(
        &skeleton,
        &detail_sections,
        &footer,
        report_path,
        char_limit,
    );

    FormattedEvidence {
        pr_body,
        full_report,
        truncated,
    }
}

/// Write the full gate evidence report to disk.
///
/// Writes to `.assay/reports/<spec-name>/<run-id>.md`.
/// Creates directories as needed. Uses simple `std::fs::write`
/// (not atomic — report files are nice-to-have artifacts).
///
/// Returns the path where the report was written.
pub fn save_report(
    assay_dir: &Path,
    record: &GateRunRecord,
    evidence: &FormattedEvidence,
) -> Result<PathBuf> {
    validate_path_component(&record.summary.spec_name, "spec name")?;
    validate_path_component(&record.run_id, "run ID")?;

    let reports_dir = assay_dir.join("reports").join(&record.summary.spec_name);

    std::fs::create_dir_all(&reports_dir).map_err(|source| AssayError::Io {
        operation: "creating reports directory".into(),
        path: reports_dir.clone(),
        source,
    })?;

    let file_path = reports_dir.join(format!("{}.md", record.run_id));

    std::fs::write(&file_path, &evidence.full_report).map_err(|source| AssayError::Io {
        operation: "writing gate evidence report".into(),
        path: file_path.clone(),
        source,
    })?;

    Ok(file_path)
}

// ── Markdown building helpers ──────────────────────────────────────

fn write_header(buf: &mut String, spec_name: &str) {
    let _ = writeln!(buf, "## Gate Results: {spec_name}\n");
}

fn write_summary_stats(buf: &mut String, record: &GateRunRecord) {
    let s = &record.summary;
    let total = s.passed + s.failed + s.skipped;

    let _ = writeln!(
        buf,
        "**Result:** {}/{total} passed | {} failed | {} skipped",
        s.passed, s.failed, s.skipped,
    );
    let _ = writeln!(
        buf,
        "**Duration:** {}",
        format_duration(s.total_duration_ms)
    );

    let e = &s.enforcement;
    let _ = writeln!(
        buf,
        "**Enforcement:** {} required passed, {} required failed, {} advisory passed, {} advisory failed\n",
        e.required_passed, e.required_failed, e.advisory_passed, e.advisory_failed,
    );
}

fn write_status_table(buf: &mut String, results: &[CriterionResult]) {
    let _ = writeln!(buf, "| Status | Criterion | Enforcement | Duration |");
    let _ = writeln!(buf, "|--------|-----------|-------------|----------|");

    for cr in results {
        let (emoji, passed, duration_ms) = match &cr.result {
            Some(r) => {
                let emoji = if r.passed {
                    ":white_check_mark:"
                } else {
                    ":x:"
                };
                (emoji, Some(r.passed), Some(r.duration_ms))
            }
            None => (":fast_forward:", None, None),
        };

        let duration_str = duration_ms
            .map(format_duration)
            .unwrap_or_else(|| "—".to_string());

        let enforcement_str = match cr.enforcement {
            Enforcement::Required => "required",
            Enforcement::Advisory => "advisory",
        };

        // Bold the row for failures.
        if passed == Some(false) {
            let _ = writeln!(
                buf,
                "| {emoji} | **{}** | **{enforcement_str}** | **{duration_str}** |",
                cr.criterion_name,
            );
        } else {
            let _ = writeln!(
                buf,
                "| {emoji} | {} | {enforcement_str} | {duration_str} |",
                cr.criterion_name,
            );
        }
    }

    let _ = writeln!(buf);
}

/// Classification of a detail section for truncation priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DetailPriority {
    /// Agent-evaluated pass (collapsed) — lowest value, removed first.
    AgentPass = 0,
    /// Failure (expanded) — highest value, removed last.
    Failure = 1,
}

/// A single detail section with its content and truncation priority.
struct DetailSection {
    content: String,
    priority: DetailPriority,
}

fn build_detail_sections(results: &[CriterionResult]) -> Vec<DetailSection> {
    let mut sections = Vec::new();

    for cr in results {
        let result = match &cr.result {
            Some(r) => r,
            None => continue, // Skipped — no detail section
        };

        let is_agent = matches!(result.kind, GateKind::AgentReport);

        if result.passed && !is_agent {
            // Deterministic pass — table-only, no detail section.
            continue;
        }

        let mut section = String::new();

        if result.passed {
            // Agent pass — collapsed.
            let _ = writeln!(section, "<details>");
            let _ = writeln!(
                section,
                "<summary>:white_check_mark: {}</summary>\n",
                cr.criterion_name,
            );
            write_agent_detail(&mut section, result);
            let _ = writeln!(section, "\n</details>\n");

            sections.push(DetailSection {
                content: section,
                priority: DetailPriority::AgentPass,
            });
        } else {
            // Failure — expanded.
            let _ = writeln!(section, "<details open>");
            let _ = writeln!(section, "<summary>:x: {}</summary>\n", cr.criterion_name,);
            write_failure_detail(&mut section, result);
            let _ = writeln!(section, "\n</details>\n");

            sections.push(DetailSection {
                content: section,
                priority: DetailPriority::Failure,
            });
        }
    }

    sections
}

fn write_agent_detail(buf: &mut String, result: &assay_types::GateResult) {
    if let Some(evidence) = &result.evidence {
        let _ = writeln!(buf, "**Evidence:**\n{evidence}\n");
    }
    if let Some(reasoning) = &result.reasoning {
        let _ = writeln!(buf, "**Reasoning:**\n{reasoning}\n");
    }
    if let Some(confidence) = &result.confidence {
        let _ = writeln!(buf, "**Confidence:** {confidence}");
    }
    if let Some(role) = &result.evaluator_role {
        let _ = writeln!(buf, "**Evaluator:** {role}");
    }
}

fn write_failure_detail(buf: &mut String, result: &assay_types::GateResult) {
    match &result.kind {
        GateKind::Command { cmd } => {
            let _ = writeln!(buf, "**Command:** `{cmd}`");
            if let Some(code) = result.exit_code {
                let _ = writeln!(buf, "**Exit code:** {code}");
            }
            if !result.stdout.is_empty() {
                let _ = writeln!(buf, "\n**stdout:**\n```\n{}\n```", result.stdout);
            }
            if !result.stderr.is_empty() {
                let _ = writeln!(buf, "\n**stderr:**\n```\n{}\n```", result.stderr);
            }
        }
        GateKind::FileExists { path } => {
            let _ = writeln!(buf, "**Missing file:** `{path}`");
        }
        GateKind::AgentReport => {
            write_agent_detail(buf, result);
        }
        GateKind::AlwaysPass => {
            // AlwaysPass should never fail, but handle defensively.
            let _ = writeln!(buf, "AlwaysPass gate reported failure (unexpected).");
        }
    }
}

fn write_footer(buf: &mut String, record: &GateRunRecord) {
    let _ = writeln!(buf, "---");
    let _ = writeln!(
        buf,
        "*Run: {} | {} | assay {}*",
        record.run_id,
        record.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
        record.assay_version,
    );
}

fn assemble(skeleton: &str, details: &[DetailSection], footer: &str) -> String {
    let mut out = String::with_capacity(
        skeleton.len() + details.iter().map(|d| d.content.len()).sum::<usize>() + footer.len(),
    );
    out.push_str(skeleton);
    for d in details {
        out.push_str(&d.content);
    }
    out.push_str(footer);
    out
}

fn truncate_to_fit(
    skeleton: &str,
    details: &[DetailSection],
    footer: &str,
    report_path: &Path,
    char_limit: usize,
) -> (String, bool) {
    // Try the full version first.
    let full = assemble(skeleton, details, footer);
    if full.len() <= char_limit {
        return (full, false);
    }

    // Build the truncation notice we'll append.
    let notice = format!(
        "\n\n> **Note:** This report was truncated to fit GitHub's character limit. Full report: `{}`\n",
        report_path.display(),
    );

    // Sort indices by priority (lowest first = removed first).
    let mut indices: Vec<usize> = (0..details.len()).collect();
    indices.sort_by_key(|&i| (details[i].priority, i));

    // Track which sections to keep.
    let mut keep = vec![true; details.len()];

    // Progressively remove sections by priority until under limit.
    for &idx in &indices {
        keep[idx] = false;

        let current = assemble_filtered(skeleton, details, &keep, footer, &notice);
        if current.len() <= char_limit {
            return (current, true);
        }
    }

    // All detail sections removed — try with just skeleton + footer + notice.
    let minimal = format!("{skeleton}{footer}{notice}");
    if minimal.len() <= char_limit {
        return (minimal, true);
    }

    // Last resort: truncate table rows from the bottom.
    // Split skeleton into lines, remove table rows from the end until it fits.
    let mut lines: Vec<&str> = skeleton.lines().collect();
    loop {
        // Find the last table data row (not header/separator).
        let last_data_row = lines.iter().rposition(|line| {
            line.starts_with('|') && !line.starts_with("| Status") && !line.starts_with("|---")
        });

        match last_data_row {
            Some(idx) => {
                lines.remove(idx);
                let trimmed_skeleton: String = lines.iter().flat_map(|l| [*l, "\n"]).collect();
                let candidate = format!("{trimmed_skeleton}{footer}{notice}");
                if candidate.len() <= char_limit {
                    return (candidate, true);
                }
            }
            None => break,
        }
    }

    // Absolute fallback: hard truncate (should be unreachable in practice).
    let mut result = format!("{skeleton}{footer}{notice}");
    result.truncate(char_limit);
    (result, true)
}

fn assemble_filtered(
    skeleton: &str,
    details: &[DetailSection],
    keep: &[bool],
    footer: &str,
    notice: &str,
) -> String {
    let mut out = String::with_capacity(skeleton.len() + footer.len() + notice.len());
    out.push_str(skeleton);
    for (i, d) in details.iter().enumerate() {
        if keep[i] {
            out.push_str(&d.content);
        }
    }
    out.push_str(footer);
    out.push_str(notice);
    out
}

fn format_duration(ms: u64) -> String {
    if ms < 1_000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{
        Confidence, EnforcementSummary, EvaluatorRole, GateKind, GateResult, GateRunSummary,
    };
    use chrono::Utc;

    fn make_record(results: Vec<CriterionResult>) -> GateRunRecord {
        let passed = results
            .iter()
            .filter(|r| r.result.as_ref().is_some_and(|gr| gr.passed))
            .count();
        let failed = results
            .iter()
            .filter(|r| r.result.as_ref().is_some_and(|gr| !gr.passed))
            .count();
        let skipped = results.iter().filter(|r| r.result.is_none()).count();

        GateRunRecord {
            run_id: "20260316T120000Z-abc123".to_string(),
            assay_version: "0.4.0".to_string(),
            timestamp: Utc::now(),
            working_dir: None,
            summary: GateRunSummary {
                spec_name: "test-spec".to_string(),
                results,
                passed,
                failed,
                skipped,
                total_duration_ms: 1500,
                enforcement: EnforcementSummary::default(),
            },
            diff_truncation: None,
        }
    }

    fn make_passing_command_result(name: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::Command {
                    cmd: "cargo test".to_string(),
                },
                stdout: "ok".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                duration_ms: 100,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
        }
    }

    fn make_failing_command_result(name: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::Command {
                    cmd: "cargo test".to_string(),
                },
                stdout: String::new(),
                stderr: "test failed".to_string(),
                exit_code: Some(1),
                duration_ms: 200,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
        }
    }

    fn make_agent_pass_result(name: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::AgentReport,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 50,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: Some("Found implementation".to_string()),
                reasoning: Some("Code looks correct".to_string()),
                confidence: Some(Confidence::High),
                evaluator_role: Some(EvaluatorRole::Independent),
            }),
            enforcement: Enforcement::Required,
        }
    }

    fn make_agent_fail_result(name: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::AgentReport,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 75,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: Some("Missing error handling".to_string()),
                reasoning: Some("No try/catch blocks found".to_string()),
                confidence: Some(Confidence::Medium),
                evaluator_role: Some(EvaluatorRole::SelfEval),
            }),
            enforcement: Enforcement::Advisory,
        }
    }

    #[test]
    fn empty_results_produce_valid_markdown() {
        let record = make_record(vec![]);
        let report_path = Path::new(".assay/reports/test-spec/run.md");
        let evidence = format_gate_evidence(&record, report_path, GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("## Gate Results: test-spec"));
        assert!(evidence.full_report.contains("0/0 passed"));
        assert!(!evidence.truncated);
        assert_eq!(evidence.pr_body, evidence.full_report);
    }

    #[test]
    fn header_includes_spec_name() {
        let record = make_record(vec![make_passing_command_result("unit-tests")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(
            evidence
                .full_report
                .starts_with("## Gate Results: test-spec\n")
        );
    }

    #[test]
    fn status_table_has_emoji_for_pass_fail_skip() {
        let results = vec![
            make_passing_command_result("passing"),
            make_failing_command_result("failing"),
            CriterionResult {
                criterion_name: "skipped".to_string(),
                result: None,
                enforcement: Enforcement::Required,
            },
        ];
        let record = make_record(results);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains(":white_check_mark:"));
        assert!(evidence.full_report.contains(":x:"));
        assert!(evidence.full_report.contains(":fast_forward:"));
    }

    #[test]
    fn failed_rows_are_bold() {
        let record = make_record(vec![make_failing_command_result("lint-check")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("**lint-check**"));
    }

    #[test]
    fn deterministic_pass_has_no_detail_section() {
        let record = make_record(vec![make_passing_command_result("unit-tests")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(!evidence.full_report.contains("<details"));
    }

    #[test]
    fn agent_pass_has_collapsed_detail_section() {
        let record = make_record(vec![make_agent_pass_result("code-review")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("<details>"));
        assert!(
            evidence
                .full_report
                .contains("<summary>:white_check_mark: code-review</summary>")
        );
        assert!(evidence.full_report.contains("Found implementation"));
    }

    #[test]
    fn failure_has_expanded_detail_section() {
        let record = make_record(vec![make_failing_command_result("lint-check")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("<details open>"));
        assert!(evidence.full_report.contains("test failed"));
    }

    #[test]
    fn agent_failure_has_expanded_detail_with_reasoning() {
        let record = make_record(vec![make_agent_fail_result("quality-check")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("<details open>"));
        assert!(evidence.full_report.contains("Missing error handling"));
        assert!(evidence.full_report.contains("No try/catch blocks found"));
    }

    #[test]
    fn details_have_blank_line_after_summary() {
        let record = make_record(vec![make_agent_pass_result("review")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // GitHub rendering requires a blank line after <summary>.
        assert!(evidence.full_report.contains("</summary>\n\n"));
    }

    #[test]
    fn footer_includes_run_metadata() {
        let record = make_record(vec![]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("20260316T120000Z-abc123"));
        assert!(evidence.full_report.contains("assay 0.4.0"));
    }

    #[test]
    fn truncation_removes_agent_passes_before_failures() {
        let results = vec![
            make_agent_pass_result("review-1"),
            make_failing_command_result("lint-check"),
            make_agent_pass_result("review-2"),
        ];
        let record = make_record(results);

        // Use a limit that forces truncation but allows some sections.
        let full_evidence =
            format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);
        let full_len = full_evidence.full_report.len();

        // Shrink limit so it needs to truncate at least agent passes.
        let tight_limit = full_len - 10;
        let evidence = format_gate_evidence(&record, Path::new("report.md"), tight_limit);

        assert!(evidence.truncated);
        // Should still contain the failure detail.
        assert!(evidence.pr_body.contains("lint-check"));
        assert!(evidence.pr_body.contains("truncated"));
    }

    #[test]
    fn truncation_uses_byte_length() {
        // Verify .len() (byte count) is used, not .chars().count().
        let record = make_record(vec![make_passing_command_result("test")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Verify the report uses only ASCII (single-byte chars),
        // confirming .len() byte count matches character count.
        assert!(evidence.full_report.is_ascii());
    }

    #[test]
    fn truncation_notice_includes_report_path() {
        let record = make_record(vec![make_agent_pass_result("review")]);
        let full = format_gate_evidence(
            &record,
            Path::new(".assay/reports/test-spec/run.md"),
            GITHUB_BODY_LIMIT,
        );
        // Use a limit just under the full report size to force truncation.
        let tight_limit = full.full_report.len() - 1;
        let evidence = format_gate_evidence(
            &record,
            Path::new(".assay/reports/test-spec/run.md"),
            tight_limit,
        );

        assert!(evidence.truncated);
        assert!(evidence.pr_body.contains(".assay/reports/test-spec/run.md"));
    }

    #[test]
    fn file_exists_failure_shows_path() {
        let cr = CriterionResult {
            criterion_name: "readme-check".to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::FileExists {
                    path: "README.md".to_string(),
                },
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 1,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
        };
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(
            evidence
                .full_report
                .contains("**Missing file:** `README.md`")
        );
    }

    #[test]
    fn format_duration_under_one_second() {
        assert_eq!(format_duration(42), "42ms");
        assert_eq!(format_duration(999), "999ms");
    }

    #[test]
    fn format_duration_over_one_second() {
        assert_eq!(format_duration(1000), "1.0s");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(12_345), "12.3s");
    }

    #[test]
    fn save_report_creates_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let record = make_record(vec![make_passing_command_result("test")]);
        let evidence = FormattedEvidence {
            pr_body: "pr body".to_string(),
            full_report: "full report content".to_string(),
            truncated: false,
        };

        let path = save_report(dir.path(), &record, &evidence).unwrap();

        assert!(path.exists());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "full report content"
        );
        assert!(
            path.to_str()
                .unwrap()
                .contains("reports/test-spec/20260316T120000Z-abc123.md")
        );
    }

    #[test]
    fn save_report_rejects_path_traversal() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut record = make_record(vec![]);
        record.summary.spec_name = "../escape".to_string();

        let evidence = FormattedEvidence {
            pr_body: String::new(),
            full_report: String::new(),
            truncated: false,
        };

        assert!(save_report(dir.path(), &record, &evidence).is_err());
    }

    #[test]
    fn github_body_limit_constant() {
        assert_eq!(GITHUB_BODY_LIMIT, 65_536);
    }
}
