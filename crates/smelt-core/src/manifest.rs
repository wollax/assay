//! Job manifest types with strict TOML parsing and validation.
//!
//! A job manifest describes a complete Smelt job: what repo to work on,
//! what container image to use, which sessions to run, and how to merge results.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use crate::error::SmeltError;

/// Top-level job manifest parsed from TOML.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobManifest {
    /// Job metadata.
    pub job: JobMeta,

    /// Container environment configuration.
    pub environment: Environment,

    /// LLM credential configuration.
    pub credentials: CredentialConfig,

    /// Session definitions (at least one required).
    pub session: Vec<SessionDef>,

    /// Merge configuration.
    pub merge: MergeConfig,
}

/// `[job]` — high-level job metadata.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobMeta {
    /// Human-readable job name.
    pub name: String,

    /// Repository URL or local path.
    pub repo: String,

    /// Base ref (branch/tag/commit) to work from.
    pub base_ref: String,
}

/// `[environment]` — container runtime configuration.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    /// Container runtime (e.g., "docker").
    pub runtime: String,

    /// Container image reference.
    pub image: String,

    /// Resource limits (e.g., "cpu" -> "2", "memory" -> "4G").
    #[serde(default)]
    pub resources: HashMap<String, String>,
}

/// `[credentials]` — LLM provider credentials.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialConfig {
    /// LLM provider name (e.g., "anthropic", "openai").
    pub provider: String,

    /// Model identifier (e.g., "claude-sonnet-4-20250514").
    pub model: String,

    /// Environment variable overrides for credential resolution.
    /// Key = logical name, Value = env var name to read from.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// `[[session]]` — a single coding session definition.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDef {
    /// Unique session name.
    pub name: String,

    /// Task specification or prompt for this session.
    pub spec: String,

    /// Harness command to run in the container.
    pub harness: String,

    /// Timeout in seconds. Must be > 0.
    pub timeout: u64,

    /// Names of sessions this session depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// `[merge]` — how to merge session results.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MergeConfig {
    /// Merge strategy (e.g., "sequential", "octopus").
    pub strategy: String,

    /// Order in which sessions are merged.
    #[serde(default)]
    pub order: Vec<String>,

    /// Whether to use AI-assisted conflict resolution.
    #[serde(default)]
    pub ai_resolution: bool,

    /// Target branch for the merged result.
    pub target: String,
}

/// Validation errors collected during manifest validation.
#[derive(Debug)]
pub struct ValidationErrors {
    errors: Vec<String>,
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, err) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "  - {err}")?;
        }
        Ok(())
    }
}

impl JobManifest {
    /// Load a job manifest from a TOML file.
    ///
    /// Returns a [`SmeltError::Manifest`] if the file cannot be read or parsed.
    pub fn load(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| SmeltError::Manifest {
            field: "file".to_string(),
            message: format!("cannot read `{}`: {e}", path.display()),
        })?;
        Self::from_str(&content, path)
    }

    /// Parse a job manifest from a TOML string.
    ///
    /// `source` is used only for error messages.
    pub fn from_str(content: &str, source: &Path) -> crate::Result<Self> {
        toml::from_str(content).map_err(|e| SmeltError::Manifest {
            field: "toml".to_string(),
            message: format!("parsing `{}`: {e}", source.display()),
        })
    }

    /// Validate the manifest's semantic constraints.
    ///
    /// Returns `Ok(())` if valid, or a [`SmeltError::Manifest`] containing
    /// all validation errors.
    pub fn validate(&self) -> crate::Result<()> {
        let mut errors = Vec::new();

        // job.name must not be empty
        if self.job.name.trim().is_empty() {
            errors.push("job.name: must not be empty".to_string());
        }

        // job.repo must not be empty
        if self.job.repo.trim().is_empty() {
            errors.push("job.repo: must not be empty".to_string());
        }

        // environment.image must not be empty
        if self.environment.image.trim().is_empty() {
            errors.push("environment.image: must not be empty".to_string());
        }

        // At least one session required
        if self.session.is_empty() {
            errors.push("session: at least one session is required".to_string());
        }

        // Unique session names
        let mut seen_names = HashSet::new();
        for (i, sess) in self.session.iter().enumerate() {
            if sess.name.trim().is_empty() {
                errors.push(format!("session[{i}].name: must not be empty"));
            } else if !seen_names.insert(&sess.name) {
                errors.push(format!(
                    "session[{i}].name: duplicate session name `{}`",
                    sess.name
                ));
            }

            // timeout > 0
            if sess.timeout == 0 {
                errors.push(format!("session[{i}].timeout: must be > 0"));
            }
        }

        // depends_on references must be valid and no self-references
        let all_names: HashSet<&str> = self.session.iter().map(|s| s.name.as_str()).collect();
        for (i, sess) in self.session.iter().enumerate() {
            for dep in &sess.depends_on {
                if dep == &sess.name {
                    errors.push(format!(
                        "session[{i}].depends_on: `{}` cannot depend on itself",
                        sess.name
                    ));
                } else if !all_names.contains(dep.as_str()) {
                    errors.push(format!(
                        "session[{i}].depends_on: unknown session `{dep}`"
                    ));
                }
            }
        }

        // Check for circular dependencies
        if let Some(cycle) = Self::detect_cycle(&self.session) {
            errors.push(format!("session dependencies: cycle detected: {cycle}"));
        }

        // merge.target must not be empty
        if self.merge.target.trim().is_empty() {
            errors.push("merge.target: must not be empty".to_string());
        }

        // merge.order entries must reference valid sessions
        for entry in &self.merge.order {
            if !all_names.contains(entry.as_str()) {
                errors.push(format!(
                    "merge.order: unknown session `{entry}`"
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            let detail = ValidationErrors { errors };
            Err(SmeltError::Manifest {
                field: "validation".to_string(),
                message: format!("manifest validation failed:\n{detail}"),
            })
        }
    }

    /// Detect cycles in session dependency graph using DFS.
    /// Returns a description of the cycle if found.
    fn detect_cycle(sessions: &[SessionDef]) -> Option<String> {
        let name_to_idx: HashMap<&str, usize> = sessions
            .iter()
            .enumerate()
            .map(|(i, s)| (s.name.as_str(), i))
            .collect();

        // 0 = unvisited, 1 = in-progress, 2 = done
        let mut state = vec![0u8; sessions.len()];

        fn dfs(
            idx: usize,
            sessions: &[SessionDef],
            name_to_idx: &HashMap<&str, usize>,
            state: &mut Vec<u8>,
            path: &mut Vec<String>,
        ) -> Option<String> {
            state[idx] = 1;
            path.push(sessions[idx].name.clone());

            for dep in &sessions[idx].depends_on {
                if let Some(&dep_idx) = name_to_idx.get(dep.as_str()) {
                    if state[dep_idx] == 1 {
                        // Found a cycle — build the cycle path
                        path.push(dep.clone());
                        return Some(path.join(" -> "));
                    }
                    if state[dep_idx] == 0 {
                        if let Some(cycle) = dfs(dep_idx, sessions, name_to_idx, state, path) {
                            return Some(cycle);
                        }
                    }
                }
            }

            path.pop();
            state[idx] = 2;
            None
        }

        let mut path = Vec::new();
        for i in 0..sessions.len() {
            if state[i] == 0 {
                if let Some(cycle) = dfs(i, sessions, &name_to_idx, &mut state, &mut path) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    /// Resolve credential sources from the environment.
    ///
    /// Returns a map of logical name -> resolution status.
    pub fn resolve_credentials(&self) -> HashMap<String, CredentialStatus> {
        let mut results = HashMap::new();

        for (logical_name, env_var) in &self.credentials.env {
            let status = if std::env::var(env_var).is_ok() {
                CredentialStatus::Resolved {
                    source: format!("env:{env_var}"),
                }
            } else {
                CredentialStatus::Missing {
                    source: format!("env:{env_var}"),
                }
            };
            results.insert(logical_name.clone(), status);
        }

        results
    }
}

/// Status of a credential resolution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialStatus {
    /// Credential was found at the given source.
    Resolved { source: String },
    /// Credential was not found at the expected source.
    Missing { source: String },
}

impl std::fmt::Display for CredentialStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved { source } => write!(f, "{source} → resolved"),
            Self::Missing { source } => write!(f, "{source} → MISSING"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[environment.resources]
cpu = "2"
memory = "4G"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[credentials.env]
api_key = "ANTHROPIC_API_KEY"

[[session]]
name = "frontend"
spec = "Implement the login page"
harness = "npm test"
timeout = 300

[[session]]
name = "backend"
spec = "Implement the auth endpoint"
harness = "cargo test"
timeout = 600
depends_on = ["frontend"]

[merge]
strategy = "sequential"
order = ["frontend", "backend"]
ai_resolution = true
target = "main"
"#;

    fn load_from_str(content: &str) -> crate::Result<JobManifest> {
        JobManifest::from_str(content, Path::new("test.toml"))
    }

    #[test]
    fn parse_valid_manifest() {
        let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
        assert_eq!(manifest.job.name, "test-job");
        assert_eq!(manifest.job.repo, "https://github.com/example/repo");
        assert_eq!(manifest.job.base_ref, "main");
        assert_eq!(manifest.environment.runtime, "docker");
        assert_eq!(manifest.environment.image, "ubuntu:22.04");
        assert_eq!(manifest.environment.resources.get("cpu").unwrap(), "2");
        assert_eq!(manifest.credentials.provider, "anthropic");
        assert_eq!(manifest.credentials.model, "claude-sonnet-4-20250514");
        assert_eq!(manifest.session.len(), 2);
        assert_eq!(manifest.session[0].name, "frontend");
        assert_eq!(manifest.session[0].timeout, 300);
        assert_eq!(manifest.session[1].depends_on, vec!["frontend"]);
        assert_eq!(manifest.merge.strategy, "sequential");
        assert!(manifest.merge.ai_resolution);
        assert_eq!(manifest.merge.target, "main");
    }

    #[test]
    fn validate_valid_manifest() {
        let manifest = load_from_str(VALID_MANIFEST).unwrap();
        manifest.validate().expect("valid manifest should pass validation");
    }

    #[test]
    fn reject_unknown_fields() {
        let bad = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"
bogus_field = "oops"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
        let err = load_from_str(bad).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("bogus_field") || msg.contains("unknown field"), "error should mention unknown field: {msg}");
    }

    #[test]
    fn reject_unknown_fields_in_session() {
        let bad = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60
extra_thing = true

[merge]
strategy = "sequential"
target = "main"
"#;
        let err = load_from_str(bad).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("extra_thing") || msg.contains("unknown field"), "error should mention unknown field: {msg}");
    }

    #[test]
    fn validate_duplicate_session_names() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "dupe"
spec = "s"
harness = "h"
timeout = 60

[[session]]
name = "dupe"
spec = "s2"
harness = "h2"
timeout = 120

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate session name"), "should report duplicate: {msg}");
    }

    #[test]
    fn validate_zero_timeout() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 0

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("session[0].timeout: must be > 0"), "should report zero timeout: {msg}");
    }

    #[test]
    fn validate_self_dependency() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60
depends_on = ["s1"]

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cannot depend on itself"), "should report self-dep: {msg}");
    }

    #[test]
    fn validate_unknown_dependency() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60
depends_on = ["nonexistent"]

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown session `nonexistent`"), "should report unknown dep: {msg}");
    }

    #[test]
    fn validate_circular_dependency() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "a"
spec = "s"
harness = "h"
timeout = 60
depends_on = ["b"]

[[session]]
name = "b"
spec = "s"
harness = "h"
timeout = 60
depends_on = ["a"]

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cycle detected"), "should report cycle: {msg}");
    }

    #[test]
    fn validate_empty_image() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = ""

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("environment.image: must not be empty"), "should report empty image: {msg}");
    }

    #[test]
    fn validate_empty_merge_target() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = ""
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("merge.target: must not be empty"), "should report empty target: {msg}");
    }

    #[test]
    fn validate_invalid_merge_order() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
order = ["s1", "nonexistent"]
target = "main"
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("merge.order: unknown session `nonexistent`"), "should report bad order: {msg}");
    }

    #[test]
    fn validate_no_sessions() {
        let toml = r#"
[job]
name = "test"
repo = "repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[merge]
strategy = "sequential"
target = "main"
"#;
        // TOML without any [[session]] entries will fail to parse since
        // session is a required Vec. Let's test that it's caught.
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("session"), "should mention missing session: {msg}");
    }

    #[test]
    fn credential_resolution_from_env() {
        let manifest = load_from_str(VALID_MANIFEST).unwrap();

        // SAFETY: This test is single-threaded and only modifies a test-specific env var.
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "test-value") };
        let creds = manifest.resolve_credentials();
        let status = creds.get("api_key").expect("should have api_key entry");
        assert!(matches!(status, CredentialStatus::Resolved { .. }));
        assert!(status.to_string().contains("resolved"));

        // Unset and check missing
        // SAFETY: Same as above — single-threaded test.
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        let creds = manifest.resolve_credentials();
        let status = creds.get("api_key").unwrap();
        assert!(matches!(status, CredentialStatus::Missing { .. }));
        assert!(status.to_string().contains("MISSING"));
    }

    #[test]
    fn load_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.toml");
        std::fs::write(&path, VALID_MANIFEST).unwrap();
        let manifest = JobManifest::load(&path).expect("should load from file");
        assert_eq!(manifest.job.name, "test-job");
        manifest.validate().expect("should validate");
    }

    #[test]
    fn load_nonexistent_file() {
        let err = JobManifest::load(Path::new("/tmp/nonexistent-manifest-12345.toml")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cannot read"), "should report read error: {msg}");
    }

    #[test]
    fn validate_multiple_errors_reported() {
        let toml = r#"
[job]
name = ""
repo = ""
base_ref = "main"

[environment]
runtime = "docker"
image = ""

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 0

[merge]
strategy = "sequential"
target = ""
"#;
        let manifest = load_from_str(toml).unwrap();
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        // Should report all four errors, not just the first
        assert!(msg.contains("job.name"), "should report empty name: {msg}");
        assert!(msg.contains("job.repo"), "should report empty repo: {msg}");
        assert!(msg.contains("environment.image"), "should report empty image: {msg}");
        assert!(msg.contains("timeout"), "should report zero timeout: {msg}");
        assert!(msg.contains("merge.target"), "should report empty target: {msg}");
    }
}
