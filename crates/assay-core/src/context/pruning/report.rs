//! Dry-run report formatting for pruning results.
//!
//! Produces human-readable summaries showing per-strategy savings,
//! sample removals, and aggregate totals.

use std::fmt::Write;

use assay_types::context::PruneReport;

/// Format a pruning report as a human-readable string.
///
/// When `_color` is true, ANSI color codes may be included (deferred).
pub fn format_dry_run_report(report: &PruneReport, _color: bool) -> String {
    let mut out = String::new();

    // Header
    writeln!(out, "Session: {}", report.session_id).unwrap();
    writeln!(
        out,
        "Original: {} ({} entries)",
        format_bytes(report.original_size),
        report.original_entries
    )
    .unwrap();

    if report.strategies.is_empty() {
        writeln!(out).unwrap();
        writeln!(out, "No strategies applied").unwrap();
        writeln!(out).unwrap();
        write_mode_line(&mut out, report);
        return out;
    }

    writeln!(out, "Strategies: {}", report.strategies.len()).unwrap();
    writeln!(out).unwrap();

    // Per-strategy details
    for summary in &report.strategies {
        writeln!(out, "Strategy: {}", summary.strategy.label()).unwrap();
        writeln!(out, "  Lines removed: {}", summary.lines_removed).unwrap();
        let pct = if report.original_size > 0 {
            (summary.bytes_saved as f64 / report.original_size as f64) * 100.0
        } else {
            0.0
        };
        writeln!(
            out,
            "  Bytes saved: {} ({:.1}%)",
            format_number(summary.bytes_saved),
            pct
        )
        .unwrap();
        if summary.protected_skipped > 0 {
            writeln!(out, "  Protected skipped: {}", summary.protected_skipped).unwrap();
        }
        if summary.lines_modified > 0 {
            writeln!(out, "  Lines modified: {}", summary.lines_modified).unwrap();
        }
        if !summary.samples.is_empty() {
            writeln!(out, "  Samples:").unwrap();
            for sample in &summary.samples {
                writeln!(
                    out,
                    "    Line {}: {} ({} bytes)",
                    sample.line_number, sample.description, sample.bytes
                )
                .unwrap();
            }
            if summary.lines_removed > summary.samples.len() {
                writeln!(
                    out,
                    "    ...and {} more",
                    summary.lines_removed - summary.samples.len()
                )
                .unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    // Aggregate summary
    let total_bytes_saved: u64 = report.strategies.iter().map(|s| s.bytes_saved).sum();
    let total_lines_removed: usize = report.strategies.iter().map(|s| s.lines_removed).sum();
    let total_lines_modified: usize = report.strategies.iter().map(|s| s.lines_modified).sum();
    let total_pct = if report.original_size > 0 {
        (total_bytes_saved as f64 / report.original_size as f64) * 100.0
    } else {
        0.0
    };

    writeln!(out, "Summary:").unwrap();
    writeln!(
        out,
        "  Total bytes saved: {} ({:.1}%)",
        format_number(total_bytes_saved),
        total_pct
    )
    .unwrap();
    writeln!(out, "  Total lines removed: {}", total_lines_removed).unwrap();
    if total_lines_modified > 0 {
        writeln!(out, "  Total lines modified: {}", total_lines_modified).unwrap();
    }
    writeln!(
        out,
        "  Final size: {} ({} entries)",
        format_bytes(report.final_size),
        report.final_entries
    )
    .unwrap();
    write_mode_line(&mut out, report);

    out
}

fn write_mode_line(out: &mut String, report: &PruneReport) {
    if report.executed {
        writeln!(out, "  Mode: executed").unwrap();
    } else {
        writeln!(out, "  Mode: dry-run (use --execute to apply)").unwrap();
    }
}

/// Format a byte count as human-readable (e.g., "1.2 MB", "45.3 KB").
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

/// Format a number with comma separators.
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{PruneSample, PruneStrategy, PruneSummary};

    fn make_report(strategies: Vec<PruneSummary>, executed: bool) -> PruneReport {
        let total_removed: u64 = strategies.iter().map(|s| s.bytes_saved).sum();
        PruneReport {
            session_id: "test-session".into(),
            original_size: 100_000,
            final_size: 100_000 - total_removed,
            original_entries: 500,
            final_entries: 500
                - strategies.iter().map(|s| s.lines_removed).sum::<usize>(),
            strategies,
            executed,
        }
    }

    fn make_summary(
        strategy: PruneStrategy,
        lines_removed: usize,
        bytes_saved: u64,
        protected_skipped: usize,
        samples: Vec<PruneSample>,
    ) -> PruneSummary {
        PruneSummary {
            strategy,
            lines_removed,
            lines_modified: 0,
            bytes_saved,
            protected_skipped,
            samples,
        }
    }

    #[test]
    fn report_zero_strategies_shows_no_strategies() {
        let report = make_report(vec![], false);
        let output = format_dry_run_report(&report, false);
        assert!(output.contains("No strategies applied"));
    }

    #[test]
    fn report_one_strategy_shows_details() {
        let report = make_report(
            vec![make_summary(
                PruneStrategy::ProgressCollapse,
                42,
                12_450,
                2,
                vec![],
            )],
            false,
        );
        let output = format_dry_run_report(&report, false);
        assert!(output.contains("Progress collapse"), "Should contain strategy label");
        assert!(output.contains("42"), "Should contain lines removed");
        assert!(output.contains("12,450") || output.contains("12450"), "Should contain bytes saved");
    }

    #[test]
    fn report_with_samples_shows_up_to_3() {
        let samples = vec![
            PruneSample {
                line_number: 15,
                description: "Progress tick".into(),
                bytes: 298,
            },
            PruneSample {
                line_number: 23,
                description: "Progress tick".into(),
                bytes: 301,
            },
            PruneSample {
                line_number: 47,
                description: "Progress tick".into(),
                bytes: 295,
            },
        ];
        let report = make_report(
            vec![make_summary(
                PruneStrategy::ProgressCollapse,
                42,
                12_450,
                0,
                samples,
            )],
            false,
        );
        let output = format_dry_run_report(&report, false);
        assert!(output.contains("Line 15"), "Should show sample line numbers");
        assert!(output.contains("Line 23"));
        assert!(output.contains("Line 47"));
    }

    #[test]
    fn report_with_many_removals_shows_and_n_more() {
        let samples = vec![
            PruneSample {
                line_number: 1,
                description: "Progress tick".into(),
                bytes: 100,
            },
            PruneSample {
                line_number: 2,
                description: "Progress tick".into(),
                bytes: 100,
            },
            PruneSample {
                line_number: 3,
                description: "Progress tick".into(),
                bytes: 100,
            },
        ];
        let report = make_report(
            vec![make_summary(
                PruneStrategy::ProgressCollapse,
                42,
                12_450,
                0,
                samples,
            )],
            false,
        );
        let output = format_dry_run_report(&report, false);
        assert!(
            output.contains("...and 39 more"),
            "Should show remaining count: {output}"
        );
    }

    #[test]
    fn report_shows_aggregate_totals() {
        let report = make_report(
            vec![
                make_summary(PruneStrategy::ProgressCollapse, 30, 10_000, 0, vec![]),
                make_summary(PruneStrategy::MetadataStrip, 12, 5_000, 0, vec![]),
            ],
            false,
        );
        let output = format_dry_run_report(&report, false);
        assert!(output.contains("Summary"), "Should have summary section");
        assert!(
            output.contains("15,000") || output.contains("15000"),
            "Should show total bytes saved: {output}"
        );
    }

    #[test]
    fn report_shows_protected_skipped() {
        let report = make_report(
            vec![make_summary(
                PruneStrategy::ProgressCollapse,
                10,
                5_000,
                3,
                vec![],
            )],
            false,
        );
        let output = format_dry_run_report(&report, false);
        assert!(
            output.contains("3"),
            "Should show protected skipped count"
        );
    }

    #[test]
    fn report_dry_run_mode_indicator() {
        let report = make_report(vec![], false);
        let output = format_dry_run_report(&report, false);
        assert!(
            output.contains("dry-run"),
            "Should indicate dry-run mode: {output}"
        );
    }

    #[test]
    fn report_executed_mode_indicator() {
        let report = make_report(vec![], true);
        let output = format_dry_run_report(&report, false);
        assert!(
            !output.contains("dry-run") || output.contains("executed"),
            "Should indicate executed mode: {output}"
        );
    }
}
