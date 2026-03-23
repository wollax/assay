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

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerNetworkConfig {
    #[serde(default = "default_host")]
    pub host: String,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub queue_dir: PathBuf,
    pub max_concurrent: usize,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_backoff_secs")]
    #[allow(dead_code)] // used in future dispatch retry logic
    pub retry_backoff_secs: u64,
    #[serde(default)]
    pub server: ServerNetworkConfig,
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
        let config: ServerConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse config file {}: {}", path.display(), e))?;
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
        Ok(())
    }
}
