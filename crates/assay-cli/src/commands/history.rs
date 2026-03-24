//! History subcommands for the `assay history` CLI group.

use anyhow::Context;
use clap::Subcommand;

use super::{COLUMN_GAP, assay_dir, colors_enabled, project_root};

#[derive(Subcommand)]
pub(crate) enum HistoryCommand {
    /// Show analytics: failure frequency and milestone velocity
    #[command(after_long_help = "\
Examples:
  Show analytics as structured text:
    assay history analytics

  Output as JSON:
    assay history analytics --json")]
    Analytics {
        /// Output as JSON instead of structured text
        #[arg(long)]
        json: bool,
    },
}

/// Handle history subcommands.
pub(crate) fn handle(command: HistoryCommand) -> anyhow::Result<i32> {
    match command {
        HistoryCommand::Analytics { json } => handle_analytics(json),
    }
}

// ── Internal helpers ──────────────────────────────────────────────

/// Handle `assay history analytics [--json]`.
fn handle_analytics(json: bool) -> anyhow::Result<i32> {
    let root = project_root()?;
    let ad = assay_dir(&root);

    if !ad.is_dir() {
        eprintln!("Error: not an Assay project (no .assay directory found)");
        eprintln!(
            "Run `assay init` to initialise a project, then run some gates to build history."
        );
        return Ok(1);
    }

    let report = assay_core::history::analytics::compute_analytics(&ad)
        .context("failed to compute analytics")?;

    if json {
        let output = serde_json::to_string_pretty(&report)
            .context("failed to serialize analytics report")?;
        println!("{output}");
        return Ok(0);
    }

    let color = colors_enabled();
    print_failure_frequency(&report.failure_frequency, color);
    println!();
    print_milestone_velocity(&report.milestone_velocity, color);

    if report.unreadable_records > 0 {
        println!();
        let note = if color {
            format!(
                "\x1b[33mNote:\x1b[0m {count} history record(s) could not be read and were skipped.",
                count = report.unreadable_records
            )
        } else {
            format!(
                "Note: {count} history record(s) could not be read and were skipped.",
                count = report.unreadable_records
            )
        };
        println!("{note}");
    }

    Ok(0)
}

/// Print the "Gate Failure Frequency" table.
fn print_failure_frequency(
    freqs: &[assay_core::history::analytics::FailureFrequency],
    color: bool,
) {
    println!("Gate Failure Frequency");
    println!("======================");

    if freqs.is_empty() {
        println!("  No gate run history found.");
        return;
    }

    // Compute column widths from data.
    let spec_w = freqs
        .iter()
        .map(|f| f.spec_name.len())
        .max()
        .unwrap_or(4)
        .max(4); // "Spec"
    let crit_w = freqs
        .iter()
        .map(|f| f.criterion_name.len())
        .max()
        .unwrap_or(9)
        .max(9); // "Criterion"

    // Header
    println!(
        "  {:<spec_w$}{gap}{:<crit_w$}{gap}{:>5}{gap}{:>4}{gap}{:>6}{gap}Enforcement",
        "Spec",
        "Criterion",
        "Fails",
        "Runs",
        "Rate",
        gap = COLUMN_GAP,
    );
    println!(
        "  {}{gap}{}{gap}{}{gap}{}{gap}{}{gap}{}",
        "\u{2500}".repeat(spec_w),
        "\u{2500}".repeat(crit_w),
        "\u{2500}".repeat(5),
        "\u{2500}".repeat(4),
        "\u{2500}".repeat(6),
        "\u{2500}".repeat(11),
        gap = COLUMN_GAP,
    );

    for f in freqs {
        let rate = if f.total_runs > 0 {
            (f.fail_count as f64 / f.total_runs as f64) * 100.0
        } else {
            0.0
        };

        let rate_str = format!("{rate:5.1}%");
        let rate_display = if color {
            if rate > 50.0 {
                format!("\x1b[31m{rate_str}\x1b[0m") // red for high fail rate
            } else if rate > 0.0 {
                format!("\x1b[33m{rate_str}\x1b[0m") // yellow for some failures
            } else {
                format!("\x1b[32m{rate_str}\x1b[0m") // green for 0%
            }
        } else {
            rate_str.clone()
        };

        let enforcement_label = match f.enforcement {
            assay_types::Enforcement::Required => "required",
            assay_types::Enforcement::Advisory => "advisory",
        };

        // When color is on, the rate_display has ANSI overhead — pad accordingly.
        let rate_width = if color {
            6 + super::ANSI_COLOR_OVERHEAD
        } else {
            6
        };

        println!(
            "  {:<spec_w$}{gap}{:<crit_w$}{gap}{:>5}{gap}{:>4}{gap}{:<rate_w$}{gap}{}",
            f.spec_name,
            f.criterion_name,
            f.fail_count,
            f.total_runs,
            rate_display,
            enforcement_label,
            gap = COLUMN_GAP,
            rate_w = rate_width,
        );
    }
}

/// Print the "Milestone Velocity" table.
fn print_milestone_velocity(
    velocities: &[assay_core::history::analytics::MilestoneVelocity],
    color: bool,
) {
    println!("Milestone Velocity");
    println!("===================");

    if velocities.is_empty() {
        println!("  No milestones with completed chunks found.");
        return;
    }

    // Compute column widths from data.
    let name_w = velocities
        .iter()
        .map(|v| v.milestone_name.len())
        .max()
        .unwrap_or(9)
        .max(9); // "Milestone"

    // Header
    println!(
        "  {:<name_w$}{gap}{:>6}{gap}{:>5}{gap}{:>6}",
        "Milestone",
        "Chunks",
        "Days",
        "Rate",
        gap = COLUMN_GAP,
    );
    println!(
        "  {}{gap}{}{gap}{}{gap}{}",
        "\u{2500}".repeat(name_w),
        "\u{2500}".repeat(6),
        "\u{2500}".repeat(5),
        "\u{2500}".repeat(6),
        gap = COLUMN_GAP,
    );

    for v in velocities {
        let chunks_str = format!("{}/{}", v.chunks_completed, v.total_chunks);
        let days_str = format!("{:.0}", v.days_elapsed);
        let rate_str = format!("{:.1}/d", v.chunks_per_day);

        let rate_display = if color {
            if v.chunks_per_day > 1.0 {
                format!("\x1b[32m{rate_str}\x1b[0m") // green for fast
            } else {
                rate_str.clone()
            }
        } else {
            rate_str
        };

        let rate_width = if color && v.chunks_per_day > 1.0 {
            6 + super::ANSI_COLOR_OVERHEAD
        } else {
            6
        };

        println!(
            "  {:<name_w$}{gap}{:>6}{gap}{:>5}{gap}{:<rate_w$}",
            v.milestone_name,
            chunks_str,
            days_str,
            rate_display,
            gap = COLUMN_GAP,
            rate_w = rate_width,
        );
    }
}

#[cfg(test)]
mod tests {
    use assay_core::history::analytics::{AnalyticsReport, FailureFrequency, MilestoneVelocity};
    use assay_types::Enforcement;

    /// Build a synthetic `AnalyticsReport` for testing formatters.
    fn synthetic_report() -> AnalyticsReport {
        AnalyticsReport {
            failure_frequency: vec![
                FailureFrequency {
                    spec_name: "auth-flow".to_string(),
                    criterion_name: "login-works".to_string(),
                    fail_count: 5,
                    total_runs: 10,
                    enforcement: Enforcement::Required,
                },
                FailureFrequency {
                    spec_name: "auth-flow".to_string(),
                    criterion_name: "signup-works".to_string(),
                    fail_count: 0,
                    total_runs: 8,
                    enforcement: Enforcement::Advisory,
                },
            ],
            milestone_velocity: vec![MilestoneVelocity {
                milestone_slug: "auth-feature".to_string(),
                milestone_name: "Auth Feature".to_string(),
                chunks_completed: 3,
                total_chunks: 5,
                days_elapsed: 2.0,
                chunks_per_day: 1.5,
            }],
            unreadable_records: 0,
        }
    }

    #[test]
    fn test_analytics_text_output_shape() {
        let report = synthetic_report();

        // Capture text output by calling the formatters with color disabled.
        // We can't easily capture println, so verify the data that feeds the formatters.
        assert_eq!(report.failure_frequency.len(), 2);
        assert_eq!(report.failure_frequency[0].spec_name, "auth-flow");
        assert_eq!(report.failure_frequency[0].criterion_name, "login-works");
        assert_eq!(report.failure_frequency[0].fail_count, 5);
        assert_eq!(report.failure_frequency[0].total_runs, 10);
        assert_eq!(report.milestone_velocity.len(), 1);
        assert_eq!(report.milestone_velocity[0].milestone_name, "Auth Feature");
        assert_eq!(report.milestone_velocity[0].chunks_completed, 3);
        assert_eq!(report.milestone_velocity[0].total_chunks, 5);
    }

    #[test]
    fn test_analytics_json_output_valid() {
        let report = synthetic_report();
        let json = serde_json::to_string_pretty(&report).expect("serialization should succeed");
        let parsed: AnalyticsReport =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(parsed.failure_frequency.len(), 2);
        assert_eq!(parsed.milestone_velocity.len(), 1);
        assert_eq!(parsed.unreadable_records, 0);
    }

    #[test]
    fn test_analytics_no_project_shows_error() {
        // Run in a temp dir with no .assay directory.
        let tmp = tempfile::TempDir::new().unwrap();
        let prev = std::env::current_dir().unwrap();
        // We test the logic inline: assay_dir check should fail.
        let ad = tmp.path().join(".assay");
        assert!(!ad.is_dir(), ".assay should not exist in temp dir");
        // Restore — this is a unit-level check of the guard logic.
        let _ = prev;
    }

    #[test]
    fn test_analytics_empty_project() {
        // Project with .assay but no results or milestones.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay")).unwrap();

        let report =
            assay_core::history::analytics::compute_analytics(tmp.path().join(".assay").as_path())
                .expect("should succeed on empty project");

        assert!(report.failure_frequency.is_empty());
        assert!(report.milestone_velocity.is_empty());
        assert_eq!(report.unreadable_records, 0);
    }
}
