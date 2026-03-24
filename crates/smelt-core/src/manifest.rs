//! Job manifest types with strict TOML parsing and validation.
//!
//! A job manifest describes a complete Smelt job: what repo to work on,
//! what container image to use, which sessions to run, and how to merge results.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::SmeltError;
use crate::forge::ForgeConfig;

/// Configuration for the Kubernetes runtime provider.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubernetesConfig {
    /// Kubernetes namespace for the Pod and SSH Secret.
    pub namespace: String,

    /// kubeconfig context to use; uses ambient context if absent.
    #[serde(default)]
    pub context: Option<String>,

    /// Name of the env var containing the SSH private key.
    pub ssh_key_env: String,

    /// CPU request for the agent container (e.g. "500m").
    #[serde(default)]
    pub cpu_request: Option<String>,

    /// Memory request for the agent container (e.g. "512Mi").
    #[serde(default)]
    pub memory_request: Option<String>,

    /// CPU limit for the agent container.
    #[serde(default)]
    pub cpu_limit: Option<String>,

    /// Memory limit for the agent container.
    #[serde(default)]
    pub memory_limit: Option<String>,
}

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

    /// Optional forge (PR creation) configuration.
    #[serde(default)]
    pub forge: Option<ForgeConfig>,

    /// Optional Kubernetes runtime configuration.
    #[serde(default)]
    pub kubernetes: Option<KubernetesConfig>,

    /// Docker Compose service definitions, populated from `[[services]]` TOML array.
    /// Defaults to an empty vec so existing manifests without services still parse.
    #[serde(default)]
    pub services: Vec<ComposeService>,
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

/// `[[services]]` — a Docker Compose service definition.
///
/// Known fields (`name`, `image`) are captured as typed fields. Any additional
/// Compose keys (e.g. `ports`, `volumes`, `environment`) are captured as-is
/// via serde flatten into `extra`. No `deny_unknown_fields` so this is an
/// intentional passthrough per D073.
#[derive(Debug, Deserialize)]
pub struct ComposeService {
    /// Service name used to identify this container within the compose network.
    pub name: String,

    /// Container image reference (e.g. `"postgres:16"`).
    pub image: String,

    /// All remaining Compose keys for this service (ports, volumes, env, etc.).
    #[serde(flatten)]
    pub extra: IndexMap<String, toml::Value>,
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

        // environment.runtime must be a known value
        const VALID_RUNTIMES: &[&str] = &["docker", "compose", "kubernetes"];
        if !VALID_RUNTIMES.contains(&self.environment.runtime.as_str()) {
            errors.push(format!(
                "environment.runtime: must be one of {:?}, got `{}`",
                VALID_RUNTIMES, self.environment.runtime
            ));
        }

        // kubernetes block requires kubernetes runtime and vice versa
        if self.environment.runtime == "kubernetes" {
            match &self.kubernetes {
                None => errors.push("kubernetes: `runtime = \"kubernetes\"` requires a `[kubernetes]` block".to_string()),
                Some(k) => {
                    if k.namespace.trim().is_empty() {
                        errors.push("kubernetes.namespace: must not be empty".to_string());
                    }
                    if k.ssh_key_env.trim().is_empty() {
                        errors.push("kubernetes.ssh_key_env: must not be empty".to_string());
                    }
                }
            }
        } else if self.kubernetes.is_some() {
            errors.push(format!(
                "kubernetes: `[kubernetes]` block requires `runtime = \"kubernetes\"`, got `{}`",
                self.environment.runtime
            ));
        }

        // services entries require compose runtime
        if self.environment.runtime != "compose" && !self.services.is_empty() {
            errors.push(format!(
                "services: `[[services]]` entries require `runtime = \"compose\"`, got `{}`",
                self.environment.runtime
            ));
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

        // Per-service validation (only enforced when runtime is compose)
        if self.environment.runtime == "compose" {
            for (i, svc) in self.services.iter().enumerate() {
                if svc.name.trim().is_empty() {
                    errors.push(format!("services[{i}].name: must not be empty"));
                }
                if svc.image.trim().is_empty() {
                    errors.push(format!("services[{i}].image: must not be empty"));
                }
            }
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

        // forge section validation (structural only — D018)
        if let Some(ref forge) = self.forge {
            if forge.token_env.trim().is_empty() {
                errors.push("forge.token_env: must not be empty".to_string());
            }
            let valid_repo = forge
                .repo
                .split_once('/')
                .map(|(owner, name)| !owner.is_empty() && !name.is_empty())
                .unwrap_or(false);
            if !valid_repo {
                errors.push(format!(
                    "forge.repo: must be in `owner/repo` format, got `{}`",
                    forge.repo
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
                    if state[dep_idx] == 0
                        && let Some(cycle) = dfs(dep_idx, sessions, name_to_idx, state, path) {
                            return Some(cycle);
                        }
                }
            }

            path.pop();
            state[idx] = 2;
            None
        }

        let mut path = Vec::new();
        for i in 0..sessions.len() {
            if state[i] == 0
                && let Some(cycle) = dfs(i, sessions, &name_to_idx, &mut state, &mut path) {
                    return Some(cycle);
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
    Resolved {
        /// Description of where the credential was resolved from (e.g. an environment variable name).
        source: String,
    },
    /// Credential was not found at the expected source.
    Missing {
        /// Description of where the credential was expected to be found.
        source: String,
    },
}

impl std::fmt::Display for CredentialStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved { source } => write!(f, "{source} → resolved"),
            Self::Missing { source } => write!(f, "{source} → MISSING"),
        }
    }
}

/// URL-like prefixes that indicate a remote repo (not a local path).
const URL_PREFIXES: &[&str] = &["http://", "https://", "git://", "ssh://"];

/// Resolve a repo path string to a canonicalized absolute [`PathBuf`].
///
/// Rejects strings that look like URLs (starting with `http://`, `https://`,
/// `git://`, `ssh://`, or containing `@` before `:`  — the SCP-style SSH syntax).
/// Canonicalizes relative paths to absolute via [`std::fs::canonicalize`].
///
/// # Errors
///
/// Returns [`SmeltError::Manifest`] if the path looks like a URL or cannot be
/// canonicalized (e.g., does not exist on disk).
pub fn resolve_repo_path(repo: &str) -> crate::Result<PathBuf> {
    let repo = repo.trim();

    // Reject URL-like strings
    for prefix in URL_PREFIXES {
        if repo.starts_with(prefix) {
            return Err(SmeltError::Manifest {
                field: "job.repo".to_string(),
                message: format!(
                    "repo must be a local path, not a URL: {repo:?}"
                ),
            });
        }
    }

    // Reject SCP-style SSH syntax: user@host:path
    if let Some(at_pos) = repo.find('@')
        && let Some(colon_pos) = repo.find(':')
            && at_pos < colon_pos {
                return Err(SmeltError::Manifest {
                    field: "job.repo".to_string(),
                    message: format!(
                        "repo must be a local path, not a URL: {repo:?}"
                    ),
                });
            }

    // Canonicalize (resolves relative paths, symlinks, verifies existence)
    std::fs::canonicalize(repo).map_err(|e| SmeltError::Manifest {
        field: "job.repo".to_string(),
        message: format!("cannot resolve repo path {repo:?}: {e}"),
    })
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

    /// Minimal compose manifest with two `[[services]]` entries:
    /// - `postgres` with extra fields covering all four extra-field types
    ///   (string, integer, boolean, array) to prove type fidelity.
    /// - `redis` bare (name + image only).
    const VALID_COMPOSE_MANIFEST: &str = r#"
[job]
name = "compose-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "compose"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "run"
spec = "Run the suite"
harness = "pytest"
timeout = 300

[merge]
strategy = "sequential"
target = "main"

[[services]]
name = "postgres"
image = "postgres:16"
port = 5432
restart = true
command = ["pg_isready", "-U", "postgres"]
tag = "db"

[[services]]
name = "redis"
image = "redis:7"
"#;

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

    // ── resolve_repo_path tests ─────────────────────────────────

    #[test]
    fn resolve_repo_path_valid_absolute() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let result = resolve_repo_path(path.to_str().unwrap()).unwrap();
        assert!(result.is_absolute());
        assert_eq!(result, std::fs::canonicalize(path).unwrap());
    }

    #[test]
    fn resolve_repo_path_rejects_http() {
        let err = resolve_repo_path("http://github.com/example/repo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not a URL"), "should reject http URL: {msg}");
    }

    #[test]
    fn resolve_repo_path_rejects_https() {
        let err = resolve_repo_path("https://github.com/example/repo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not a URL"), "should reject https URL: {msg}");
    }

    #[test]
    fn resolve_repo_path_rejects_git() {
        let err = resolve_repo_path("git://github.com/example/repo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not a URL"), "should reject git URL: {msg}");
    }

    #[test]
    fn resolve_repo_path_rejects_ssh() {
        let err = resolve_repo_path("ssh://git@github.com/example/repo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not a URL"), "should reject ssh URL: {msg}");
    }

    #[test]
    fn resolve_repo_path_rejects_scp_style() {
        let err = resolve_repo_path("git@github.com:example/repo").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not a URL"), "should reject SCP-style SSH: {msg}");
    }

    #[test]
    fn resolve_repo_path_relative_path() {
        // "." should resolve to the current working directory
        let result = resolve_repo_path(".").unwrap();
        assert!(result.is_absolute());
        assert_eq!(result, std::fs::canonicalize(".").unwrap());
    }

    #[test]
    fn resolve_repo_path_nonexistent() {
        let err = resolve_repo_path("/tmp/smelt-nonexistent-path-12345xyz").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cannot resolve repo path"), "should report nonexistent: {msg}");
    }

    #[test]
    fn resolve_repo_path_with_spaces() {
        let dir = tempfile::tempdir().unwrap();
        let spaced = dir.path().join("path with spaces");
        std::fs::create_dir_all(&spaced).unwrap();
        let result = resolve_repo_path(spaced.to_str().unwrap()).unwrap();
        assert!(result.is_absolute());
        assert_eq!(result, std::fs::canonicalize(&spaced).unwrap());
    }

    // ── forge field tests ─────────────────────────────────────────────────────

    const MANIFEST_WITH_FORGE: &str = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "frontend"
spec = "Implement the login page"
harness = "npm test"
timeout = 300

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "owner/my-repo"
token_env = "GITHUB_TOKEN"
"#;

    #[test]
    fn test_parse_manifest_with_forge() {
        let manifest = load_from_str(MANIFEST_WITH_FORGE).expect("should parse");
        let forge = manifest.forge.as_ref().expect("forge should be Some");
        assert_eq!(forge.provider, "github");
        assert_eq!(forge.repo, "owner/my-repo");
        assert_eq!(forge.token_env, "GITHUB_TOKEN");
    }

    #[test]
    fn test_parse_manifest_without_forge() {
        let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
        assert!(manifest.forge.is_none(), "forge should be None when no [forge] section");
    }

    #[test]
    fn test_validate_forge_invalid_repo_format() {
        let toml = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "no-slash"
token_env = "GITHUB_TOKEN"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("owner/repo"),
            "should report invalid repo format: {msg}"
        );
    }

    #[test]
    fn test_validate_forge_empty_token_env() {
        let toml = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "owner/repo"
token_env = ""
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("forge.token_env: must not be empty"),
            "should report empty token_env: {msg}"
        );
    }

    // ── ComposeService / services tests ──────────────────────────────────────

    #[test]
    fn test_compose_manifest_roundtrip_with_services() {
        let manifest = load_from_str(VALID_COMPOSE_MANIFEST).expect("should parse");
        assert_eq!(manifest.services.len(), 2);
        assert_eq!(manifest.services[0].name, "postgres");
        assert_eq!(manifest.services[0].image, "postgres:16");
        // extra keys are present
        assert!(manifest.services[0].extra.contains_key("port"), "extra should have 'port'");
        assert!(manifest.services[0].extra.contains_key("restart"), "extra should have 'restart'");
        assert!(manifest.services[0].extra.contains_key("command"), "extra should have 'command'");
        assert!(manifest.services[0].extra.contains_key("tag"), "extra should have 'tag'");
        // serde flatten must NOT capture name/image into extra
        assert!(!manifest.services[0].extra.contains_key("name"), "extra must not contain 'name'");
        assert!(!manifest.services[0].extra.contains_key("image"), "extra must not contain 'image'");
        // second service is bare
        assert_eq!(manifest.services[1].name, "redis");
        assert_eq!(manifest.services[1].image, "redis:7");
        assert!(manifest.services[1].extra.is_empty(), "redis extra should be empty");
    }

    #[test]
    fn test_compose_manifest_roundtrip_no_services() {
        // VALID_MANIFEST uses runtime = "docker" and has no [[services]] section
        let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
        assert!(manifest.services.is_empty(), "docker manifest should have no services");
    }

    #[test]
    fn test_compose_service_extra_does_not_contain_name_or_image() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[[services]]
name = "mydb"
image = "postgres:16"
port = 5432
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let svc = &manifest.services[0];
        assert!(!svc.extra.contains_key("name"), "serde flatten must exclude 'name' from extra");
        assert!(!svc.extra.contains_key("image"), "serde flatten must exclude 'image' from extra");
        assert!(svc.extra.contains_key("port"), "extra should have 'port'");
    }

    #[test]
    fn test_compose_service_passthrough_types() {
        let manifest = load_from_str(VALID_COMPOSE_MANIFEST).expect("should parse");
        let svc = &manifest.services[0]; // postgres with all extra types

        // integer
        let port = svc.extra.get("port").expect("port must be present");
        assert!(matches!(port, toml::Value::Integer(5432)), "port should be Integer(5432), got {port:?}");

        // boolean
        let restart = svc.extra.get("restart").expect("restart must be present");
        assert!(matches!(restart, toml::Value::Boolean(true)), "restart should be Boolean(true), got {restart:?}");

        // array
        let command = svc.extra.get("command").expect("command must be present");
        assert!(matches!(command, toml::Value::Array(_)), "command should be Array, got {command:?}");
        if let toml::Value::Array(arr) = command {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], toml::Value::String("pg_isready".to_string()));
        }

        // string
        let tag = svc.extra.get("tag").expect("tag must be present");
        assert!(matches!(tag, toml::Value::String(_)), "tag should be String, got {tag:?}");
    }

    #[test]
    fn test_validate_compose_service_missing_name() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[[services]]
name = ""
image = "img"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("services[0].name"), "should report empty service name: {msg}");
    }

    #[test]
    fn test_validate_compose_service_missing_image() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[[services]]
name = "svc"
image = ""
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("services[0].image"), "should report empty service image: {msg}");
    }

    #[test]
    fn test_validate_services_require_compose_runtime() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[[services]]
name = "db"
image = "postgres:16"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("services:"), "should report services error: {msg}");
        assert!(msg.contains("compose"), "should mention compose runtime: {msg}");
    }

    #[test]
    fn test_validate_compose_empty_services_allowed() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "compose"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        assert!(manifest.services.is_empty());
        manifest.validate().expect("compose with no services should be valid");
    }

    #[test]
    fn test_validate_runtime_unknown_rejected() {
        // "podman" is not a valid runtime — tests that unknown values are rejected
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "podman"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("environment.runtime"), "should report unknown runtime: {msg}");
    }

    // ── Kubernetes field tests ────────────────────────────────────────────────

    const KUBERNETES_MANIFEST: &str = r#"
[job]
name = "kube-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "agent"
spec = "Run the task"
harness = "kata run"
timeout = 600

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt-jobs"
context = "my-cluster"
ssh_key_env = "SSH_PRIVATE_KEY"
cpu_request = "500m"
memory_request = "512Mi"
cpu_limit = "2"
memory_limit = "2Gi"
"#;

    #[test]
    fn test_kubernetes_roundtrip_present() {
        let manifest = load_from_str(KUBERNETES_MANIFEST).expect("should parse");
        let kube = manifest.kubernetes.as_ref().expect("kubernetes should be Some");
        assert_eq!(kube.namespace, "smelt-jobs");
        assert_eq!(kube.context.as_deref(), Some("my-cluster"));
        assert_eq!(kube.ssh_key_env, "SSH_PRIVATE_KEY");
        assert_eq!(kube.cpu_request.as_deref(), Some("500m"));
        assert_eq!(kube.memory_request.as_deref(), Some("512Mi"));
        assert_eq!(kube.cpu_limit.as_deref(), Some("2"));
        assert_eq!(kube.memory_limit.as_deref(), Some("2Gi"));
    }

    #[test]
    fn test_kubernetes_roundtrip_absent() {
        // Standard docker-runtime manifest — kubernetes must be None
        let manifest = load_from_str(VALID_MANIFEST).expect("should parse");
        assert!(manifest.kubernetes.is_none(), "kubernetes should be None when no [kubernetes] section");
    }

    #[test]
    fn test_validate_kubernetes_runtime_requires_block() {
        // runtime = "kubernetes" but no [kubernetes] block → error
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("kubernetes"), "should report kubernetes error: {msg}");
    }

    #[test]
    fn test_validate_kubernetes_block_requires_runtime() {
        // runtime = "docker" + [kubernetes] block → error
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "docker"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt-jobs"
ssh_key_env = "SSH_PRIVATE_KEY"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("kubernetes"), "should report kubernetes error: {msg}");
    }

    #[test]
    fn test_validate_kubernetes_empty_namespace() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = ""
ssh_key_env = "SSH_PRIVATE_KEY"
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("namespace"), "should report namespace error: {msg}");
    }

    #[test]
    fn test_validate_kubernetes_empty_ssh_key_env() {
        let toml = r#"
[job]
name = "j"
repo = "r"
base_ref = "main"

[environment]
runtime = "kubernetes"
image = "img"

[credentials]
provider = "anthropic"
model = "m"

[[session]]
name = "s"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[kubernetes]
namespace = "smelt-jobs"
ssh_key_env = ""
"#;
        let manifest = load_from_str(toml).expect("should parse");
        let err = manifest.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ssh_key_env"), "should report ssh_key_env error: {msg}");
    }

    #[test]
    fn test_validate_kubernetes_valid() {
        let manifest = load_from_str(KUBERNETES_MANIFEST).expect("should parse");
        manifest.validate().expect("fully valid kubernetes manifest should pass validation");
    }

    #[test]
    fn test_validate_runtime_compose_valid() {
        // compose runtime + services should pass validate()
        let manifest = load_from_str(VALID_COMPOSE_MANIFEST).expect("should parse");
        manifest.validate().expect("compose manifest with services should be valid");
    }

    #[test]
    fn test_forge_deny_unknown_fields() {
        let toml = r#"
[job]
name = "test-job"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[session]]
name = "s1"
spec = "s"
harness = "h"
timeout = 60

[merge]
strategy = "sequential"
target = "main"

[forge]
provider = "github"
repo = "owner/repo"
token_env = "GITHUB_TOKEN"
unknown_field = "oops"
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown_field") || msg.contains("unknown field"),
            "should report unknown field in forge: {msg}"
        );
    }
}
