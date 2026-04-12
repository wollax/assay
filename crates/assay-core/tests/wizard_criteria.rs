//! Integration tests for `assay_core::wizard::apply_criteria_wizard`.

use std::fs;

use assay_core::spec::compose;
use assay_core::wizard::apply_criteria_wizard;
use assay_types::{CriteriaLibrary, CriteriaWizardInput, CriterionInput};
use tempfile::TempDir;

fn full_input(name: &str, overwrite: bool) -> CriteriaWizardInput {
    CriteriaWizardInput {
        name: name.to_string(),
        description: "shared lib".to_string(),
        version: Some("0.1.0".to_string()),
        tags: vec!["rust".to_string()],
        criteria: vec![
            CriterionInput {
                name: "c1".to_string(),
                description: "d1".to_string(),
                cmd: Some("echo 1".to_string()),
            },
            CriterionInput {
                name: "c2".to_string(),
                description: "d2".to_string(),
                cmd: None,
            },
        ],
        overwrite,
    }
}

// ── Test 1: apply_criteria_wizard_creates ────────────────────────────────────

#[test]
fn apply_criteria_wizard_creates() {
    let dir = TempDir::new().unwrap();
    let assay_dir = dir.path();

    let input = full_input("my-lib", false);
    let out = apply_criteria_wizard(&input, assay_dir).unwrap();

    assert!(
        out.path.exists(),
        "library file should exist at {:?}",
        out.path
    );

    let contents = fs::read_to_string(&out.path).unwrap();
    let roundtrip: CriteriaLibrary =
        toml::from_str(&contents).expect("should parse as CriteriaLibrary");

    assert_eq!(roundtrip.name, "my-lib");
    assert_eq!(roundtrip.description, "shared lib");
    assert_eq!(roundtrip.version, Some("0.1.0".to_string()));
    assert_eq!(roundtrip.tags, vec!["rust"]);
    assert_eq!(roundtrip.criteria.len(), 2);
    assert_eq!(roundtrip.criteria[0].name, "c1");
    assert_eq!(roundtrip.criteria[0].cmd, Some("echo 1".to_string()));
    assert_eq!(roundtrip.criteria[1].name, "c2");
    assert_eq!(roundtrip.criteria[1].cmd, None);
}

// ── Test 2: apply_criteria_wizard_collision ──────────────────────────────────

#[test]
fn apply_criteria_wizard_collision() {
    let dir = TempDir::new().unwrap();
    let assay_dir = dir.path();

    // Create the library first.
    apply_criteria_wizard(&full_input("my-lib", false), assay_dir)
        .expect("first call should succeed");

    let original_path = assay_dir.join("criteria").join("my-lib.toml");
    let original = fs::read_to_string(&original_path).unwrap();

    // Second call with overwrite=false should fail.
    let err = apply_criteria_wizard(&full_input("my-lib", false), assay_dir)
        .expect_err("second call with overwrite=false should fail");

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
    let after = fs::read_to_string(&original_path).unwrap();
    assert_eq!(
        original, after,
        "file contents must not change on collision"
    );
}

// ── Test 3: apply_criteria_wizard_edit_overwrites ────────────────────────────

#[test]
fn apply_criteria_wizard_edit_overwrites() {
    let dir = TempDir::new().unwrap();
    let assay_dir = dir.path();

    // First write.
    apply_criteria_wizard(&full_input("my-lib", false), assay_dir)
        .expect("first write should succeed");

    // Overwrite with new content.
    let mut new_input = full_input("my-lib", true);
    new_input.description = "updated description".to_string();
    new_input.criteria = vec![CriterionInput {
        name: "only-one".to_string(),
        description: "The only criterion now".to_string(),
        cmd: None,
    }];

    let out = apply_criteria_wizard(&new_input, assay_dir).expect("overwrite should succeed");

    let contents = fs::read_to_string(&out.path).unwrap();
    let roundtrip: CriteriaLibrary = toml::from_str(&contents).unwrap();

    assert_eq!(roundtrip.description, "updated description");
    assert_eq!(
        roundtrip.criteria.len(),
        1,
        "old content must be fully replaced"
    );
    assert_eq!(roundtrip.criteria[0].name, "only-one");
}

// ── Test 6: scan_finds_created_library ───────────────────────────────────────

#[test]
fn scan_finds_created_library() {
    let dir = TempDir::new().unwrap();
    let assay_dir = dir.path();

    apply_criteria_wizard(&full_input("my-lib", false), assay_dir).expect("should create library");

    let libs = compose::scan_libraries(assay_dir).expect("scan_libraries should succeed");
    let names: Vec<&str> = libs.iter().map(|l| l.name.as_str()).collect();
    assert!(
        names.contains(&"my-lib"),
        "scan_libraries should find 'my-lib', got: {names:?}"
    );
}
