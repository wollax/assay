//! Assay invocation — translates Smelt manifests into Assay CLI commands.
//!
//! [`AssayInvoker`] is the translation layer between Smelt's [`JobManifest`] and
//! Assay's expected CLI contract. It builds an Assay-compatible TOML manifest,
//! writes it into a running container, and constructs the `assay run` command.
//!
//! Design decision (D002): Smelt does not depend on the Assay crate directly.
//! Instead, we define our own serde structs that mirror Assay's expected input
//! format, keeping the two projects loosely coupled.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::SmeltError;
use crate::manifest::JobManifest;
use crate::provider::{ContainerId, ExecHandle};

/// Path where the Assay manifest is written inside the container.
const CONTAINER_MANIFEST_PATH: &str = "/tmp/smelt-manifest.toml";

// ── Assay manifest serde types ──────────────────────────────────────

/// Top-level Assay manifest — serialized to TOML and written into the container.
#[derive(Debug, Serialize, Deserialize)]
pub struct AssayManifest {
    /// Session definitions for Assay to execute.
    pub session: Vec<AssaySession>,
}

/// A single session in the Assay manifest.
#[derive(Debug, Serialize, Deserialize)]
pub struct AssaySession {
    /// Unique session name.
    pub name: String,
    /// Task specification or prompt.
    pub spec: String,
    /// Harness command to run.
    pub harness: String,
    /// Timeout in seconds.
    pub timeout: u64,
    /// Names of sessions this session depends on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

// ── AssayInvoker ────────────────────────────────────────────────────

/// Stateless translation layer between Smelt manifests and Assay CLI invocations.
///
/// All methods are associated functions — no instance state is required.
pub struct AssayInvoker;

impl AssayInvoker {
    /// Build an Assay-compatible TOML manifest string from a Smelt [`JobManifest`].
    ///
    /// Maps each [`SessionDef`](crate::manifest::SessionDef) to an [`AssaySession`]
    /// and serializes the result as pretty-printed TOML.
    pub fn build_manifest_toml(manifest: &JobManifest) -> String {
        let assay_manifest = AssayManifest {
            session: manifest
                .session
                .iter()
                .map(|s| AssaySession {
                    name: s.name.clone(),
                    spec: s.spec.clone(),
                    harness: s.harness.clone(),
                    timeout: s.timeout,
                    depends_on: s.depends_on.clone(),
                })
                .collect(),
        };

        let toml_str =
            toml::to_string_pretty(&assay_manifest).expect("AssayManifest serialization is infallible for valid data");

        info!(
            session_count = manifest.session.len(),
            toml_bytes = toml_str.len(),
            "built assay manifest TOML"
        );
        debug!(toml_content = %toml_str, "assay manifest content");

        toml_str
    }

    /// Write a TOML manifest into a running container via base64-encoded exec.
    ///
    /// Encodes `toml_content` as base64 and runs:
    /// ```text
    /// sh -c "echo '<base64>' | base64 -d > /tmp/smelt-manifest.toml"
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`SmeltError::Provider`] with operation `"write_manifest"` if the
    /// exec command fails or returns a non-zero exit code.
    pub async fn write_manifest_to_container(
        provider: &impl crate::provider::RuntimeProvider,
        container: &ContainerId,
        toml_content: &str,
    ) -> crate::Result<ExecHandle> {
        let encoded = BASE64.encode(toml_content.as_bytes());

        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!("echo '{}' | base64 -d > {}", encoded, CONTAINER_MANIFEST_PATH),
        ];

        info!(
            container = %container,
            manifest_path = CONTAINER_MANIFEST_PATH,
            encoded_bytes = encoded.len(),
            "writing assay manifest to container"
        );

        let handle = provider.exec(container, &command).await.map_err(|e| {
            SmeltError::provider(
                "write_manifest",
                format!("failed to write manifest to container {container}: {e}"),
            )
        })?;

        if handle.exit_code != 0 {
            return Err(SmeltError::provider(
                "write_manifest",
                format!(
                    "manifest write exited with code {} in container {container}: stderr={}",
                    handle.exit_code,
                    handle.stderr.trim()
                ),
            ));
        }

        info!(
            container = %container,
            exit_code = handle.exit_code,
            "assay manifest written successfully"
        );

        Ok(handle)
    }

    /// Construct the `assay run` CLI command vector.
    ///
    /// Produces `["assay", "run", "/tmp/smelt-manifest.toml"]` with an optional
    /// `--timeout` flag set to the maximum timeout across all sessions.
    pub fn build_run_command(manifest: &JobManifest) -> Vec<String> {
        let max_timeout = manifest
            .session
            .iter()
            .map(|s| s.timeout)
            .max()
            .unwrap_or(300);

        let cmd = vec![
            "assay".to_string(),
            "run".to_string(),
            CONTAINER_MANIFEST_PATH.to_string(),
            "--timeout".to_string(),
            max_timeout.to_string(),
        ];

        info!(
            command = ?cmd,
            max_timeout,
            "built assay run command"
        );

        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Helper: build a minimal valid JobManifest from TOML.
    fn test_manifest(sessions_toml: &str) -> JobManifest {
        let toml = format!(
            r#"
[job]
name = "test-job"
repo = "."
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

{sessions_toml}

[merge]
strategy = "sequential"
target = "main"
"#
        );
        JobManifest::from_str(&toml, Path::new("test.toml")).unwrap()
    }

    #[test]
    fn test_single_session_manifest() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "unit-tests"
spec = "Run the unit test suite"
harness = "cargo test"
timeout = 300
"#,
        );

        let toml_str = AssayInvoker::build_manifest_toml(&manifest);

        // Parse back to verify structure
        let parsed: toml::Value = toml::from_str(&toml_str).expect("should be valid TOML");

        let sessions = parsed["session"].as_array().expect("should have session array");
        assert_eq!(sessions.len(), 1);

        let s = &sessions[0];
        assert_eq!(s["name"].as_str().unwrap(), "unit-tests");
        assert_eq!(s["spec"].as_str().unwrap(), "Run the unit test suite");
        assert_eq!(s["harness"].as_str().unwrap(), "cargo test");
        assert_eq!(s["timeout"].as_integer().unwrap(), 300);

        // depends_on should not be present (skip_serializing_if empty)
        assert!(s.get("depends_on").is_none(), "empty depends_on should be omitted");
    }

    #[test]
    fn test_multi_session_with_deps() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "frontend"
spec = "Build the login page"
harness = "npm test"
timeout = 300

[[session]]
name = "backend"
spec = "Build the auth API"
harness = "cargo test"
timeout = 600
depends_on = ["frontend"]

[[session]]
name = "integration"
spec = "Run integration tests"
harness = "make test-integration"
timeout = 900
depends_on = ["frontend", "backend"]
"#,
        );

        let toml_str = AssayInvoker::build_manifest_toml(&manifest);
        let parsed: toml::Value = toml::from_str(&toml_str).unwrap();
        let sessions = parsed["session"].as_array().unwrap();

        assert_eq!(sessions.len(), 3);

        // First session: no depends_on
        assert!(sessions[0].get("depends_on").is_none());

        // Second session: depends on frontend
        let deps1 = sessions[1]["depends_on"]
            .as_array()
            .expect("should have depends_on");
        assert_eq!(deps1.len(), 1);
        assert_eq!(deps1[0].as_str().unwrap(), "frontend");

        // Third session: depends on both
        let deps2 = sessions[2]["depends_on"].as_array().unwrap();
        assert_eq!(deps2.len(), 2);
        let dep_names: Vec<&str> = deps2.iter().map(|d| d.as_str().unwrap()).collect();
        assert_eq!(dep_names, vec!["frontend", "backend"]);
    }

    #[test]
    fn test_special_chars_in_spec() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "edge-cases"
spec = "Handle strings with 'quotes', \"double quotes\", [brackets], and {braces}"
harness = "echo 'hello \"world\"'"
timeout = 60
"#,
        );

        let toml_str = AssayInvoker::build_manifest_toml(&manifest);

        // Must be valid TOML after round-trip
        let parsed: toml::Value =
            toml::from_str(&toml_str).expect("special chars should survive TOML serialization");

        let sessions = parsed["session"].as_array().unwrap();
        let spec = sessions[0]["spec"].as_str().unwrap();
        assert!(spec.contains("'quotes'"), "single quotes should survive");
        assert!(spec.contains("\"double quotes\""), "double quotes should survive");
        assert!(spec.contains("[brackets]"), "brackets should survive");
        assert!(spec.contains("{braces}"), "braces should survive");
    }

    #[test]
    fn test_build_command_single_session() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "test"
spec = "Run tests"
harness = "cargo test"
timeout = 300
"#,
        );

        let cmd = AssayInvoker::build_run_command(&manifest);
        assert_eq!(cmd[0], "assay");
        assert_eq!(cmd[1], "run");
        assert_eq!(cmd[2], "/tmp/smelt-manifest.toml");
        assert_eq!(cmd[3], "--timeout");
        assert_eq!(cmd[4], "300");
    }

    #[test]
    fn test_build_command_uses_max_timeout() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "fast"
spec = "Quick test"
harness = "echo ok"
timeout = 60

[[session]]
name = "slow"
spec = "Slow test"
harness = "sleep 500"
timeout = 900

[[session]]
name = "medium"
spec = "Medium test"
harness = "make test"
timeout = 300
"#,
        );

        let cmd = AssayInvoker::build_run_command(&manifest);
        assert_eq!(cmd[3], "--timeout");
        assert_eq!(cmd[4], "900", "should use max timeout across all sessions");
    }

    #[test]
    fn test_manifest_toml_is_valid_toml() {
        // Verify that the generated TOML can be parsed back into AssayManifest-compatible structure
        let manifest = test_manifest(
            r#"
[[session]]
name = "s1"
spec = "Do something"
harness = "make test"
timeout = 120
depends_on = ["s2"]

[[session]]
name = "s2"
spec = "Do something else"
harness = "npm test"
timeout = 240
"#,
        );

        let toml_str = AssayInvoker::build_manifest_toml(&manifest);

        // Parse back using our own types to verify round-trip
        let roundtrip: AssayManifest =
            toml::from_str(&toml_str).expect("should deserialize back to AssayManifest");
        assert_eq!(roundtrip.session.len(), 2);
        assert_eq!(roundtrip.session[0].name, "s1");
        assert_eq!(roundtrip.session[0].depends_on, vec!["s2"]);
        assert_eq!(roundtrip.session[1].name, "s2");
        assert!(roundtrip.session[1].depends_on.is_empty());
    }
}
