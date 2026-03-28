//! Integration tests for `assay_core::wizard`.
//!
//! These tests define the expected API contract for the wizard module.
//! They will fail to compile until T02 implements `assay_core::wizard`.

use assay_core::milestone::{milestone_load, milestone_save};
use assay_core::wizard::{
    CriterionInput, WizardChunkInput, WizardInputs, create_from_inputs, create_spec_from_params,
};
use assay_types::{GatesSpec, Milestone, MilestoneStatus};
use chrono::Utc;
use tempfile::TempDir;

/// Helper: build a minimal `Milestone` for test setup.
fn make_milestone(slug: &str) -> Milestone {
    let now = Utc::now();
    Milestone {
        slug: slug.to_string(),
        name: format!("Test {slug}"),
        description: None,
        status: MilestoneStatus::Draft,
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
}

/// Helper: build `WizardInputs` with the given slug and chunks.
fn make_inputs(slug: &str, chunks: Vec<WizardChunkInput>) -> WizardInputs {
    WizardInputs {
        slug: slug.to_string(),
        name: format!("My {slug} Feature"),
        description: None,
        chunks,
    }
}

/// Helper: build a `WizardChunkInput` with a given slug and one dummy criterion.
fn one_criterion_chunk(slug: &str) -> WizardChunkInput {
    WizardChunkInput {
        slug: slug.to_string(),
        name: format!("Chunk {slug}"),
        criteria: vec![CriterionInput {
            name: "criterion-1".to_string(),
            description: String::new(),
            cmd: None,
        }],
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn wizard_create_from_inputs_writes_files() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    let inputs = make_inputs(
        "my-feature",
        vec![
            one_criterion_chunk("my-feature-chunk-1"),
            one_criterion_chunk("my-feature-chunk-2"),
        ],
    );

    let result = create_from_inputs(&inputs, &assay_dir, &specs_dir);
    assert!(
        result.is_ok(),
        "create_from_inputs should succeed: {:?}",
        result.err()
    );

    // Milestone file must exist and load successfully.
    let ms = milestone_load(&assay_dir, "my-feature");
    assert!(ms.is_ok(), "milestone_load should succeed: {:?}", ms.err());

    // Both chunk gates.toml files must exist and parse as GatesSpec.
    for chunk_slug in &["my-feature-chunk-1", "my-feature-chunk-2"] {
        let gates_path = specs_dir.join(chunk_slug).join("gates.toml");
        assert!(
            gates_path.exists(),
            "gates.toml should exist at {gates_path:?}"
        );
        let content = std::fs::read_to_string(&gates_path).unwrap();
        let parsed: Result<GatesSpec, _> = toml::from_str(&content);
        assert!(
            parsed.is_ok(),
            "gates.toml for {chunk_slug} should parse as GatesSpec: {:?}",
            parsed.err()
        );
    }
}

#[test]
fn wizard_create_from_inputs_sets_milestone_and_order_on_specs() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    let inputs = make_inputs(
        "my-feature",
        vec![
            one_criterion_chunk("my-feature-chunk-1"),
            one_criterion_chunk("my-feature-chunk-2"),
        ],
    );

    create_from_inputs(&inputs, &assay_dir, &specs_dir).unwrap();

    // First chunk: order 0
    let path1 = specs_dir.join("my-feature-chunk-1").join("gates.toml");
    let content1 = std::fs::read_to_string(&path1).unwrap();
    let spec1: GatesSpec = toml::from_str(&content1).unwrap();
    assert_eq!(
        spec1.milestone,
        Some("my-feature".to_string()),
        "chunk-1 gates.milestone should be 'my-feature'"
    );
    assert_eq!(spec1.order, Some(0), "chunk-1 gates.order should be 0");

    // Second chunk: order 1
    let path2 = specs_dir.join("my-feature-chunk-2").join("gates.toml");
    let content2 = std::fs::read_to_string(&path2).unwrap();
    let spec2: GatesSpec = toml::from_str(&content2).unwrap();
    assert_eq!(
        spec2.milestone,
        Some("my-feature".to_string()),
        "chunk-2 gates.milestone should be 'my-feature'"
    );
    assert_eq!(spec2.order, Some(1), "chunk-2 gates.order should be 1");
}

#[test]
fn wizard_slug_collision_returns_error() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    let inputs = make_inputs(
        "my-feature",
        vec![one_criterion_chunk("my-feature-chunk-1")],
    );

    // First call: must succeed.
    create_from_inputs(&inputs, &assay_dir, &specs_dir)
        .expect("first create_from_inputs should succeed");

    // Second call with same slug: must return an error.
    let result = create_from_inputs(&inputs, &assay_dir, &specs_dir);
    assert!(
        result.is_err(),
        "second create_from_inputs with same slug should return Err"
    );
}

#[test]
fn wizard_create_spec_patches_milestone() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    // Pre-create a milestone so create_spec_from_params can attach to it.
    let ms = make_milestone("my-feature");
    milestone_save(&assay_dir, &ms).expect("milestone_save should succeed");

    // Create a spec linked to the existing milestone.
    let result = create_spec_from_params(
        "new-chunk",
        "New Chunk",
        Some("my-feature"),
        &assay_dir,
        &specs_dir,
        vec![],
    );
    assert!(
        result.is_ok(),
        "create_spec_from_params should succeed: {:?}",
        result.err()
    );

    // Reload the milestone and verify chunks contains the new chunk slug.
    let updated = milestone_load(&assay_dir, "my-feature").unwrap();
    let chunk_slugs: Vec<&str> = updated.chunks.iter().map(|c| c.slug.as_str()).collect();
    assert!(
        chunk_slugs.contains(&"new-chunk"),
        "milestone chunks should contain 'new-chunk', got: {chunk_slugs:?}"
    );
}

#[test]
fn wizard_create_spec_rejects_nonexistent_milestone() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    // No milestone with slug "ghost" exists.
    let result = create_spec_from_params(
        "some-chunk",
        "Some Chunk",
        Some("ghost"),
        &assay_dir,
        &specs_dir,
        vec![],
    );
    assert!(
        result.is_err(),
        "create_spec_from_params with nonexistent milestone should return Err"
    );
}

#[test]
fn wizard_cmd_field_round_trips_to_gates_toml() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    let inputs = make_inputs(
        "cmd-test",
        vec![WizardChunkInput {
            slug: "cmd-chunk".to_string(),
            name: "Cmd Chunk".to_string(),
            criteria: vec![CriterionInput {
                name: "builds".to_string(),
                description: "Project compiles".to_string(),
                cmd: Some("cargo test".to_string()),
            }],
        }],
    );

    create_from_inputs(&inputs, &assay_dir, &specs_dir).unwrap();

    let gates_path = specs_dir.join("cmd-chunk").join("gates.toml");
    let content = std::fs::read_to_string(&gates_path).unwrap();
    let spec: GatesSpec = toml::from_str(&content).unwrap();

    assert_eq!(spec.criteria.len(), 1);
    let crit = &spec.criteria[0];
    assert_eq!(crit.name, "builds");
    assert_eq!(crit.description, "Project compiles");
    assert_eq!(
        crit.cmd,
        Some("cargo test".to_string()),
        "cmd should round-trip through gates.toml"
    );
}

#[test]
fn wizard_cmd_none_omits_cmd_from_gates_toml() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");

    let inputs = make_inputs(
        "no-cmd-test",
        vec![WizardChunkInput {
            slug: "no-cmd-chunk".to_string(),
            name: "No Cmd Chunk".to_string(),
            criteria: vec![CriterionInput {
                name: "reviewed".to_string(),
                description: "Code reviewed".to_string(),
                cmd: None,
            }],
        }],
    );

    create_from_inputs(&inputs, &assay_dir, &specs_dir).unwrap();

    let gates_path = specs_dir.join("no-cmd-chunk").join("gates.toml");
    let content = std::fs::read_to_string(&gates_path).unwrap();
    let spec: GatesSpec = toml::from_str(&content).unwrap();

    assert_eq!(spec.criteria.len(), 1);
    assert_eq!(
        spec.criteria[0].cmd, None,
        "cmd: None should produce no cmd field"
    );

    // Also verify the raw TOML doesn't contain a `cmd` key.
    assert!(
        !content.contains("cmd ="),
        "raw TOML should not contain 'cmd =' when cmd is None, got:\n{content}"
    );
}
