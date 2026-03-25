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
}

impl ServerConfig {
    /// Load and validate a `ServerConfig` from a TOML file at `path`.
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is malformed or contains unknown fields
    /// - `max_concurrent` is zero
    /// - `server.port` is zero
    pub fn load(path: &std::path::Path) -> anyhow::Result<ServerConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read config file {}: {}", path.display(), e))?;
        let config: ServerConfig = toml::from_str(&content).map_err(|e| {
            anyhow::anyhow!("failed to parse config file {}: {}", path.display(), e)
        })?;
        config.validate()?;
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

        Ok(())
    }
}
