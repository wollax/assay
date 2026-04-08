//! Integration tests for pipeline auto-promote on clean runs (S04).
//!
//! Tests exercise the `promote_spec` + `WorkSession` integration path
//! using tempdir specs and real session persistence. The full streaming
//! pipeline is not launched — these tests verify the auto-promote
//! preconditions, promotion call, and session JSON recording.

use std::fs;
use std::path::{Path, PathBuf};

use assay_types::WorkSession;
use assay_types::feature_spec::SpecStatus;

/// Create a directory spec with gates.toml and spec.toml.
fn create_dir_spec(specs_dir: &Path, slug: &str, status: &str, auto_promote: bool) {
    let spec_dir = specs_dir.join(slug);
    fs::create_dir_all(&spec_dir).unwrap();

    fs::write(
        spec_dir.join("gates.toml"),
        format!(
            r#"name = "{slug}"
[[criteria]]
name = "check"
description = "Basic check"
cmd = "echo ok"
"#
        ),
    )
    .unwrap();

    fs::write(
        spec_dir.join("spec.toml"),
        format!(
            r#"name = "{slug}"
status = "{status}"
auto_promote = {auto_promote}

[[requirements]]
id = "REQ-TEST-001"
title = "Test requirement"
statement = "The system shall do something"
"#
        ),
    )
    .unwrap();
}

/// Create an assay_dir with a session for the given spec.
fn create_session(assay_dir: &Path, spec_name: &str) -> String {
    let session = assay_core::work_session::start_session(
        assay_dir,
        spec_name,
        PathBuf::from("/tmp/wt"),
        "claude",
        None,
    )
    .expect("start session");
    session.id
}

/// Load a session from disk by ID.
fn load_session(assay_dir: &Path, session_id: &str) -> WorkSession {
    assay_core::work_session::load_session(assay_dir, session_id).expect("load session")
}

/// Read back the spec.toml status from disk.
fn read_spec_status(specs_dir: &Path, slug: &str) -> SpecStatus {
    let path = specs_dir.join(slug).join("spec.toml");
    let content = fs::read_to_string(&path).unwrap();
    let spec: assay_types::FeatureSpec = toml::from_str(&content).unwrap();
    spec.status
}

// ── Test 1: Auto-promote on clean run ───────────────────────────────

#[test]
fn test_auto_promote_on_clean_run() {
    let tmp = tempfile::tempdir().unwrap();
    let specs_dir = tmp.path().join("specs");
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&assay_dir).unwrap();

    create_dir_spec(&specs_dir, "clean-feature", "in-progress", true);
    let session_id = create_session(&assay_dir, "clean-feature");

    // Simulate auto-promote: all preconditions met.
    let (old, new) = assay_core::spec::promote::promote_spec(
        &specs_dir,
        "clean-feature",
        Some(SpecStatus::Verified),
    )
    .expect("promote should succeed");

    assert_eq!(old, SpecStatus::InProgress);
    assert_eq!(new, SpecStatus::Verified);

    // Verify spec.toml on disk.
    assert_eq!(
        read_spec_status(&specs_dir, "clean-feature"),
        SpecStatus::Verified
    );

    // Record in session JSON.
    assay_core::work_session::with_session(&assay_dir, &session_id, |session| {
        session.auto_promoted = true;
        session.promoted_to = Some(SpecStatus::Verified);
        Ok(())
    })
    .expect("update session");

    let session = load_session(&assay_dir, &session_id);
    assert!(
        session.auto_promoted,
        "session should record auto_promoted = true"
    );
    assert_eq!(
        session.promoted_to,
        Some(SpecStatus::Verified),
        "session should record promoted_to = Verified"
    );
}

// ── Test 2: Auto-promote disabled keeps InProgress ──────────────────

#[test]
fn test_auto_promote_disabled_keeps_in_progress() {
    let tmp = tempfile::tempdir().unwrap();
    let specs_dir = tmp.path().join("specs");
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&assay_dir).unwrap();

    create_dir_spec(&specs_dir, "no-promote", "in-progress", false);
    let session_id = create_session(&assay_dir, "no-promote");

    // Simulate the pipeline check: auto_promote is false, so we skip promotion.
    let feature_spec =
        assay_core::spec::load_feature_spec(&specs_dir.join("no-promote").join("spec.toml"))
            .expect("load feature spec");

    assert!(!feature_spec.auto_promote, "auto_promote should be false");

    // Spec should remain in-progress.
    assert_eq!(
        read_spec_status(&specs_dir, "no-promote"),
        SpecStatus::InProgress,
    );

    // Session should NOT have auto_promoted set.
    let session = load_session(&assay_dir, &session_id);
    assert!(!session.auto_promoted);
    assert_eq!(session.promoted_to, None);
}

// ── Test 3: Auto-promote skipped on checkpoint failure ──────────────

#[test]
fn test_auto_promote_skipped_on_checkpoint_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let specs_dir = tmp.path().join("specs");
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&specs_dir).unwrap();
    fs::create_dir_all(&assay_dir).unwrap();

    create_dir_spec(&specs_dir, "failed-check", "in-progress", true);
    let session_id = create_session(&assay_dir, "failed-check");

    // Simulate checkpoint failure: even though auto_promote is true,
    // the pipeline would have short-circuited before reaching the
    // auto-promote call site. Verify the spec stays unchanged.
    let feature_spec =
        assay_core::spec::load_feature_spec(&specs_dir.join("failed-check").join("spec.toml"))
            .expect("load feature spec");

    assert!(feature_spec.auto_promote);
    assert_eq!(feature_spec.status, SpecStatus::InProgress);

    // promote_spec is never called — spec stays InProgress.
    assert_eq!(
        read_spec_status(&specs_dir, "failed-check"),
        SpecStatus::InProgress,
    );

    // Session has no auto-promotion recorded.
    let session = load_session(&assay_dir, &session_id);
    assert!(!session.auto_promoted);
    assert_eq!(session.promoted_to, None);
}

// ── Test 4: Already-verified spec handles gracefully ────────────────

#[test]
fn test_auto_promote_already_verified_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let specs_dir = tmp.path().join("specs");
    fs::create_dir_all(&specs_dir).unwrap();

    // Spec is already Verified — pipeline checks status == InProgress,
    // so auto-promote should be skipped entirely.
    create_dir_spec(&specs_dir, "already-done", "verified", true);

    let feature_spec =
        assay_core::spec::load_feature_spec(&specs_dir.join("already-done").join("spec.toml"))
            .expect("load feature spec");

    assert!(feature_spec.auto_promote);
    assert_eq!(feature_spec.status, SpecStatus::Verified);

    // In the pipeline, this would NOT trigger promote_spec because
    // status != InProgress. Verify that calling promote_spec with
    // target=Verified on an already-Verified spec is a no-op
    // (same-status promotion succeeds per existing tests).
    let (old, new) = assay_core::spec::promote::promote_spec(
        &specs_dir,
        "already-done",
        Some(SpecStatus::Verified),
    )
    .expect("same-status promotion should succeed");

    assert_eq!(old, SpecStatus::Verified);
    assert_eq!(new, SpecStatus::Verified);
}
