//! Integration tests for ProviderConfig round-trip and config_save.

use std::fs;

use tempfile::TempDir;

use assay_core::config::{load, save};
use assay_types::{Config, ProviderConfig, ProviderKind};

/// Helper: create a minimal `.assay/config.toml` in `tmp`.
fn setup_project(tmp: &TempDir, toml: &str) {
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&assay_dir).unwrap();
    fs::write(assay_dir.join("config.toml"), toml).unwrap();
}

#[test]
fn config_toml_roundtrip_without_provider() {
    // An existing config.toml without a [provider] section must load without error.
    let tmp = TempDir::new().unwrap();
    setup_project(
        &tmp,
        r#"project_name = "my-project"
"#,
    );
    let cfg = load(tmp.path()).expect("config without [provider] should load without error");
    assert_eq!(cfg.project_name, "my-project");
    assert!(
        cfg.provider.is_none(),
        "provider should be None when absent"
    );
}

#[test]
fn config_toml_roundtrip_with_provider() {
    // A config.toml with [provider] section round-trips correctly.
    let tmp = TempDir::new().unwrap();
    setup_project(
        &tmp,
        r#"project_name = "my-project"

[provider]
provider = "open_ai"
planning_model = "gpt-4o"
"#,
    );
    let cfg = load(tmp.path()).expect("config with [provider] should load");
    let prov = cfg.provider.as_ref().expect("provider should be Some");
    assert_eq!(prov.provider, ProviderKind::OpenAi);
    assert_eq!(prov.planning_model.as_deref(), Some("gpt-4o"));
    assert!(prov.execution_model.is_none());
    assert!(prov.review_model.is_none());
}

#[test]
fn config_save_creates_file() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&assay_dir).unwrap();

    let cfg = Config {
        project_name: "save-test".to_string(),
        specs_dir: "specs/".to_string(),
        gates: None,
        guard: None,
        worktree: None,
        sessions: None,
        provider: None,
        workflow: None,
    };

    save(tmp.path(), &cfg).expect("save should succeed");

    let content = fs::read_to_string(assay_dir.join("config.toml")).unwrap();
    assert!(content.contains("save-test"));
}

#[test]
fn config_save_overwrites_existing() {
    let tmp = TempDir::new().unwrap();
    setup_project(&tmp, "project_name = \"old-name\"\n");

    let cfg = Config {
        project_name: "new-name".to_string(),
        specs_dir: "specs/".to_string(),
        gates: None,
        guard: None,
        worktree: None,
        sessions: None,
        provider: None,
        workflow: None,
    };

    save(tmp.path(), &cfg).expect("save should succeed");
    let loaded = load(tmp.path()).unwrap();
    assert_eq!(loaded.project_name, "new-name");
}

#[test]
fn config_save_with_provider_persists() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    fs::create_dir_all(&assay_dir).unwrap();

    let cfg = Config {
        project_name: "provider-test".to_string(),
        specs_dir: "specs/".to_string(),
        gates: None,
        guard: None,
        worktree: None,
        sessions: None,
        provider: Some(ProviderConfig {
            provider: ProviderKind::OpenAi,
            planning_model: Some("gpt-4o".to_string()),
            execution_model: None,
            review_model: None,
        }),
        workflow: None,
    };

    save(tmp.path(), &cfg).expect("save with provider should succeed");
    let loaded = load(tmp.path()).unwrap();
    let prov = loaded
        .provider
        .as_ref()
        .expect("provider should be present after save");
    assert_eq!(prov.provider, ProviderKind::OpenAi);
    assert_eq!(prov.planning_model.as_deref(), Some("gpt-4o"));
}
