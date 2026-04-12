//! Integration tests for `assay_core::wizard::apply_gate_wizard`.

use std::fs;

use assay_core::wizard::apply_gate_wizard;
use assay_types::{CriterionInput, GateWizardInput, GatesSpec, SpecPreconditions};
use tempfile::TempDir;

fn full_input(slug: &str, overwrite: bool) -> GateWizardInput {
    GateWizardInput {
        slug: slug.to_string(),
        description: Some("My gate description".to_string()),
        extends: Some("base-gate".to_string()),
        include: vec!["security-lib".to_string(), "perf-lib".to_string()],
        criteria: vec![
            CriterionInput {
                name: "compiles".to_string(),
                description: "Code compiles without errors".to_string(),
                cmd: Some("cargo build".to_string()),
            },
            CriterionInput {
                name: "tests-pass".to_string(),
                description: "All tests pass".to_string(),
                cmd: Some("cargo test".to_string()),
            },
            CriterionInput {
                name: "reviewed".to_string(),
                description: "Code reviewed by a human".to_string(),
                cmd: None,
            },
        ],
        preconditions: Some(SpecPreconditions {
            requires: vec!["db-schema".to_string()],
            commands: vec!["docker ps".to_string()],
        }),
        overwrite,
    }
}

fn minimal_input(slug: &str) -> GateWizardInput {
    GateWizardInput {
        slug: slug.to_string(),
        description: None,
        extends: None,
        include: vec![],
        criteria: vec![],
        preconditions: None,
        overwrite: false,
    }
}

// ── Test 1: apply_gate_wizard_creates ────────────────────────────────────────

#[test]
fn apply_gate_wizard_creates() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = tmp.path().join("specs");

    let input = full_input("my-gate", false);
    let out = apply_gate_wizard(&input, &assay_dir, &specs_dir).unwrap();

    // File must exist at expected path.
    let expected_path = specs_dir.join("my-gate").join("gates.toml");
    assert!(
        out.path.exists(),
        "gates.toml should exist at {:?}",
        out.path
    );
    assert_eq!(out.path, expected_path);

    // Roundtrip: read back and verify all fields.
    let contents = fs::read_to_string(&out.path).unwrap();
    let roundtrip: GatesSpec = toml::from_str(&contents).expect("should parse as GatesSpec");

    assert_eq!(roundtrip.name, "my-gate");
    assert_eq!(roundtrip.description, "My gate description");
    assert_eq!(roundtrip.extends, Some("base-gate".to_string()));
    assert_eq!(roundtrip.include, vec!["security-lib", "perf-lib"]);
    assert_eq!(roundtrip.criteria.len(), 3);
    assert_eq!(roundtrip.criteria[0].name, "compiles");
    assert_eq!(roundtrip.criteria[0].cmd, Some("cargo build".to_string()));
    assert_eq!(roundtrip.criteria[2].cmd, None);

    let preconds = roundtrip
        .preconditions
        .as_ref()
        .expect("preconditions must be set");
    assert_eq!(preconds.requires, vec!["db-schema"]);
    assert_eq!(preconds.commands, vec!["docker ps"]);
}

// ── Test 2: apply_gate_wizard_collision ──────────────────────────────────────

#[test]
fn apply_gate_wizard_collision() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = tmp.path().join("specs");

    // Create the gate first.
    let input = full_input("my-gate", false);
    apply_gate_wizard(&input, &assay_dir, &specs_dir).expect("first call should succeed");

    // Read original content to verify it doesn't change.
    let original = fs::read_to_string(specs_dir.join("my-gate").join("gates.toml")).unwrap();

    // Second call with overwrite=false should fail.
    let err = apply_gate_wizard(&input, &assay_dir, &specs_dir)
        .expect_err("second call with overwrite=false should fail");

    // Error must be Io with AlreadyExists kind.
    match err {
        assay_core::AssayError::Io { ref source, .. } => {
            assert_eq!(
                source.kind(),
                std::io::ErrorKind::AlreadyExists,
                "source kind must be AlreadyExists"
            );
        }
        other => panic!("expected AssayError::Io, got: {other:?}"),
    }

    // File must be unchanged.
    let after = fs::read_to_string(specs_dir.join("my-gate").join("gates.toml")).unwrap();
    assert_eq!(
        original, after,
        "file contents must not change on collision"
    );
}

// ── Test 3: apply_gate_wizard_edit_overwrites ────────────────────────────────

#[test]
fn apply_gate_wizard_edit_overwrites() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = tmp.path().join("specs");

    // First write.
    let input = full_input("my-gate", false);
    apply_gate_wizard(&input, &assay_dir, &specs_dir).expect("first write should succeed");

    // Overwrite with different description.
    let mut new_input = full_input("my-gate", true);
    new_input.description = Some("Updated description".to_string());
    new_input.criteria = vec![CriterionInput {
        name: "only-criterion".to_string(),
        description: "The only one now".to_string(),
        cmd: None,
    }];

    let out =
        apply_gate_wizard(&new_input, &assay_dir, &specs_dir).expect("overwrite should succeed");

    let contents = fs::read_to_string(&out.path).unwrap();
    let roundtrip: GatesSpec = toml::from_str(&contents).unwrap();

    assert_eq!(roundtrip.description, "Updated description");
    assert_eq!(
        roundtrip.criteria.len(),
        1,
        "old content must be fully replaced"
    );
    assert_eq!(roundtrip.criteria[0].name, "only-criterion");
}

// ── Test 7: empty_criteria_allowed ───────────────────────────────────────────

#[test]
fn empty_criteria_allowed() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = tmp.path().join("specs");

    let input = minimal_input("template-gate");
    let out = apply_gate_wizard(&input, &assay_dir, &specs_dir).unwrap();

    assert!(out.path.exists());
    let contents = fs::read_to_string(&out.path).unwrap();
    let roundtrip: GatesSpec = toml::from_str(&contents).unwrap();
    assert_eq!(roundtrip.criteria.len(), 0);
}

// ── Test 8: output_roundtrip ─────────────────────────────────────────────────

#[test]
fn output_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = tmp.path().join("specs");

    let input = full_input("roundtrip-gate", false);
    let out = apply_gate_wizard(&input, &assay_dir, &specs_dir).unwrap();

    // Re-read the file and compare with the in-memory spec returned.
    let contents = fs::read_to_string(&out.path).unwrap();
    let from_disk: GatesSpec = toml::from_str(&contents).unwrap();

    assert_eq!(out.spec.name, from_disk.name);
    assert_eq!(out.spec.description, from_disk.description);
    assert_eq!(out.spec.extends, from_disk.extends);
    assert_eq!(out.spec.include, from_disk.include);
    assert_eq!(out.spec.criteria.len(), from_disk.criteria.len());
    assert_eq!(out.spec.preconditions, from_disk.preconditions);
}
