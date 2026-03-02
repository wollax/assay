//! Project initialization logic.
//!
//! Creates the `.assay/` directory structure with config, example spec,
//! and gitignore. Refuses to overwrite an existing project.

use std::path::{Path, PathBuf};

use crate::error::{AssayError, Result};

/// Options for project initialization.
pub struct InitOptions {
    /// Override the project name (otherwise inferred from directory name).
    pub name: Option<String>,
}

/// Summary of what was created during initialization.
#[derive(Debug)]
pub struct InitResult {
    /// The resolved project name (inferred or overridden).
    pub project_name: String,
    /// All files created during initialization.
    pub created_files: Vec<PathBuf>,
}

/// Initialize an Assay project at the given root directory.
///
/// Creates `.assay/config.toml`, `.assay/specs/`, `.assay/.gitignore`,
/// and an example spec file.
///
/// Returns `AssayError::AlreadyInitialized` if `.assay/` already exists.
pub fn init(root: &Path, options: &InitOptions) -> Result<InitResult> {
    let assay_dir = root.join(".assay");
    let specs_dir = assay_dir.join("specs");

    // CFG-04: Refuse to overwrite existing .assay/
    match std::fs::create_dir(&assay_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(AssayError::AlreadyInitialized);
        }
        Err(source) => {
            return Err(AssayError::Io {
                operation: "creating .assay directory".into(),
                path: assay_dir,
                source,
            });
        }
    }

    // Create specs/ subdirectory
    std::fs::create_dir(&specs_dir).map_err(|source| AssayError::Io {
        operation: "creating specs directory".into(),
        path: specs_dir.clone(),
        source,
    })?;

    let mut created_files = Vec::new();

    // CFG-02: Generate config.toml
    let project_name = options
        .name
        .clone()
        .unwrap_or_else(|| infer_project_name(root));

    let config_path = assay_dir.join("config.toml");
    let config_content = render_config_template(&project_name);
    std::fs::write(&config_path, &config_content).map_err(|source| AssayError::Io {
        operation: "writing config".into(),
        path: config_path.clone(),
        source,
    })?;
    created_files.push(config_path);

    // CFG-03: Example spec
    let spec_path = specs_dir.join("hello-world.toml");
    std::fs::write(&spec_path, render_example_spec()).map_err(|source| AssayError::Io {
        operation: "writing example spec".into(),
        path: spec_path.clone(),
        source,
    })?;
    created_files.push(spec_path);

    // .gitignore inside .assay/
    let gitignore_path = assay_dir.join(".gitignore");
    std::fs::write(&gitignore_path, render_gitignore()).map_err(|source| AssayError::Io {
        operation: "writing .gitignore".into(),
        path: gitignore_path.clone(),
        source,
    })?;
    created_files.push(gitignore_path);

    Ok(InitResult {
        project_name,
        created_files,
    })
}

/// Infer the project name from the directory's leaf name.
///
/// Falls back to `"assay-project"` when the path has no usable leaf
/// (e.g., root `/` or `..`).
fn infer_project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or("assay-project")
        .to_string()
}

/// Generate config.toml content with comments.
fn render_config_template(project_name: &str) -> String {
    format!(
        r#"# Assay project configuration
# Documentation: https://assay.dev/docs/config

# Project name (required)
project_name = "{project_name}"

# Directory containing spec files (relative to .assay/)
specs_dir = "specs/"

# Gate execution configuration
# Uncomment and customize as needed.
# [gates]
# Maximum time (seconds) a gate command can run before being killed.
# default_timeout = 300
#
# Working directory for gate execution. See GATE-04.
# working_dir = "."
"#
    )
}

/// Generate the example spec file content.
fn render_example_spec() -> &'static str {
    r#"# Example specification
# This file demonstrates how to write an Assay spec.
# Spec files live in .assay/specs/ and use TOML format.

# The spec name (required, must be unique across all specs)
name = "hello-world"

# A human-readable description of what this spec covers
description = "A starter spec to verify your Assay setup works"

# Criteria define the acceptance conditions for this spec.
# Each criterion has a name and description.
# Add an optional `cmd` field to make it machine-evaluatable.

[[criteria]]
name = "project-builds"
description = "The project compiles without errors"
cmd = "echo 'hello from assay'"

[[criteria]]
name = "readme-exists"
description = "A README file exists in the project root"
# No `cmd` — this criterion is evaluated manually (or by an agent in future versions)
"#
}

/// Generate .gitignore content for the .assay/ directory.
fn render_gitignore() -> &'static str {
    r#"# Assay transient files
# Results from gate evaluations
results/
# Cache files
*.cache
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Create a unique temporary directory for test isolation.
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("assay-init-test-{name}-{}", std::process::id()));
        // Clean up any previous run
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).expect("failed to create test dir");
        dir
    }

    /// Clean up a test directory.
    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_init_creates_all_artifacts() {
        let root = test_dir("all-artifacts");
        let options = InitOptions { name: None };

        let result = init(&root, &options).expect("init should succeed");

        // .assay/ directory exists
        assert!(root.join(".assay").is_dir());
        // specs/ subdirectory exists
        assert!(root.join(".assay/specs").is_dir());
        // config.toml exists
        assert!(root.join(".assay/config.toml").is_file());
        // hello-world.toml exists
        assert!(root.join(".assay/specs/hello-world.toml").is_file());
        // .gitignore exists
        assert!(root.join(".assay/.gitignore").is_file());

        // created_files tracks all three written files
        assert_eq!(result.created_files.len(), 3);

        cleanup(&root);
    }

    #[test]
    fn test_init_refuses_existing() {
        let root = test_dir("refuses-existing");
        let options = InitOptions { name: None };

        // First init succeeds
        init(&root, &options).expect("first init should succeed");

        // Second init fails with AlreadyInitialized
        let err = init(&root, &options).expect_err("second init should fail");
        assert!(
            matches!(err, AssayError::AlreadyInitialized),
            "expected AlreadyInitialized, got: {err:?}"
        );

        cleanup(&root);
    }

    #[test]
    fn test_init_infers_project_name() {
        let root = test_dir("my-cool-project");
        // Rename to get an actual meaningful directory name
        let named_root = root.parent().unwrap().join("my-cool-project");
        let _ = std::fs::remove_dir_all(&named_root);
        std::fs::rename(&root, &named_root).expect("rename should work");

        let options = InitOptions { name: None };
        let result = init(&named_root, &options).expect("init should succeed");

        assert_eq!(result.project_name, "my-cool-project");

        let config = std::fs::read_to_string(named_root.join(".assay/config.toml"))
            .expect("config should be readable");
        assert!(
            config.contains(r#"project_name = "my-cool-project""#),
            "config should contain inferred project name, got:\n{config}"
        );

        cleanup(&named_root);
    }

    #[test]
    fn test_init_name_override() {
        let root = test_dir("name-override");
        let options = InitOptions {
            name: Some("custom".to_string()),
        };

        let result = init(&root, &options).expect("init should succeed");

        assert_eq!(result.project_name, "custom");

        let config = std::fs::read_to_string(root.join(".assay/config.toml"))
            .expect("config should be readable");
        assert!(
            config.contains(r#"project_name = "custom""#),
            "config should contain custom project name, got:\n{config}"
        );

        cleanup(&root);
    }

    #[test]
    fn test_init_config_template_has_comments() {
        let root = test_dir("config-comments");
        let options = InitOptions { name: None };

        init(&root, &options).expect("init should succeed");

        let config = std::fs::read_to_string(root.join(".assay/config.toml"))
            .expect("config should be readable");

        assert!(
            config.contains("# [gates]"),
            "config should have commented-out [gates] section, got:\n{config}"
        );
        assert!(
            config.contains("# default_timeout"),
            "config should have commented-out default_timeout, got:\n{config}"
        );

        cleanup(&root);
    }

    #[test]
    fn test_init_example_spec_has_both_criteria_modes() {
        let root = test_dir("spec-criteria");
        let options = InitOptions { name: None };

        init(&root, &options).expect("init should succeed");

        let spec = std::fs::read_to_string(root.join(".assay/specs/hello-world.toml"))
            .expect("spec should be readable");

        // Has a runnable criterion with cmd
        assert!(
            spec.contains("cmd ="),
            "spec should have a criterion with cmd, got:\n{spec}"
        );

        // Has a descriptive-only criterion (the readme-exists one has no cmd)
        // Verify there's a criterion without cmd by checking the pattern
        assert!(
            spec.contains("# No `cmd`"),
            "spec should have a criterion without cmd (with explanatory comment), got:\n{spec}"
        );

        cleanup(&root);
    }

    #[test]
    fn test_infer_project_name_fallback() {
        // Root path `/` has no file_name component
        let name = infer_project_name(Path::new("/"));
        assert_eq!(name, "assay-project");
    }
}
