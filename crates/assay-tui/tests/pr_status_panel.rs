//! Integration tests for the TUI PR status panel.
//!
//! Exercises: event→state storage via `handle_pr_status_update`,
//! `poll_targets` population from milestone files, and refresh
//! after milestone reload via `handle_agent_done`.

use std::fs;

use assay_core::pr::{PrStatusInfo, PrStatusState};
use assay_tui::app::App;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Write a minimal milestone TOML to `<tmp>/.assay/milestones/<slug>.toml`.
fn write_milestone_toml(tmp: &TempDir, slug: &str, pr_number: Option<u64>) {
    let milestones_dir = tmp.path().join(".assay").join("milestones");
    fs::create_dir_all(&milestones_dir).expect("create milestones dir");

    let pr_line = match pr_number {
        Some(n) => format!("pr_number = {n}\n"),
        None => String::new(),
    };

    let now = "2026-01-01T00:00:00Z";
    let content = format!(
        r#"slug = "{slug}"
name = "Milestone {slug}"
status = "draft"
{pr_line}created_at = "{now}"
updated_at = "{now}"

[[chunks]]
slug = "chunk-a"
order = 1
"#
    );
    fs::write(milestones_dir.join(format!("{slug}.toml")), content).expect("write milestone toml");
}

// ── Test 1: handle_pr_status_update stores and overwrites info ───────────────

#[test]
fn test_handle_pr_status_update_stores_info() {
    let mut app = App::with_project_root(None).expect("App::with_project_root failed");

    let info = PrStatusInfo {
        state: PrStatusState::Open,
        ci_pass: 2,
        ci_fail: 0,
        ci_pending: 1,
        review_decision: "APPROVED".to_string(),
    };
    app.handle_pr_status_update("my-ms".into(), info);

    let stored = app.pr_statuses.get("my-ms").expect("should have entry");
    assert_eq!(stored.state, PrStatusState::Open);
    assert_eq!(stored.ci_pass, 2);
    assert_eq!(stored.ci_fail, 0);
    assert_eq!(stored.ci_pending, 1);
    assert_eq!(stored.review_decision, "APPROVED");

    // Overwrite with a different state
    let info2 = PrStatusInfo {
        state: PrStatusState::Merged,
        ci_pass: 5,
        ci_fail: 1,
        ci_pending: 0,
        review_decision: "CHANGES_REQUESTED".to_string(),
    };
    app.handle_pr_status_update("my-ms".into(), info2);

    let updated = app
        .pr_statuses
        .get("my-ms")
        .expect("should still have entry");
    assert_eq!(
        updated.state,
        PrStatusState::Merged,
        "state should be overwritten"
    );
    assert_eq!(updated.ci_pass, 5);
    assert_eq!(updated.ci_fail, 1);
    assert_eq!(updated.review_decision, "CHANGES_REQUESTED");
}

// ── Test 2: poll_targets populated from milestones ───────────────────────────

#[test]
fn test_poll_targets_populated_from_milestones() {
    let tmp = TempDir::new().expect("create temp dir");

    write_milestone_toml(&tmp, "ms-with-pr", Some(42));
    write_milestone_toml(&tmp, "ms-without-pr", None);

    let app = App::with_project_root(Some(tmp.path().to_path_buf()))
        .expect("App::with_project_root failed");

    let targets = app.poll_targets.lock().expect("lock poll_targets");
    assert_eq!(
        targets.len(),
        1,
        "only milestone with pr_number should be a poll target"
    );
    assert_eq!(targets[0].0, "ms-with-pr");
    assert_eq!(targets[0].1, 42);
}

// ── Test 3: poll_targets refreshed after milestone reload ────────────────────

#[test]
fn test_poll_targets_refreshed_after_milestone_reload() {
    let tmp = TempDir::new().expect("create temp dir");

    write_milestone_toml(&tmp, "first-ms", Some(10));

    let mut app = App::with_project_root(Some(tmp.path().to_path_buf()))
        .expect("App::with_project_root failed");

    {
        let targets = app.poll_targets.lock().expect("lock poll_targets");
        assert_eq!(targets.len(), 1, "should start with 1 poll target");
    }

    // Write a second milestone with pr_number to disk
    write_milestone_toml(&tmp, "second-ms", Some(20));

    // handle_agent_done refreshes milestones and poll_targets from disk
    app.handle_agent_done(0);

    let targets = app.poll_targets.lock().expect("lock poll_targets");
    assert_eq!(targets.len(), 2, "should have 2 poll targets after reload");

    // Verify both entries are present (order may vary due to fs scan)
    let slugs: Vec<&str> = targets.iter().map(|(s, _)| s.as_str()).collect();
    assert!(slugs.contains(&"first-ms"), "should contain first-ms");
    assert!(slugs.contains(&"second-ms"), "should contain second-ms");

    let first = targets.iter().find(|(s, _)| s == "first-ms").unwrap();
    assert_eq!(first.1, 10);
    let second = targets.iter().find(|(s, _)| s == "second-ms").unwrap();
    assert_eq!(second.1, 20);
}
