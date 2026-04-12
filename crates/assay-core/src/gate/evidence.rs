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

    // Assemble the full report (used for both disk persistence and as the
    // starting point for truncation — avoids a redundant second assembly).
    let full_report = assemble(&skeleton, &detail_sections, &footer);

    // Apply truncation for the PR body.
    let (pr_body, truncated) = truncate_to_fit(
        &full_report,
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

        // Escape pipe characters that would break the markdown table.
        let name = cr.criterion_name.replace('|', "\\|");

        // Bold the row for failures.
        if passed == Some(false) {
            let _ = writeln!(
                buf,
                "| {emoji} | **{name}** | **{enforcement_str}** | **{duration_str}** |",
            );
        } else {
            let _ = writeln!(
                buf,
                "| {emoji} | {name} | {enforcement_str} | {duration_str} |",
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

        // Escape pipe characters in criterion names for markdown safety.
        let name = cr.criterion_name.replace('|', "\\|");

        if result.passed {
            // Agent pass — collapsed.
            let _ = writeln!(section, "<details>");
            let _ = writeln!(section, "<summary>:white_check_mark: {name}</summary>\n",);
            write_agent_detail(&mut section, result);
            let _ = writeln!(section, "\n</details>\n");

            sections.push(DetailSection {
                content: section,
                priority: DetailPriority::AgentPass,
            });
        } else {
            // Failure — expanded.
            let _ = writeln!(section, "<details open>");
            let _ = writeln!(section, "<summary>:x: {name}</summary>\n");
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
        GateKind::EventCount {
            event_type,
            min,
            max,
        } => {
            let _ = writeln!(buf, "**Event type:** `{event_type}`");
            if let Some(min) = min {
                let _ = writeln!(buf, "**Min:** {min}");
            }
            if let Some(max) = max {
                let _ = writeln!(buf, "**Max:** {max}");
            }
            if !result.stderr.is_empty() {
                let _ = writeln!(buf, "\n**stderr:**\n```\n{}\n```", result.stderr);
            }
        }
        GateKind::NoToolErrors => {
            if !result.stderr.is_empty() {
                let _ = writeln!(buf, "\n**stderr:**\n```\n{}\n```", result.stderr);
            }
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
    full_report: &str,
    skeleton: &str,
    details: &[DetailSection],
    footer: &str,
    report_path: &Path,
    char_limit: usize,
) -> (String, bool) {
    // Check if the full report already fits.
    if full_report.len() <= char_limit {
        return (full_report.to_string(), false);
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

    // Absolute fallback: safe truncate at a char boundary (unreachable in practice).
    let mut result = format!("{skeleton}{footer}{notice}");
    if char_limit < result.len() {
        let boundary = (0..=char_limit)
            .rev()
            .find(|&i| result.is_char_boundary(i))
            .unwrap_or(0);
        result.truncate(boundary);
    }
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
            source: None,
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
            source: None,
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
            source: None,
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
            source: None,
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
                source: None,
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

        // Get the full report to measure sizes.
        let full_evidence =
            format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);
        let full_len = full_evidence.full_report.len();

        // The truncation notice adds ~100 bytes. We need a limit that:
        // - Is too small for full_report (forces truncation)
        // - Is large enough for skeleton + failure detail + footer + notice
        // Remove one agent pass (~200 bytes) worth, which should trigger removal
        // of agent passes while keeping the failure.
        let tight_limit = full_len - 200;
        let evidence = format_gate_evidence(&record, Path::new("report.md"), tight_limit);

        assert!(evidence.truncated);
        // At least one agent pass detail should be removed (lowest priority).
        // The failure detail should be preserved (highest priority).
        assert!(
            evidence.pr_body.contains("test failed"),
            "failure detail should be preserved"
        );
        assert!(evidence.pr_body.contains("truncated"));
    }

    #[test]
    fn truncation_enforces_byte_limit() {
        // Verify that pr_body.len() (byte count) stays within the limit
        // even when truncation is required.
        let results = vec![
            make_agent_pass_result("review-1"),
            make_agent_pass_result("review-2"),
            make_failing_command_result("build"),
        ];
        let record = make_record(results);
        let full = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Set a limit that forces truncation.
        let limit = full.full_report.len() - 50;
        let evidence = format_gate_evidence(&record, Path::new("report.md"), limit);

        assert!(evidence.truncated);
        assert!(
            evidence.pr_body.len() <= limit,
            "pr_body byte length {} exceeds limit {}",
            evidence.pr_body.len(),
            limit
        );
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
            source: None,
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

    // ── Additional helpers ────────────────────────────────────────────

    fn make_file_exists_pass(name: &str, path: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::FileExists {
                    path: path.to_string(),
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
            source: None,
        }
    }

    fn make_file_exists_fail(name: &str, path: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::FileExists {
                    path: path.to_string(),
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
            source: None,
        }
    }

    fn make_skipped(name: &str) -> CriterionResult {
        CriterionResult {
            criterion_name: name.to_string(),
            result: None,
            enforcement: Enforcement::Required,
            source: None,
        }
    }

    // ── Task 1: Formatting and detail section tests ───────────────────

    #[test]
    fn all_pass_deterministic_has_no_detail_sections() {
        let results = vec![
            make_passing_command_result("unit-tests"),
            make_passing_command_result("integration-tests"),
            make_passing_command_result("lint"),
        ];
        let record = make_record(results);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Table should have 3 rows with pass emoji.
        let pass_count = evidence.full_report.matches(":white_check_mark:").count();
        assert_eq!(pass_count, 3);

        // No detail sections for deterministic passes.
        assert!(!evidence.full_report.contains("<details"));
        assert!(!evidence.truncated);
    }

    #[test]
    fn mixed_results_produce_correct_structure() {
        let results = vec![
            make_passing_command_result("build"),
            make_failing_command_result("lint-check"),
            make_agent_pass_result("code-review"),
            make_agent_fail_result("quality-check"),
            make_skipped("perf-check"),
        ];
        let record = make_record(results);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);
        let report = &evidence.full_report;

        // Emoji counts: table + detail section summaries.
        // :white_check_mark: appears in table for build + code-review, AND in detail summary for code-review = 3
        assert_eq!(report.matches(":white_check_mark:").count(), 3);
        // :x: appears in table for lint-check + quality-check, AND in detail summaries for both = 4
        assert_eq!(report.matches(":x:").count(), 4);
        assert_eq!(report.matches(":fast_forward:").count(), 1); // perf-check

        // Failed command has expanded detail with stderr.
        assert!(report.contains("<details open>"));
        assert!(report.contains("test failed")); // stderr from lint-check

        // Agent pass has collapsed detail with evidence.
        assert!(report.contains("<details>\n<summary>:white_check_mark: code-review</summary>"));
        assert!(report.contains("Found implementation"));

        // Agent fail has expanded detail with reasoning.
        assert!(report.contains("<summary>:x: quality-check</summary>"));
        assert!(report.contains("Missing error handling"));
        assert!(report.contains("No try/catch blocks found"));

        // Deterministic pass (build) has NO detail section.
        assert!(!report.contains("<summary>:white_check_mark: build</summary>"));

        // Skipped has no detail section.
        assert!(!report.contains("<summary>:fast_forward: perf-check</summary>"));

        // Bold on failures.
        assert!(report.contains("**lint-check**"));
        assert!(report.contains("**quality-check**"));
    }

    #[test]
    fn detail_sections_have_blank_lines_for_github_rendering() {
        let results = vec![
            make_agent_pass_result("review"),
            make_failing_command_result("build"),
        ];
        let record = make_record(results);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);
        let report = &evidence.full_report;

        // Every <details> block must have blank line after <summary> and before </details>.
        for (i, line) in report.lines().enumerate() {
            if line.starts_with("<summary>") {
                // Next line must be empty (blank line after summary).
                let lines: Vec<&str> = report.lines().collect();
                assert!(
                    lines.get(i + 1).is_some_and(|l| l.is_empty()),
                    "Expected blank line after <summary> at line {i}: {line}"
                );
            }
        }

        // Verify blank line before </details>.
        assert!(
            report.contains("\n\n</details>"),
            "Expected blank line before </details>"
        );
    }

    #[test]
    fn enforcement_summary_shows_breakdown() {
        let results = vec![
            make_passing_command_result("build"),
            make_failing_command_result("lint"),
        ];
        let mut record = make_record(results);
        record.summary.enforcement = EnforcementSummary {
            required_passed: 1,
            required_failed: 1,
            advisory_passed: 0,
            advisory_failed: 0,
        };

        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence
            .full_report
            .contains("**Enforcement:** 1 required passed, 1 required failed, 0 advisory passed, 0 advisory failed"));
    }

    #[test]
    fn file_exists_fail_detail_section_via_helper() {
        let record = make_record(vec![make_file_exists_fail("config-check", "config.toml")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("<details open>"));
        assert!(
            evidence
                .full_report
                .contains("**Missing file:** `config.toml`")
        );
        assert!(evidence.full_report.contains(":x:"));
    }

    #[test]
    fn file_exists_pass_has_no_detail_section() {
        let record = make_record(vec![make_file_exists_pass("readme", "README.md")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // FileExists pass is deterministic — no detail section.
        assert!(!evidence.full_report.contains("<details"));
        assert!(evidence.full_report.contains(":white_check_mark:"));
    }

    // ── Task 2: Truncation tests ─────────────────────────────────────

    #[test]
    fn no_truncation_within_limit() {
        let record = make_record(vec![make_passing_command_result("build")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(!evidence.truncated);
        assert_eq!(evidence.pr_body, evidence.full_report);
    }

    #[test]
    fn truncation_removes_failures_last() {
        let results = vec![
            make_failing_command_result("fail-1"),
            make_failing_command_result("fail-2"),
            make_failing_command_result("fail-3"),
            make_agent_pass_result("review"),
        ];
        let record = make_record(results);

        let full = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);
        let full_len = full.full_report.len();

        // Tight limit — should remove agent pass first, then start removing failures.
        // Set limit so agent pass must go but failures might fit.
        let tight = full_len - 10;
        let evidence = format_gate_evidence(&record, Path::new("report.md"), tight);

        assert!(evidence.truncated);
        // Agent pass detail should be removed first.
        assert!(!evidence.pr_body.contains("Found implementation"));
    }

    #[test]
    fn truncation_preserves_summary_table() {
        let results = vec![
            make_agent_pass_result("review-1"),
            make_agent_pass_result("review-2"),
            make_failing_command_result("build"),
        ];
        let record = make_record(results);

        // Very tight limit — only skeleton + footer should survive.
        let evidence = format_gate_evidence(&record, Path::new("report.md"), 600);

        assert!(evidence.truncated);
        // Summary table header must be present.
        assert!(evidence.pr_body.contains("| Status | Criterion |"));
        // Header must be present.
        assert!(evidence.pr_body.contains("## Gate Results:"));
    }

    #[test]
    fn full_report_is_never_truncated() {
        let results = vec![
            make_agent_pass_result("review-1"),
            make_agent_pass_result("review-2"),
            make_agent_pass_result("review-3"),
            make_failing_command_result("build"),
            make_failing_command_result("lint"),
        ];
        let record = make_record(results);

        // Full report at max limit.
        let full = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Full report at very tight limit.
        let tight = format_gate_evidence(&record, Path::new("report.md"), 100);

        // full_report must be identical regardless of char_limit.
        assert_eq!(full.full_report, tight.full_report);

        // full_report must contain all detail sections.
        assert!(tight.full_report.contains("review-1"));
        assert!(tight.full_report.contains("review-2"));
        assert!(tight.full_report.contains("review-3"));
        assert!(tight.full_report.contains("test failed")); // failure stderr
    }

    #[test]
    fn truncation_with_multibyte_utf8_uses_byte_length() {
        // Create a criterion with multi-byte chars in the name.
        let mut cr = make_agent_pass_result("résumé-chëck-日本語");
        cr.result.as_mut().unwrap().evidence = Some("Ünïcödë evidence — «test»".to_string());

        let record = make_record(vec![cr]);
        let full = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Verify we have multi-byte characters.
        assert!(full.full_report.len() > full.full_report.chars().count());

        // Force truncation using byte length just under full.
        let tight = full.full_report.len() - 1;
        let evidence = format_gate_evidence(&record, Path::new("report.md"), tight);

        assert!(evidence.truncated);
        // pr_body byte length must be within limit.
        assert!(evidence.pr_body.len() <= tight);
    }

    // ── Task 2: Persistence tests ────────────────────────────────────

    #[test]
    fn save_report_creates_directories() {
        let dir = tempfile::TempDir::new().unwrap();
        // reports dir does not exist yet.
        let reports_dir = dir.path().join("reports").join("test-spec");
        assert!(!reports_dir.exists());

        let record = make_record(vec![]);
        let evidence = FormattedEvidence {
            pr_body: String::new(),
            full_report: "the report".to_string(),
            truncated: false,
        };

        let path = save_report(dir.path(), &record, &evidence).unwrap();
        assert!(path.exists());
        assert!(reports_dir.exists());
    }

    #[test]
    fn save_report_content_matches_full_report() {
        let dir = tempfile::TempDir::new().unwrap();
        let record = make_record(vec![make_passing_command_result("test")]);
        let evidence = format_gate_evidence(
            &record,
            &dir.path().join("reports/test-spec/run.md"),
            GITHUB_BODY_LIMIT,
        );

        let path = save_report(dir.path(), &record, &evidence).unwrap();
        let saved_content = std::fs::read_to_string(&path).unwrap();

        assert_eq!(saved_content, evidence.full_report);
    }

    #[test]
    fn save_report_rejects_slash_in_run_id() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut record = make_record(vec![]);
        record.run_id = "evil/path".to_string();

        let evidence = FormattedEvidence {
            pr_body: String::new(),
            full_report: String::new(),
            truncated: false,
        };

        assert!(save_report(dir.path(), &record, &evidence).is_err());
    }

    // ── Review fix: missing test coverage ────────────────────────────

    #[test]
    fn always_pass_has_no_detail_section() {
        let cr = CriterionResult {
            criterion_name: "placeholder".to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::AlwaysPass,
                stdout: String::new(),
                stderr: String::new(),
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
            enforcement: Enforcement::Required,
            source: None,
        };
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(!evidence.full_report.contains("<details"));
    }

    #[test]
    fn always_pass_failure_shows_unexpected_message() {
        let cr = CriterionResult {
            criterion_name: "broken-placeholder".to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::AlwaysPass,
                stdout: String::new(),
                stderr: String::new(),
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
            enforcement: Enforcement::Required,
            source: None,
        };
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(
            evidence
                .full_report
                .contains("AlwaysPass gate reported failure (unexpected)")
        );
    }

    #[test]
    fn command_failure_with_both_stdout_and_stderr() {
        let cr = CriterionResult {
            criterion_name: "build".to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::Command {
                    cmd: "cargo build".to_string(),
                },
                stdout: "Compiling crate v0.1.0".to_string(),
                stderr: "error[E0308]: mismatched types".to_string(),
                exit_code: Some(101),
                duration_ms: 500,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
            source: None,
        };
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(evidence.full_report.contains("**stdout:**"));
        assert!(evidence.full_report.contains("Compiling crate v0.1.0"));
        assert!(evidence.full_report.contains("**stderr:**"));
        assert!(
            evidence
                .full_report
                .contains("error[E0308]: mismatched types")
        );
    }

    #[test]
    fn command_failure_with_no_exit_code() {
        let cr = CriterionResult {
            criterion_name: "killed".to_string(),
            result: Some(GateResult {
                passed: false,
                kind: GateKind::Command {
                    cmd: "long-running-test".to_string(),
                },
                stdout: String::new(),
                stderr: "killed by signal".to_string(),
                exit_code: None,
                duration_ms: 30_000,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
            source: None,
        };
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        assert!(!evidence.full_report.contains("**Exit code:**"));
        assert!(evidence.full_report.contains("killed by signal"));
    }

    #[test]
    fn agent_pass_with_all_none_fields() {
        let cr = CriterionResult {
            criterion_name: "sparse-review".to_string(),
            result: Some(GateResult {
                passed: true,
                kind: GateKind::AgentReport,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 10,
                timestamp: Utc::now(),
                truncated: false,
                original_bytes: None,
                evidence: None,
                reasoning: None,
                confidence: None,
                evaluator_role: None,
            }),
            enforcement: Enforcement::Required,
            source: None,
        };
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Should still produce a valid collapsed detail section.
        assert!(evidence.full_report.contains("<details>"));
        assert!(
            evidence
                .full_report
                .contains("<summary>:white_check_mark: sparse-review</summary>")
        );
        // No content fields.
        assert!(!evidence.full_report.contains("**Evidence:**"));
        assert!(!evidence.full_report.contains("**Reasoning:**"));
    }

    #[test]
    fn advisory_failure_row_is_bold() {
        let record = make_record(vec![make_agent_fail_result("style-check")]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // make_agent_fail_result uses Enforcement::Advisory.
        assert!(evidence.full_report.contains("**style-check**"));
        assert!(evidence.full_report.contains("**advisory**"));
    }

    #[test]
    fn pipe_in_criterion_name_does_not_break_table() {
        let mut cr = make_passing_command_result("foo | bar");
        cr.criterion_name = "foo | bar".to_string();
        let record = make_record(vec![cr]);
        let evidence = format_gate_evidence(&record, Path::new("report.md"), GITHUB_BODY_LIMIT);

        // Pipe should be escaped in table.
        assert!(evidence.full_report.contains(r"foo \| bar"));
        // Table row should still have exactly 5 pipe-delimited columns.
        let data_row = evidence
            .full_report
            .lines()
            .find(|l| l.contains(r"foo \| bar"))
            .expect("should find escaped row");
        // Count unescaped pipes (column delimiters).
        let unescaped_pipes = data_row
            .chars()
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|w| w[1] == '|' && w[0] != '\\')
            .count()
            + if data_row.starts_with('|') { 1 } else { 0 };
        assert_eq!(
            unescaped_pipes, 5,
            "table row should have 5 column delimiters"
        );
    }

    #[test]
    fn hard_truncate_fallback_safe_with_multibyte() {
        // Exercise the absolute fallback path with a limit that would land
        // mid-character. Should not panic.
        let mut cr = make_agent_pass_result("résumé");
        cr.result.as_mut().unwrap().evidence = Some("日本語テスト".to_string());
        let record = make_record(vec![cr]);

        // Limit of 1 forces the hard truncate fallback.
        let evidence = format_gate_evidence(&record, Path::new("report.md"), 1);

        assert!(evidence.truncated);
        assert!(evidence.pr_body.len() <= 1);
        // Should not panic — that's the main assertion.
    }
}
