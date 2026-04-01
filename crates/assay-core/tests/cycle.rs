//! Integration tests for `assay_core::milestone::cycle` — cycle state machine.
//!
//! These tests drive T02 implementation. They are expected to fail to compile
//! until `assay_core::milestone::cycle` is created in T02.

use chrono::Utc;
use std::fs;
use tempfile::TempDir;

use assay_core::milestone::cycle::{
    active_chunk, cycle_advance, cycle_status, milestone_phase_transition,
};
use assay_core::milestone::{milestone_save, milestone_scan};
use assay_types::{ChunkRef, Milestone, MilestoneStatus};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_assay_dir(tmp: &TempDir) -> std::path::PathBuf {
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&assay_dir).expect("create .assay dir");
    assay_dir
}

fn make_milestone_with_status(
    slug: &str,
    status: MilestoneStatus,
    chunks: Vec<ChunkRef>,
) -> Milestone {
    let now = Utc::now();
    Milestone {
        slug: slug.to_string(),
        name: format!("Milestone {slug}"),
        description: None,
        status,
        chunks,
        completed_chunks: vec![],
        depends_on: vec![],
        pr_branch: None,
        pr_base: None,
        pr_number: None,
        pr_url: None,
        pr_labels: None,
        pr_reviewers: None,
        pr_body_template: None,
        created_at: now,
        updated_at: now,
    }
}

/// Create a minimal passing gates.toml spec for the given chunk slug inside the assay_dir.
fn create_passing_spec(assay_dir: &std::path::Path, chunk_slug: &str) {
    let spec_dir = assay_dir.join("specs").join(chunk_slug);
    fs::create_dir_all(&spec_dir).expect("create spec dir");
    let gates_toml = format!(
        "name = \"{chunk_slug}\"\n\n[[criteria]]\nname = \"pass\"\ndescription = \"always passes\"\ncmd = \"true\"\n"
    );
    fs::write(spec_dir.join("gates.toml"), gates_toml).expect("write gates.toml");
}

/// Create a minimal failing gates.toml spec for the given chunk slug inside the assay_dir.
fn create_failing_spec(assay_dir: &std::path::Path, chunk_slug: &str) {
    let spec_dir = assay_dir.join("specs").join(chunk_slug);
    fs::create_dir_all(&spec_dir).expect("create spec dir");
    let gates_toml = format!(
        "name = \"{chunk_slug}\"\n\n[[criteria]]\nname = \"fail\"\ndescription = \"always fails\"\ncmd = \"false\"\n"
    );
    fs::write(spec_dir.join("gates.toml"), gates_toml).expect("write gates.toml");
}

// ── Test 1: No milestones → cycle_status returns Ok(None) ────────────────────

#[test]
fn test_cycle_status_no_milestones() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let result = cycle_status(&assay_dir).expect("cycle_status should not error on empty dir");
    assert!(
        result.is_none(),
        "expected None with no milestones, got: {result:?}"
    );
}

// ── Test 2: Draft milestone → cycle_status returns Ok(None) ──────────────────

#[test]
fn test_cycle_status_draft_milestone() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let draft = make_milestone_with_status(
        "draft-ms",
        MilestoneStatus::Draft,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
            depends_on: vec![],
        }],
    );
    milestone_save(&assay_dir, &draft).expect("save draft milestone");

    let result = cycle_status(&assay_dir).expect("cycle_status should succeed");
    assert!(
        result.is_none(),
        "Draft milestone should not appear as active cycle, got: {result:?}"
    );
}

// ── Test 3: InProgress milestone with 2 chunks → correct CycleStatus ─────────

#[test]
fn test_cycle_status_in_progress() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let milestone = make_milestone_with_status(
        "active-ms",
        MilestoneStatus::InProgress,
        vec![
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
                depends_on: vec![],
            },
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
                depends_on: vec![],
            },
        ],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let status = cycle_status(&assay_dir)
        .expect("cycle_status should succeed")
        .expect("expected Some(CycleStatus) for InProgress milestone");

    assert_eq!(status.milestone_slug, "active-ms");
    assert_eq!(status.phase, MilestoneStatus::InProgress);
    assert_eq!(
        status.active_chunk_slug.as_deref(),
        Some("chunk-a"),
        "active chunk should be lowest-order chunk"
    );
    assert_eq!(status.completed_count, 0);
    assert_eq!(status.total_count, 2);
}

// ── Test 4: active_chunk returns the lowest-order chunk ──────────────────────

#[test]
fn test_active_chunk_sorted_by_order() {
    // No tempdir or disk I/O needed — active_chunk is a pure in-memory function.

    // Chunks stored with order=2 before order=1 (intentionally reversed insertion order).
    // active_chunk must sort by order and return chunk-a (order=1).
    let mut milestone = make_milestone_with_status(
        "order-test",
        MilestoneStatus::InProgress,
        vec![
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
                depends_on: vec![],
            },
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
                depends_on: vec![],
            },
        ],
    );

    let chunk = active_chunk(&milestone).expect("expected active chunk");
    assert_eq!(
        chunk.slug, "chunk-a",
        "active_chunk must return lowest-order chunk, got: {}",
        chunk.slug
    );
    assert_eq!(chunk.order, 1);

    // Mark chunk-a as completed — now chunk-b should be active
    milestone.completed_chunks.push("chunk-a".to_string());
    let next_chunk = active_chunk(&milestone).expect("expected second active chunk");
    assert_eq!(
        next_chunk.slug, "chunk-b",
        "after completing chunk-a, chunk-b should be active"
    );
}

// ── Test 5: cycle_advance marks active chunk complete and saves milestone ─────

#[test]
fn test_cycle_advance_marks_chunk_complete() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_passing_spec(&assay_dir, "chunk-a");
    let milestone = make_milestone_with_status(
        "advance-test",
        MilestoneStatus::InProgress,
        vec![
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
                depends_on: vec![],
            },
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
                depends_on: vec![],
            },
        ],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();
    let status = cycle_advance(&assay_dir, &specs_dir, &working_dir, None)
        .expect("cycle_advance should succeed with passing gate");

    // Active chunk has advanced
    assert_eq!(
        status.active_chunk_slug.as_deref(),
        Some("chunk-b"),
        "active chunk should be chunk-b after advancing past chunk-a"
    );
    assert_eq!(status.completed_count, 1);
    assert_eq!(status.total_count, 2);

    // Milestone persisted with completed_chunks
    let saved = milestone_scan(&assay_dir)
        .expect("scan milestones")
        .into_iter()
        .find(|m| m.slug == "advance-test")
        .expect("milestone should still exist");
    assert!(
        saved.completed_chunks.contains(&"chunk-a".to_string()),
        "completed_chunks should contain 'chunk-a', got: {:?}",
        saved.completed_chunks
    );
}

// ── Test 6: advance past last chunk moves milestone to Verify ─────────────────

#[test]
fn test_cycle_advance_all_chunks_move_to_verify() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_passing_spec(&assay_dir, "chunk-a");
    create_passing_spec(&assay_dir, "chunk-b");

    let milestone = make_milestone_with_status(
        "two-chunk-ms",
        MilestoneStatus::InProgress,
        vec![
            ChunkRef {
                slug: "chunk-a".to_string(),
                order: 1,
                depends_on: vec![],
            },
            ChunkRef {
                slug: "chunk-b".to_string(),
                order: 2,
                depends_on: vec![],
            },
        ],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    // Advance past chunk-a
    cycle_advance(&assay_dir, &specs_dir, &working_dir, None)
        .expect("first advance should succeed");

    // Advance past chunk-b (last chunk)
    let final_status = cycle_advance(&assay_dir, &specs_dir, &working_dir, None)
        .expect("second advance should succeed");

    assert_eq!(
        final_status.phase,
        MilestoneStatus::Verify,
        "milestone should transition to Verify after all chunks complete"
    );
    assert!(
        final_status.active_chunk_slug.is_none(),
        "no active chunk when all chunks complete"
    );
    assert_eq!(final_status.completed_count, 2);
    assert_eq!(final_status.total_count, 2);

    // Verify persisted status
    let saved = milestone_scan(&assay_dir)
        .expect("scan milestones")
        .into_iter()
        .find(|m| m.slug == "two-chunk-ms")
        .expect("milestone should exist");
    assert_eq!(saved.status, MilestoneStatus::Verify);
}

// ── Test 7: gates fail → Err, milestone unchanged ────────────────────────────

#[test]
fn test_cycle_advance_gates_fail_returns_error() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    create_failing_spec(&assay_dir, "chunk-fail");

    let milestone = make_milestone_with_status(
        "fail-test",
        MilestoneStatus::InProgress,
        vec![ChunkRef {
            slug: "chunk-fail".to_string(),
            order: 1,
            depends_on: vec![],
        }],
    );
    milestone_save(&assay_dir, &milestone).expect("save milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = cycle_advance(&assay_dir, &specs_dir, &working_dir, None);
    assert!(
        result.is_err(),
        "cycle_advance should return Err when required gates fail"
    );

    // Milestone must be unchanged
    let saved = milestone_scan(&assay_dir)
        .expect("scan milestones")
        .into_iter()
        .find(|m| m.slug == "fail-test")
        .expect("milestone should still exist");
    assert!(
        saved.completed_chunks.is_empty(),
        "completed_chunks should still be empty after gate failure, got: {:?}",
        saved.completed_chunks
    );
}

// ── Test 8: valid phase transitions ──────────────────────────────────────────

#[test]
fn test_milestone_phase_transition_valid() {
    let now = Utc::now();

    // Draft → InProgress (requires chunks)
    let mut m = Milestone {
        slug: "trans-test".to_string(),
        name: "Trans Test".to_string(),
        description: None,
        status: MilestoneStatus::Draft,
        chunks: vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
            depends_on: vec![],
        }],
        completed_chunks: vec![],
        depends_on: vec![],
        pr_branch: None,
        pr_base: None,
        pr_number: None,
        pr_url: None,
        pr_labels: None,
        pr_reviewers: None,
        pr_body_template: None,
        created_at: now,
        updated_at: now,
    };

    milestone_phase_transition(&mut m, MilestoneStatus::InProgress)
        .expect("Draft → InProgress should succeed when chunks present");
    assert_eq!(m.status, MilestoneStatus::InProgress);

    // InProgress → Verify (all chunks complete)
    m.completed_chunks.push("chunk-a".to_string());
    milestone_phase_transition(&mut m, MilestoneStatus::Verify)
        .expect("InProgress → Verify should succeed when all chunks complete");
    assert_eq!(m.status, MilestoneStatus::Verify);

    // Verify → Complete
    milestone_phase_transition(&mut m, MilestoneStatus::Complete)
        .expect("Verify → Complete should succeed");
    assert_eq!(m.status, MilestoneStatus::Complete);
}

// ── Test 9: invalid phase transitions return Err ─────────────────────────────

#[test]
fn test_milestone_phase_transition_invalid() {
    let now = Utc::now();

    let make = |status: MilestoneStatus| -> Milestone {
        Milestone {
            slug: "inv-test".to_string(),
            name: "Inv Test".to_string(),
            description: None,
            status,
            chunks: vec![],
            completed_chunks: vec![],
            depends_on: vec![],
            pr_branch: None,
            pr_base: None,
            pr_number: None,
            pr_url: None,
            pr_labels: None,
            pr_reviewers: None,
            pr_body_template: None,
            created_at: now,
            updated_at: now,
        }
    };

    // Verify → InProgress is not a valid forward transition
    let mut m = make(MilestoneStatus::Verify);
    let err = milestone_phase_transition(&mut m, MilestoneStatus::InProgress)
        .expect_err("Verify → InProgress should be invalid");
    let msg = err.to_string();
    assert!(
        msg.contains("verify")
            || msg.contains("in_progress")
            || msg.contains("invalid")
            || msg.contains("transition"),
        "error should describe the invalid transition, got: {msg}"
    );

    // Draft → Verify is not a valid transition
    let mut m2 = make(MilestoneStatus::Draft);
    let err2 = milestone_phase_transition(&mut m2, MilestoneStatus::Verify)
        .expect_err("Draft → Verify should be invalid");
    let msg2 = err2.to_string();
    assert!(
        msg2.contains("draft")
            || msg2.contains("verify")
            || msg2.contains("invalid")
            || msg2.contains("transition"),
        "error should describe the invalid transition, got: {msg2}"
    );

    // Draft → InProgress without chunks should fail
    let mut m3 = make(MilestoneStatus::Draft);
    let err3 = milestone_phase_transition(&mut m3, MilestoneStatus::InProgress)
        .expect_err("Draft → InProgress without chunks should fail");
    let msg3 = err3.to_string();
    assert!(
        msg3.contains("chunk") || msg3.contains("no chunks") || msg3.contains("precondition"),
        "error should mention missing chunks, got: {msg3}"
    );
}

// ── Test 10: no InProgress milestone → cycle_advance returns Err ──────────────

#[test]
fn test_cycle_advance_no_active_milestone() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    // Save a Draft milestone — should not be targetted by cycle_advance
    let draft = make_milestone_with_status(
        "draft-only",
        MilestoneStatus::Draft,
        vec![ChunkRef {
            slug: "chunk-a".to_string(),
            order: 1,
            depends_on: vec![],
        }],
    );
    milestone_save(&assay_dir, &draft).expect("save draft milestone");

    let specs_dir = assay_dir.join("specs");
    let working_dir = tmp.path().to_path_buf();

    let result = cycle_advance(&assay_dir, &specs_dir, &working_dir, None);
    assert!(
        result.is_err(),
        "cycle_advance should return Err when no InProgress milestone exists"
    );

    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("no active")
            || msg.contains("InProgress")
            || msg.contains("in_progress")
            || msg.contains("milestone"),
        "error should describe missing active milestone, got: {msg}"
    );
}
