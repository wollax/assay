use std::path::PathBuf;

use serde::Deserialize;

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8765
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_backoff_secs() -> u64 {
    5
}

fn default_workers() -> Vec<WorkerConfig> {
    vec![]
}

fn default_poll_interval_secs() -> u64 {
    30
}

fn default_label_prefix() -> String {
    "smelt".to_string()
}

fn default_ssh_timeout_secs() -> u64 {
    3
}

fn default_ssh_port() -> u16 {
    22
}

/// Configuration for a single SSH worker host.
///
/// Each entry under `[[workers]]` in `server.toml` maps to one `WorkerConfig`.
/// `key_env` stores the *name* of an environment variable that holds the path
/// to the SSH private key — never the key value itself.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    pub host: String,
    pub user: String,
    /// Name of the env var that holds the path to the SSH private key.
    pub key_env: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
}

/// Network binding settings for the HTTP API server.
///
/// Defaults to `127.0.0.1:8765` when omitted from the config file.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerNetworkConfig {
    /// IP address to bind the HTTP listener to (default `127.0.0.1`).
    #[serde(default = "default_host")]
    pub host: String,
    /// TCP port for the HTTP listener (default `8765`).
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerNetworkConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Optional bearer-token authentication configuration.
///
/// When the `[auth]` section is present in `server.toml`, the HTTP API
/// enforces bearer-token authentication with an optional read/write
/// permission split.  Token values are **never** stored in config —
/// only the names of environment variables that hold them.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct AuthConfig {
    /// Name of the env var holding the read-write (full access) token.
    pub write_token_env: String,
    /// Name of the env var holding the read-only token (optional).
    /// When omitted, only the write token grants any access.
    pub read_token_env: Option<String>,
}

/// Tracker-integration configuration for polling issues from an external
/// tracker (Linear, GitHub) and converting them into Smelt jobs.
///
/// Appears as `[tracker]` in `server.toml`. When absent, tracker polling
/// is disabled.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TrackerConfig {
    /// Tracker provider — must be `"github"` or `"linear"`.
    pub provider: String,
    /// Path to the template manifest used to generate job manifests from
    /// tracker issues. Validated at startup (D017).
    pub manifest_template: PathBuf,
    /// How often (in seconds) to poll the tracker for ready issues.
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
    /// Prefix for lifecycle labels (e.g. `"smelt"` → `"smelt:ready"`).
    #[serde(default = "default_label_prefix")]
    pub label_prefix: String,
    /// Default harness name injected into generated manifests.
    pub default_harness: String,
    /// Default timeout in seconds for generated jobs.
    pub default_timeout: u64,
}

/// Top-level server configuration loaded from `server.toml`.
///
/// Controls concurrency limits, retry policy, network binding, SSH worker
/// pool, and the on-disk queue directory.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Directory where the persistent queue state file is stored.
    pub queue_dir: PathBuf,
    /// Maximum number of jobs that may execute concurrently.
    pub max_concurrent: usize,
    /// How many times a failed job is retried before it is marked `Failed`.
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    /// Seconds to wait between retry attempts (exponential back-off base).
    ///
    /// Deserialized from config but not yet consumed by the dispatch loop;
    /// kept for forward-compatibility so existing `server.toml` files remain
    /// valid when retry back-off is wired up.
    #[serde(default = "default_retry_backoff_secs")]
    #[allow(dead_code)]
    pub retry_backoff_secs: u64,
    /// HTTP API network binding (host + port).
    #[serde(default)]
    pub server: ServerNetworkConfig,
    /// SSH worker pool. When present, `smelt serve` dispatches jobs to these
    /// remote hosts instead of running them locally.
    #[serde(default = "default_workers")]
    pub workers: Vec<WorkerConfig>,
    /// Timeout in seconds for SSH connection attempts to worker hosts.
    #[serde(default = "default_ssh_timeout_secs")]
    pub ssh_timeout_secs: u64,
    /// Optional bearer-token authentication for the HTTP API.
    /// When absent, the API is unauthenticated (backward-compatible).
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    /// Optional tracker integration for polling issues and converting them
    /// to Smelt jobs. When absent, tracker polling is disabled.
    #[serde(default)]
    pub tracker: Option<TrackerConfig>,
}

impl ServerConfig {
    /// Load and validate a `ServerConfig` from a TOML file at `path`.
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is malformed or contains unknown fields
    /// - `max_concurrent` is zero
    /// - `server.port` is zero
    /// - A `[tracker]` section is present but its `manifest_template` is
    ///   invalid (D017: validate at startup, not at dispatch time)
    pub fn load(path: &std::path::Path) -> anyhow::Result<ServerConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read config file {}: {}", path.display(), e))?;
        let config: ServerConfig = toml::from_str(&content).map_err(|e| {
            anyhow::anyhow!("failed to parse config file {}: {}", path.display(), e)
        })?;
        config.validate()?;

        // Validate template manifest at startup (D017).
        if let Some(ref tracker) = config.tracker {
            super::tracker::load_template_manifest(&tracker.manifest_template)?;
        }

        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.max_concurrent == 0 {
            anyhow::bail!("max_concurrent must be at least 1");
        }
        if self.server.port == 0 {
            anyhow::bail!("server.port must be non-zero");
        }

        // Collect all worker validation errors before returning (D018).
        let mut worker_errors: Vec<String> = Vec::new();
        for (i, w) in self.workers.iter().enumerate() {
            if w.host.trim().is_empty() {
                worker_errors.push(format!("worker[{i}]: host must not be empty"));
            }
            if w.user.trim().is_empty() {
                worker_errors.push(format!("worker[{i}]: user must not be empty"));
            }
        }
        if !worker_errors.is_empty() {
            anyhow::bail!(
                "invalid worker configuration:\n  {}",
                worker_errors.join("\n  ")
            );
        }

        // Collect tracker validation errors (D018).
        if let Some(ref tracker) = self.tracker {
            let mut tracker_errors: Vec<String> = Vec::new();
            if tracker.provider != "github" && tracker.provider != "linear" {
                tracker_errors.push(format!(
                    "provider must be \"github\" or \"linear\", got \"{}\"",
                    tracker.provider
                ));
            }
            if tracker.poll_interval_secs == 0 {
                tracker_errors.push("poll_interval_secs must be > 0".into());
            }
            if tracker.default_timeout == 0 {
                tracker_errors.push("default_timeout must be > 0".into());
            }
            if tracker.default_harness.trim().is_empty() {
                tracker_errors.push("default_harness must not be empty".into());
            }
            if tracker.label_prefix.trim().is_empty() {
                tracker_errors.push("label_prefix must not be empty".into());
            }
            if tracker.manifest_template.as_os_str().is_empty() {
                tracker_errors.push("manifest_template must not be empty".into());
            }
            if !tracker_errors.is_empty() {
                anyhow::bail!(
                    "invalid tracker configuration:\n  {}",
                    tracker_errors.join("\n  ")
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Valid template manifest TOML (zero sessions) used by tracker config tests.
    const TEMPLATE_MANIFEST_TOML: &str = r#"
[job]
name = "template"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

[credentials]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[merge]
strategy = "sequential"
target = "main"
"#;

    /// Write a template manifest to a temp file and return it (keeps it alive).
    fn write_template_file() -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(TEMPLATE_MANIFEST_TOML.as_bytes()).unwrap();
        f
    }

    /// Helper: write TOML content to a temp file and load it as `ServerConfig`.
    fn load_from_str(toml_content: &str) -> anyhow::Result<ServerConfig> {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();
        ServerConfig::load(f.path())
    }

    /// Helper: build server config TOML with a tracker section pointing at the
    /// given template file path.
    fn with_tracker_toml(template_path: &std::path::Path) -> String {
        format!(
            r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "github"
manifest_template = "{}"
default_harness = "bash"
default_timeout = 600
"#,
            template_path.display()
        )
    }

    /// Minimal valid config without a `[tracker]` section — must still parse.
    const MINIMAL: &str = r#"
queue_dir = "/tmp/q"
max_concurrent = 2
"#;

    #[test]
    fn tracker_section_parses_correctly() {
        let template = write_template_file();
        let toml = with_tracker_toml(template.path());
        let cfg = load_from_str(&toml).unwrap();
        let t = cfg.tracker.as_ref().unwrap();
        assert_eq!(t.provider, "github");
        assert_eq!(t.poll_interval_secs, 30); // default
        assert_eq!(t.label_prefix, "smelt"); // default
        assert_eq!(t.default_harness, "bash");
        assert_eq!(t.default_timeout, 600);
    }

    #[test]
    fn missing_tracker_section_still_works() {
        let cfg = load_from_str(MINIMAL).unwrap();
        assert!(cfg.tracker.is_none());
    }

    #[test]
    fn tracker_invalid_provider_rejected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "jira"
manifest_template = "/tmp/t.tera"
default_harness = "bash"
default_timeout = 600
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("provider must be"), "got: {msg}");
    }

    #[test]
    fn tracker_zero_poll_interval_rejected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "linear"
manifest_template = "/tmp/t.tera"
default_harness = "bash"
default_timeout = 600
poll_interval_secs = 0
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("poll_interval_secs must be > 0"), "got: {msg}");
    }

    #[test]
    fn tracker_zero_default_timeout_rejected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "github"
manifest_template = "/tmp/t.tera"
default_harness = "bash"
default_timeout = 0
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("default_timeout must be > 0"), "got: {msg}");
    }

    #[test]
    fn tracker_empty_default_harness_rejected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "github"
manifest_template = "/tmp/t.tera"
default_harness = ""
default_timeout = 600
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("default_harness must not be empty"),
            "got: {msg}"
        );
    }

    #[test]
    fn tracker_empty_label_prefix_rejected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "github"
manifest_template = "/tmp/t.tera"
default_harness = "bash"
default_timeout = 600
label_prefix = "  "
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("label_prefix must not be empty"), "got: {msg}");
    }

    #[test]
    fn tracker_empty_manifest_template_rejected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "github"
manifest_template = ""
default_harness = "bash"
default_timeout = 600
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("manifest_template must not be empty"),
            "got: {msg}"
        );
    }

    #[test]
    fn tracker_multiple_errors_collected() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "jira"
manifest_template = "/tmp/t.tera"
default_harness = ""
default_timeout = 0
poll_interval_secs = 0
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        // All errors collected in one message (D018)
        assert!(msg.contains("provider must be"), "got: {msg}");
        assert!(msg.contains("poll_interval_secs"), "got: {msg}");
        assert!(msg.contains("default_timeout"), "got: {msg}");
        assert!(msg.contains("default_harness"), "got: {msg}");
    }

    #[test]
    fn tracker_bad_template_rejected_at_startup() {
        // Template with sessions should be rejected at ServerConfig::load() time (D017)
        let mut template = NamedTempFile::new().unwrap();
        template
            .write_all(
                br#"
[job]
name = "bad"
repo = "https://github.com/example/repo"
base_ref = "main"

[environment]
runtime = "docker"
image = "ubuntu:22.04"

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
"#,
            )
            .unwrap();

        let toml = with_tracker_toml(template.path());
        let err = load_from_str(&toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must not contain [[session]] entries"),
            "got: {msg}"
        );
        // Error message should include the file path
        assert!(
            msg.contains(template.path().to_str().unwrap()),
            "error should mention file path, got: {msg}"
        );
    }

    #[test]
    fn tracker_nonexistent_template_rejected_at_startup() {
        let toml = r#"
queue_dir = "/tmp/q"
max_concurrent = 2

[tracker]
provider = "github"
manifest_template = "/nonexistent/template.toml"
default_harness = "bash"
default_timeout = 600
"#;
        let err = load_from_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("failed to load template manifest"),
            "got: {msg}"
        );
    }

    #[test]
    fn test_server_config_with_tracker_section() {
        // Full round-trip: valid server config with tracker pointing at valid template
        let template = write_template_file();
        let toml = with_tracker_toml(template.path());
        let cfg = load_from_str(&toml).unwrap();
        assert!(cfg.tracker.is_some());
        assert_eq!(cfg.max_concurrent, 2);
        assert_eq!(cfg.tracker.as_ref().unwrap().provider, "github");
    }
}
