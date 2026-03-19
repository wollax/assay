//! Integration tests for `assay_core::milestone` — scan, load, and save.

use chrono::Utc;
use tempfile::TempDir;

use assay_core::milestone::{milestone_load, milestone_save, milestone_scan};
use assay_types::{ChunkRef, Milestone, MilestoneStatus};

fn make_milestone(slug: &str) -> Milestone {
    let now = Utc::now();
    Milestone {
        slug: slug.to_string(),
        name: format!("Milestone {slug}"),
        description: Some(format!("Description for {slug}")),
        status: MilestoneStatus::InProgress,
        chunks: vec![ChunkRef {
            slug: "auth-flow".to_string(),
            order: 1,
        }],
        depends_on: vec!["foundation".to_string()],
        pr_branch: Some(format!("feat/{slug}")),
        pr_base: Some("main".to_string()),
        created_at: now,
        updated_at: now,
    }
}

fn make_assay_dir(tmp: &TempDir) -> std::path::PathBuf {
    let assay_dir = tmp.path().join(".assay");
    std::fs::create_dir_all(&assay_dir).expect("create .assay dir");
    assay_dir
}

// ── Test 1: Save + Load round-trip ──────────────────────────────────────────

#[test]
fn test_milestone_save_and_load_roundtrip() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let original = make_milestone("my-feature");
    milestone_save(&assay_dir, &original).expect("save milestone");

    let loaded = milestone_load(&assay_dir, "my-feature").expect("load milestone");

    assert_eq!(loaded.slug, original.slug);
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.description, original.description);
    assert_eq!(loaded.status, original.status);
    assert_eq!(loaded.chunks.len(), original.chunks.len());
    assert_eq!(loaded.chunks[0].slug, original.chunks[0].slug);
    assert_eq!(loaded.chunks[0].order, original.chunks[0].order);
    assert_eq!(loaded.depends_on, original.depends_on);
    assert_eq!(loaded.pr_branch, original.pr_branch);
    assert_eq!(loaded.pr_base, original.pr_base);
    // DateTime precision: compare at second granularity since TOML RFC 3339 drops sub-second.
    assert_eq!(
        loaded.created_at.timestamp(),
        original.created_at.timestamp()
    );
    assert_eq!(
        loaded.updated_at.timestamp(),
        original.updated_at.timestamp()
    );
}

// ── Test 2: Scan returns empty vec for missing milestones dir ────────────────

#[test]
fn test_milestone_scan_empty_for_missing_dir() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);
    // Do NOT create .assay/milestones/ — it must be absent.

    let result = milestone_scan(&assay_dir).expect("scan should not error");
    assert!(
        result.is_empty(),
        "expected empty vec for missing milestones dir, got: {result:?}"
    );
}

// ── Test 3: Scan returns all saved milestones ────────────────────────────────

#[test]
fn test_milestone_scan_returns_all_milestones() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let alpha = make_milestone("alpha");
    let beta = make_milestone("beta");
    milestone_save(&assay_dir, &alpha).expect("save alpha");
    milestone_save(&assay_dir, &beta).expect("save beta");

    let mut result = milestone_scan(&assay_dir).expect("scan milestones");
    // Sorted by slug
    result.sort_by(|a, b| a.slug.cmp(&b.slug));

    let slugs: Vec<&str> = result.iter().map(|m| m.slug.as_str()).collect();
    assert_eq!(slugs, vec!["alpha", "beta"]);
}

// ── Test 4: Load returns Err with path info for nonexistent slug ─────────────

#[test]
fn test_milestone_load_error_for_missing_slug() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let err =
        milestone_load(&assay_dir, "nonexistent").expect_err("expected Err for nonexistent slug");

    let display = err.to_string();
    assert!(
        display.contains("nonexistent"),
        "error message should contain slug, got: {display}"
    );
    assert!(
        display.contains("reading milestone"),
        "error message should contain operation, got: {display}"
    );
}

// ── Test 5: Slug validation rejects path traversal ───────────────────────────

#[test]
fn test_milestone_slug_validation_rejects_traversal() {
    let tmp = TempDir::new().expect("create temp dir");
    let assay_dir = make_assay_dir(&tmp);

    let mut evil = make_milestone("evil");
    evil.slug = "../evil".to_string();

    let err = milestone_save(&assay_dir, &evil).expect_err("expected Err for traversal slug");

    let display = err.to_string();
    assert!(
        display.contains("../evil") || display.contains("milestone slug"),
        "error should mention the invalid slug, got: {display}"
    );
}
