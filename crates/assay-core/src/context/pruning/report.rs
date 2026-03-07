//! Dry-run report formatting for pruning results.
//!
//! Produces human-readable summaries showing per-strategy savings,
//! sample removals, and aggregate totals.

use assay_types::context::PruneReport;

/// Format a pruning report as a human-readable string.
///
/// When `_color` is true, ANSI color codes may be included (deferred).
pub fn format_dry_run_report(_report: &PruneReport, _color: bool) -> String {
    todo!("Implemented in GREEN phase")
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
        // Should contain summary section with totals
        assert!(output.contains("Summary"), "Should have summary section");
        // Total bytes saved
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
        // Should NOT say dry-run when executed
        assert!(
            !output.contains("dry-run") || output.contains("executed"),
            "Should indicate executed mode: {output}"
        );
    }
}
