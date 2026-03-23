//! Assay invocation — translates Smelt manifests into Assay CLI commands.
//!
//! [`AssayInvoker`] is the translation layer between Smelt's [`JobManifest`] and
//! Assay's expected CLI contract. It builds Assay-compatible TOML files (a run
//! manifest and per-session spec files), writes them into a running container, and
//! constructs the `assay run` command.
//!
//! Design decision (D002): Smelt does not depend on the Assay crate directly.
//! Instead, we define our own serde structs that mirror Assay's expected input
//! format, keeping the two projects loosely coupled.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::SmeltError;
use crate::manifest::{JobManifest, SessionDef};
use crate::provider::{ContainerId, ExecHandle};

/// Path where the Assay run manifest is written inside the container.
const CONTAINER_MANIFEST_PATH: &str = "/tmp/smelt-manifest.toml";

// ── Assay serde types ────────────────────────────────────────────────

/// Top-level Assay run manifest — serialized to TOML and written into the container.
///
/// Maps to `[[sessions]]` (plural) in Assay's schema.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SmeltRunManifest {
    /// Session references for Assay to execute.
    pub sessions: Vec<SmeltManifestSession>,
}

/// A single entry in the run manifest's `[[sessions]]` array.
///
/// Each entry refers to a spec file by sanitized name and optionally carries
/// a human-readable display name and dependency list.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SmeltManifestSession {
    /// Sanitized spec file name (no extension) — references a file under
    /// `/workspace/.assay/specs/<spec>.toml`.
    pub spec: String,

    /// Optional human-readable display name (the original session name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Names of sessions (by spec reference) this session depends on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
}

/// An Assay spec file — one per session, written to
/// `/workspace/.assay/specs/<sanitized_name>.toml`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SmeltSpec {
    /// Unique spec name (sanitized session name).
    pub name: String,

    /// Free-text description / task specification prompt.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Criteria (gates) that Assay evaluates for this spec.
    pub criteria: Vec<SmeltCriterion>,
}

/// A single criterion within a spec's `[[criteria]]` array.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SmeltCriterion {
    /// Criterion name.
    pub name: String,

    /// Human-readable description.
    pub description: String,

    /// Shell command to run as the criterion gate (optional — absent means
    /// the criterion is evaluated by the LLM only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
}

// ── AssayInvoker ────────────────────────────────────────────────────

/// Stateless translation layer between Smelt manifests and Assay CLI invocations.
///
/// All methods are associated functions — no instance state is required.
pub struct AssayInvoker;

impl AssayInvoker {
    // ── Name sanitization ────────────────────────────────────────

    /// Sanitize a session name for use as a spec file name.
    ///
    /// Replaces any character that is not `[a-zA-Z0-9_-]` with `-`,
    /// collapses consecutive `-` into one, and trims leading/trailing `-`.
    /// Returns `"unnamed"` if the result is empty.
    pub fn sanitize_session_name(name: &str) -> String {
        // Replace disallowed characters with '-'
        let replaced: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect();

        // Collapse consecutive '-' into a single '-'
        let mut collapsed = String::with_capacity(replaced.len());
        let mut prev_dash = false;
        for c in replaced.chars() {
            if c == '-' {
                if !prev_dash {
                    collapsed.push(c);
                }
                prev_dash = true;
            } else {
                collapsed.push(c);
                prev_dash = false;
            }
        }

        // Trim leading and trailing '-'
        let trimmed = collapsed.trim_matches('-').to_string();

        if trimmed.is_empty() {
            "unnamed".to_string()
        } else {
            trimmed
        }
    }

    // ── TOML builders ────────────────────────────────────────────

    /// Build an Assay spec TOML string for a single session.
    ///
    /// The spec file is written to `/workspace/.assay/specs/<sanitized_name>.toml`
    /// and describes the task and the harness gate criterion.
    pub fn build_spec_toml(session: &SessionDef) -> String {
        let sanitized_name = Self::sanitize_session_name(&session.name);

        let spec = SmeltSpec {
            name: sanitized_name.clone(),
            description: session.spec.clone(),
            criteria: vec![SmeltCriterion {
                name: "harness".to_string(),
                description: format!("Harness gate for {}", session.name),
                cmd: Some(session.harness.clone()),
            }],
        };

        let toml_str = toml::to_string_pretty(&spec)
            .expect("SmeltSpec serialization is infallible for valid data");

        info!(
            session_name = %session.name,
            toml_bytes = toml_str.len(),
            "built assay spec TOML"
        );

        toml_str
    }

    /// Build an Assay run manifest TOML string from a Smelt [`JobManifest`].
    ///
    /// Maps each [`SessionDef`] to a
    /// `SmeltManifestSession` (referencing a spec file by sanitized name)
    /// and serializes the result as pretty-printed TOML.
    pub fn build_run_manifest_toml(manifest: &JobManifest) -> String {
        let run_manifest = SmeltRunManifest {
            sessions: manifest
                .session
                .iter()
                .map(|s| SmeltManifestSession {
                    spec: Self::sanitize_session_name(&s.name),
                    name: Some(s.name.clone()),
                    depends_on: s.depends_on.clone(),
                })
                .collect(),
        };

        let toml_str = toml::to_string_pretty(&run_manifest)
            .expect("SmeltRunManifest serialization is infallible for valid data");

        info!(
            session_count = manifest.session.len(),
            toml_bytes = toml_str.len(),
            "built assay run manifest TOML"
        );
        debug!(toml_content = %toml_str, "assay run manifest content");

        toml_str
    }

    // ── Command builders ─────────────────────────────────────────

    /// Build the command that ensures the Assay specs directory exists inside
    /// the container.
    ///
    /// Returns `["mkdir", "-p", "/workspace/.assay/specs"]`.
    pub fn build_ensure_specs_dir_command() -> Vec<String> {
        vec![
            "mkdir".to_string(),
            "-p".to_string(),
            "/workspace/.assay/specs".to_string(),
        ]
    }

    /// Build the command that writes a minimal Assay config file into the
    /// container, guarded by a `[ ! -f ... ]` check so it never overwrites an
    /// existing file.
    ///
    /// Produces a `sh -c "if [ ! -f /workspace/.assay/config.toml ]; then ... fi"` command.
    pub fn build_write_assay_config_command(project_name: &str) -> Vec<String> {
        let config_content = format!("project_name = {:?}\n", project_name);
        let encoded = BASE64.encode(config_content.as_bytes());

        let script = format!(
            "if [ ! -f /workspace/.assay/config.toml ]; then mkdir -p /workspace/.assay && echo '{}' | base64 -d > /workspace/.assay/config.toml; fi",
            encoded
        );

        vec!["sh".to_string(), "-c".to_string(), script]
    }

    /// Construct the `assay run` CLI command vector.
    ///
    /// Produces:
    /// ```text
    /// ["assay", "run", "/tmp/smelt-manifest.toml",
    ///  "--timeout", "<max_timeout>",
    ///  "--base-branch", "<base_ref>"]
    /// ```
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
            "--base-branch".to_string(),
            manifest.job.base_ref.clone(),
        ];

        info!(
            command = ?cmd,
            max_timeout,
            base_branch = %manifest.job.base_ref,
            "built assay run command"
        );

        cmd
    }

    // ── Container I/O ─────────────────────────────────────────────

    /// Write a TOML run manifest into a running container via base64-encoded exec.
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

    /// Write a spec TOML file into a running container via base64-encoded exec.
    ///
    /// Encodes `toml_content` as base64 and runs:
    /// ```text
    /// sh -c "echo '<base64>' | base64 -d > /workspace/.assay/specs/<sanitized_name>.toml"
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`SmeltError::Provider`] with operation `"write_spec_file"` if the
    /// exec command fails or returns a non-zero exit code.
    pub async fn write_spec_file_to_container(
        provider: &impl crate::provider::RuntimeProvider,
        container: &ContainerId,
        sanitized_name: &str,
        toml_content: &str,
    ) -> crate::Result<ExecHandle> {
        let encoded = BASE64.encode(toml_content.as_bytes());
        let spec_path = format!("/workspace/.assay/specs/{}.toml", sanitized_name);

        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            format!("echo '{}' | base64 -d > {}", encoded, spec_path),
        ];

        info!(
            container = %container,
            spec_name = %sanitized_name,
            spec_path = %spec_path,
            encoded_bytes = encoded.len(),
            "writing assay spec file to container"
        );

        let handle = provider.exec(container, &command).await.map_err(|e| {
            SmeltError::provider(
                "write_spec_file",
                format!(
                    "failed to write spec file '{sanitized_name}' to container {container}: {e}"
                ),
            )
        })?;

        if handle.exit_code != 0 {
            return Err(SmeltError::provider(
                "write_spec_file",
                format!(
                    "spec file write for '{sanitized_name}' exited with code {} in container {container}: stderr={}",
                    handle.exit_code,
                    handle.stderr.trim()
                ),
            ));
        }

        info!(
            container = %container,
            spec_name = %sanitized_name,
            spec_path = %spec_path,
            exit_code = handle.exit_code,
            "assay spec file written successfully"
        );

        Ok(handle)
    }
}

// ── Unit tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── Test helper ──────────────────────────────────────────────

    /// Wrap `sessions_toml` (one or more `[[session]]` blocks) in a valid
    /// manifest header and parse with `JobManifest::from_str`.
    fn test_manifest(sessions_toml: &str) -> JobManifest {
        let full = format!(
            r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:latest"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

{sessions_toml}

[merge]
strategy = "sequential"
target = "main"
"#
        );
        JobManifest::from_str(&full, Path::new("<test>")).expect("test manifest must parse")
    }

    // ── Manifest TOML key tests ───────────────────────────────────

    /// Assay's RunManifest uses `sessions` (plural); any regression to `session`
    /// causes a silent parse failure at the Assay side.
    #[test]
    fn test_run_manifest_uses_sessions_key_plural() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "unit-tests"
spec = "Run the unit test suite"
harness = "cargo test"
timeout = 300
"#,
        );

        let toml_str = AssayInvoker::build_run_manifest_toml(&manifest);
        let parsed: toml::Value =
            toml::from_str(&toml_str).expect("generated run manifest must be valid TOML");

        assert!(
            parsed.get("sessions").and_then(|v| v.as_array()).is_some(),
            "run manifest must use `sessions` key (plural); got:\n{toml_str}"
        );
        assert!(
            parsed.get("session").is_none(),
            "run manifest must NOT have a `session` key (singular); got:\n{toml_str}"
        );
    }

    /// The manifest session `spec` field must contain the sanitized session name
    /// (a file reference), NOT the free-text description.
    #[test]
    fn test_run_manifest_spec_is_sanitized_name_not_description() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "unit-tests"
spec = "Run the unit test suite"
harness = "cargo test"
timeout = 300
"#,
        );

        let toml_str = AssayInvoker::build_run_manifest_toml(&manifest);
        let parsed: toml::Value =
            toml::from_str(&toml_str).expect("generated run manifest must be valid TOML");

        let spec_val = parsed["sessions"][0]["spec"]
            .as_str()
            .expect("sessions[0].spec must be a string");

        assert_eq!(
            spec_val, "unit-tests",
            "spec must equal sanitized session name; got: {spec_val:?}"
        );
        assert_ne!(
            spec_val, "Run the unit test suite",
            "spec must NOT be the free-text description"
        );
    }

    /// The run manifest must not contain unknown fields (harness, timeout) in
    /// sessions entries — `deny_unknown_fields` would reject them at the Assay side.
    #[test]
    fn test_run_manifest_no_unknown_fields() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "unit-tests"
spec = "Run the unit test suite"
harness = "cargo test"
timeout = 300
"#,
        );

        let toml_str = AssayInvoker::build_run_manifest_toml(&manifest);

        // Must round-trip through our own deny_unknown_fields types
        let parsed_typed: SmeltRunManifest =
            toml::from_str(&toml_str).expect("SmeltRunManifest roundtrip must succeed");
        assert_eq!(parsed_typed.sessions.len(), 1);

        // Also check raw TOML: no harness or timeout in sessions[0]
        let parsed_raw: toml::Value =
            toml::from_str(&toml_str).expect("raw TOML must parse");

        assert!(
            parsed_raw["sessions"][0].get("harness").is_none(),
            "sessions[0] must not contain `harness` field"
        );
        assert!(
            parsed_raw["sessions"][0].get("timeout").is_none(),
            "sessions[0] must not contain `timeout` field"
        );
    }

    // ── Spec TOML structure tests ─────────────────────────────────

    /// The spec TOML must have `name`, `description`, and a `[[criteria]]` array
    /// whose first entry has `cmd` equal to the session harness.
    #[test]
    fn test_spec_toml_structure() {
        let session = SessionDef {
            name: "auth".to_string(),
            spec: "Implement the auth flow".to_string(),
            harness: "cargo test --test auth".to_string(),
            timeout: 300,
            depends_on: vec![],
        };

        let toml_str = AssayInvoker::build_spec_toml(&session);
        let parsed: toml::Value =
            toml::from_str(&toml_str).expect("spec TOML must be valid");

        assert_eq!(
            parsed["name"].as_str().unwrap(),
            "auth",
            "spec name must be sanitized session name"
        );
        assert_eq!(
            parsed["description"].as_str().unwrap(),
            "Implement the auth flow",
            "spec description must equal session.spec"
        );

        let criteria = parsed["criteria"]
            .as_array()
            .expect("spec must have a `criteria` array");
        assert!(
            criteria.len() >= 1,
            "spec criteria must have at least one entry"
        );
        assert_eq!(
            criteria[0]["cmd"].as_str().unwrap(),
            "cargo test --test auth",
            "criteria[0].cmd must equal session harness"
        );
    }

    /// The spec TOML must round-trip through `SmeltSpec` (deny_unknown_fields).
    #[test]
    fn test_spec_toml_deny_unknown_fields_roundtrip() {
        let session = SessionDef {
            name: "auth".to_string(),
            spec: "Implement the auth flow".to_string(),
            harness: "cargo test --test auth".to_string(),
            timeout: 300,
            depends_on: vec![],
        };

        let toml_str = AssayInvoker::build_spec_toml(&session);
        let parsed: SmeltSpec =
            toml::from_str(&toml_str).expect("SmeltSpec deny_unknown_fields roundtrip must succeed");
        assert_eq!(parsed.name, "auth");
        assert_eq!(parsed.criteria.len(), 1);
    }

    // ── Sanitization tests ────────────────────────────────────────

    /// Table-driven test for all sanitization edge cases.
    #[test]
    fn test_sanitize_session_name() {
        let cases: Vec<(&str, &str)> = vec![
            ("frontend", "frontend"),          // already clean
            ("my/session", "my-session"),       // slash replaced
            ("my session", "my-session"),       // space replaced
            ("a/b/c", "a-b-c"),                 // multi-slash
            ("trailing-", "trailing"),           // trailing dash trimmed
            ("-leading", "leading"),             // leading dash trimmed
            ("", "unnamed"),                     // empty → fallback
            ("---", "unnamed"),                  // all dashes → fallback after trim
        ];

        for (input, expected) in cases {
            assert_eq!(
                AssayInvoker::sanitize_session_name(input),
                expected,
                "input = {input:?}"
            );
        }
    }

    // ── Command builder tests ─────────────────────────────────────

    /// `build_run_command` must include `--base-branch` followed by the manifest's base_ref.
    #[test]
    fn test_build_run_command_includes_base_branch() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "tests"
spec = "Run tests"
harness = "cargo test"
timeout = 300
"#,
        );

        let cmd = AssayInvoker::build_run_command(&manifest);
        let idx = cmd
            .iter()
            .position(|s| s == "--base-branch")
            .expect("build_run_command must include --base-branch");
        assert_eq!(
            cmd[idx + 1], "main",
            "--base-branch must be followed by base_ref value"
        );
    }

    /// `build_run_command` must include `--timeout` with the max session timeout.
    #[test]
    fn test_build_run_command_includes_timeout() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "fast"
spec = "Fast tests"
harness = "cargo test --lib"
timeout = 300

[[session]]
name = "slow"
spec = "Slow integration tests"
harness = "cargo test --test integration"
timeout = 900
"#,
        );

        let cmd = AssayInvoker::build_run_command(&manifest);
        let idx = cmd
            .iter()
            .position(|s| s == "--timeout")
            .expect("build_run_command must include --timeout");
        assert_eq!(
            cmd[idx + 1], "900",
            "--timeout must equal max session timeout"
        );
    }

    /// `build_ensure_specs_dir_command` must return exactly `["mkdir", "-p", "/workspace/.assay/specs"]`.
    #[test]
    fn test_build_ensure_specs_dir_command() {
        assert_eq!(
            AssayInvoker::build_ensure_specs_dir_command(),
            vec!["mkdir", "-p", "/workspace/.assay/specs"]
        );
    }

    /// `build_write_assay_config_command` must use `sh -c` with an idempotency guard.
    #[test]
    fn test_build_write_assay_config_command() {
        let cmd = AssayInvoker::build_write_assay_config_command("my-project");

        assert_eq!(cmd[0], "sh", "cmd[0] must be sh");
        assert_eq!(cmd[1], "-c", "cmd[1] must be -c");

        let script = &cmd[2];
        assert!(
            script.contains("if [ ! -f /workspace/.assay/config.toml ]"),
            "script must contain idempotency guard; got:\n{script}"
        );
        assert!(
            script.contains("base64 -d"),
            "script must use base64 -d; got:\n{script}"
        );
        assert!(
            script.contains("/workspace/.assay/config.toml"),
            "script must write to /workspace/.assay/config.toml; got:\n{script}"
        );
    }

    /// Multi-session manifest: `depends_on` is preserved for sessions that have it,
    /// and absent (not serialized) for sessions that don't.
    #[test]
    fn test_multi_session_depends_on_preserved() {
        let manifest = test_manifest(
            r#"
[[session]]
name = "alpha"
spec = "First session"
harness = "cargo test --test alpha"
timeout = 300

[[session]]
name = "beta"
spec = "Second session"
harness = "cargo test --test beta"
timeout = 300

[[session]]
name = "gamma"
spec = "Third session"
harness = "cargo test --test gamma"
timeout = 300
depends_on = ["alpha", "beta"]
"#,
        );

        let toml_str = AssayInvoker::build_run_manifest_toml(&manifest);
        let parsed: toml::Value =
            toml::from_str(&toml_str).expect("run manifest must parse");

        // sessions[2] must have depends_on with expected values
        let depends_on = parsed["sessions"][2]["depends_on"]
            .as_array()
            .expect("sessions[2] must have depends_on");
        let names: Vec<&str> = depends_on.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(names.contains(&"alpha"), "depends_on must include alpha");
        assert!(names.contains(&"beta"), "depends_on must include beta");

        // sessions[0] must NOT have depends_on (skip_serializing_if = Vec::is_empty)
        assert!(
            parsed["sessions"][0].get("depends_on").is_none(),
            "sessions[0] must not serialize an empty depends_on"
        );
    }
}
