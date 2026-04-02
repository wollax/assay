//! Job manifest types with strict TOML parsing and validation.
//!
//! A job manifest describes a complete Smelt job: what repo to work on,
//! what container image to use, which sessions to run, and how to merge results.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::Deserialize;

use crate::error::SmeltError;
use crate::forge::ForgeConfig;
use crate::tracker::StateBackendConfig;

mod validation;
pub use validation::ValidationErrors;

#[cfg(test)]
mod tests;

/// Configuration for the Kubernetes runtime provider.
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct JobManifest {
    /// Job metadata.
    pub job: JobMeta,

    /// Container environment configuration.
    pub environment: Environment,

    /// LLM credential configuration.
    pub credentials: CredentialConfig,

    /// Session definitions. Empty after deserialization of template manifests;
    /// `validate()` enforces at least one is present for execution manifests.
    #[serde(default)]
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

    /// Optional state-backend configuration (mirrors Assay's `StateBackendConfig`).
    ///
    /// When present, Smelt passes this through to Assay for state persistence.
    /// See D154 for the passthrough design.
    #[serde(default)]
    pub state_backend: Option<StateBackendConfig>,

    /// Computed environment variables injected into containers at dispatch time.
    ///
    /// Unlike `credentials.env` (which maps logical names to host env var names
    /// for lookup), these are direct key=value pairs set programmatically —
    /// e.g. `SMELT_EVENT_URL`, `SMELT_JOB_ID`. Not deserialized from TOML.
    #[serde(skip)]
    pub runtime_env: HashMap<String, String>,

    /// Declarative cross-job PeerUpdate routing rules (D179).
    ///
    /// When a session-completion event is ingested for this job, Smelt evaluates
    /// these rules and delivers a `PeerUpdate` signal to each matching target job.
    /// Populated from `[[notify]]` TOML array syntax.
    #[serde(default)]
    pub notify: Vec<NotifyRule>,
}

/// A single cross-job notification rule (D179).
///
/// Declared as `[[notify]]` in the manifest TOML:
/// ```toml
/// [[notify]]
/// target_job = "frontend"
/// on_session_complete = true
/// ```
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NotifyRule {
    /// Name of the job to notify when the trigger condition fires.
    pub target_job: String,
    /// Fire when the source job's Assay session completes (phase == "complete").
    #[serde(default)]
    pub on_session_complete: bool,
}

/// `[job]` — high-level job metadata.
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
pub struct ComposeService {
    /// Service name used to identify this container within the compose network.
    pub name: String,

    /// Container image reference (e.g. `"postgres:16"`).
    pub image: String,

    /// All remaining Compose keys for this service (ports, volumes, env, etc.).
    #[serde(flatten)]
    pub extra: IndexMap<String, toml::Value>,
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
        validation::validate_manifest(self)
    }

    /// Resolve credential sources from the environment.
    ///
    /// Returns a map of logical name → [`CredentialStatus`]. Env vars with a
    /// valid UTF-8 value produce [`CredentialStatus::Resolved`]; absent vars
    /// produce [`CredentialStatus::Missing`]. Non-UTF-8 values also produce
    /// `Missing` (with a `source` annotation of `"env:VAR (non-UTF-8 value)"`)
    /// and emit a `tracing::warn!` — callers should not rely on silent resolution
    /// when env vars may contain raw bytes.
    pub fn resolve_credentials(&self) -> HashMap<String, CredentialStatus> {
        let mut results = HashMap::new();

        for (logical_name, env_var) in &self.credentials.env {
            let status = match std::env::var(env_var) {
                Ok(_) => CredentialStatus::Resolved {
                    source: format!("env:{env_var}"),
                },
                Err(std::env::VarError::NotPresent) => CredentialStatus::Missing {
                    source: format!("env:{env_var}"),
                },
                Err(std::env::VarError::NotUnicode(_)) => {
                    tracing::warn!(
                        env_var = %env_var,
                        "credential env var contains non-UTF-8 bytes and cannot be read"
                    );
                    CredentialStatus::Missing {
                        source: format!("env:{env_var} (non-UTF-8 value)"),
                    }
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
                message: format!("repo must be a local path, not a URL: {repo:?}"),
            });
        }
    }

    // Reject SCP-style SSH syntax: user@host:path
    if let Some(at_pos) = repo.find('@')
        && let Some(colon_pos) = repo.find(':')
        && at_pos < colon_pos
    {
        return Err(SmeltError::Manifest {
            field: "job.repo".to_string(),
            message: format!("repo must be a local path, not a URL: {repo:?}"),
        });
    }

    // Canonicalize (resolves relative paths, symlinks, verifies existence)
    std::fs::canonicalize(repo).map_err(|e| SmeltError::Manifest {
        field: "job.repo".to_string(),
        message: format!("cannot resolve repo path {repo:?}: {e}"),
    })
}
