//! Unit tests for `format_pr_section` and RunState new-field backward-compat.

use std::time::{SystemTime, UNIX_EPOCH};

use smelt_cli::commands::status::format_pr_section;
use smelt_core::forge::{CiStatus, PrState};
use smelt_core::monitor::{JobPhase, RunState};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Minimal RunState with no PR fields set.
fn make_state_no_pr() -> RunState {
    RunState {
        job_name: "test-job".into(),
        phase: JobPhase::Complete,
        container_id: None,
        sessions: vec![],
        started_at: now(),
        updated_at: now(),
        pid: std::process::id(),
        pr_url: None,
        pr_number: None,
        pr_status: None,
        ci_status: None,
        review_count: None,
        forge_repo: None,
        forge_token_env: None,
    }
}

/// RunState with pr_url set and optionally other status fields.
fn make_state_with_pr(
    pr_status: Option<PrState>,
    ci_status: Option<CiStatus>,
    review_count: Option<u32>,
) -> RunState {
    RunState {
        pr_url: Some("https://github.com/o/r/pull/42".into()),
        pr_number: Some(42),
        pr_status,
        ci_status,
        review_count,
        forge_repo: Some("o/r".into()),
        forge_token_env: Some("GITHUB_TOKEN".into()),
        ..make_state_no_pr()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// When there is no PR URL, format_pr_section must return None.
#[test]
fn test_format_pr_section_absent_when_no_url() {
    let state = make_state_no_pr();
    assert!(
        format_pr_section(&state).is_none(),
        "expected None when pr_url is None"
    );
}

/// When pr_url is set, the section must contain the URL.
#[test]
fn test_format_pr_section_shows_url() {
    let state = make_state_with_pr(None, None, None);
    let section = format_pr_section(&state).expect("expected Some when pr_url is set");
    assert!(
        section.contains("https://github.com/o/r/pull/42"),
        "section should contain the PR URL; got:\n{section}"
    );
}

/// When all status fields are populated, the section must contain their values.
#[test]
fn test_format_pr_section_shows_state_ci_reviews() {
    let state = make_state_with_pr(
        Some(PrState::Open),
        Some(CiStatus::Pending),
        Some(3),
    );
    let section = format_pr_section(&state).expect("expected Some");
    assert!(section.contains("Open"), "should contain 'Open'; got:\n{section}");
    assert!(section.contains("Pending"), "should contain 'Pending'; got:\n{section}");
    assert!(section.contains('3'), "should contain '3' for reviews; got:\n{section}");
}

/// When pr_url is set but all status fields are None, the section must
/// contain "unknown" for state and CI (not panic or omit the section).
#[test]
fn test_format_pr_section_shows_unknown_when_no_cached_status() {
    let state = make_state_with_pr(None, None, None);
    let section = format_pr_section(&state).expect("expected Some");
    // Must contain "unknown" at least twice (state + CI)
    let unknown_count = section.matches("unknown").count();
    assert!(
        unknown_count >= 2,
        "expected at least 2 'unknown' placeholders; got {unknown_count} in:\n{section}"
    );
}

/// Deserializing a TOML string that lacks the five new fields must succeed
/// and produce None for all five (backward-compat guarantee).
#[test]
fn test_run_state_new_fields_backward_compat() {
    let old_toml = r#"
job_name = "legacy-job"
phase = "complete"
sessions = ["s1"]
started_at = 1700000000
updated_at = 1700000060
pid = 42
"#;
    let state: RunState = toml::from_str(old_toml)
        .expect("RunState must deserialize without the five new fields");

    assert!(state.pr_status.is_none(), "pr_status should default to None");
    assert!(state.ci_status.is_none(), "ci_status should default to None");
    assert!(state.review_count.is_none(), "review_count should default to None");
    assert!(state.forge_repo.is_none(), "forge_repo should default to None");
    assert!(state.forge_token_env.is_none(), "forge_token_env should default to None");
}
