//! Project-level Smelt configuration loaded from `.smelt/config.toml`.
//!
//! [`SmeltConfig`] provides defaults that can be overridden by individual
//! job manifests. If no config file exists, sensible defaults are used.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::SmeltError;

/// Project-level configuration, loaded from `.smelt/config.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmeltConfig {
    /// Default container image when a manifest doesn't specify one.
    #[serde(default = "default_image")]
    pub default_image: String,

    /// Default credential environment variable sources.
    /// Key = logical credential name, Value = env var to read.
    #[serde(default)]
    pub credential_sources: HashMap<String, String>,

    /// Default resource limits applied when a manifest doesn't specify them.
    #[serde(default)]
    pub default_resources: HashMap<String, String>,

    /// Default timeout in seconds for sessions that don't specify one.
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,
}

fn default_image() -> String {
    "ubuntu:22.04".to_string()
}

fn default_timeout() -> u64 {
    600
}

impl Default for SmeltConfig {
    fn default() -> Self {
        Self {
            default_image: default_image(),
            credential_sources: HashMap::new(),
            default_resources: HashMap::new(),
            default_timeout: default_timeout(),
        }
    }
}

/// The canonical config file name within the `.smelt/` directory.
const CONFIG_FILENAME: &str = "config.toml";

impl SmeltConfig {
    /// Load configuration from `.smelt/config.toml` relative to `project_root`.
    ///
    /// If the file does not exist, returns [`SmeltConfig::default()`].
    /// If the file exists but cannot be parsed, returns a [`SmeltError::Config`].
    pub fn load(project_root: &Path) -> crate::Result<Self> {
        let config_path = project_root.join(".smelt").join(CONFIG_FILENAME);
        Self::load_from(&config_path)
    }

    /// Load configuration from an explicit file path.
    ///
    /// Returns defaults if the file does not exist.
    pub fn load_from(path: &Path) -> crate::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => Self::parse(&content, path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(SmeltError::config(path, format!("cannot read: {e}"))),
        }
    }

    /// Parse configuration from a TOML string.
    fn parse(content: &str, source: &Path) -> crate::Result<Self> {
        toml::from_str(content).map_err(|e| {
            SmeltError::config(source, format!("parse error: {e}"))
        })
    }

    /// The expected config file path for a given project root.
    pub fn config_path(project_root: &Path) -> PathBuf {
        project_root.join(".smelt").join(CONFIG_FILENAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = SmeltConfig::default();
        assert_eq!(cfg.default_image, "ubuntu:22.04");
        assert_eq!(cfg.default_timeout, 600);
        assert!(cfg.credential_sources.is_empty());
        assert!(cfg.default_resources.is_empty());
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let cfg = SmeltConfig::load_from(Path::new("/tmp/nonexistent-smelt-config-12345.toml"))
            .expect("missing file should return defaults");
        assert_eq!(cfg.default_image, "ubuntu:22.04");
        assert_eq!(cfg.default_timeout, 600);
    }

    #[test]
    fn parse_full_config() {
        let toml = r#"
default_image = "node:20-slim"
default_timeout = 900

[credential_sources]
api_key = "ANTHROPIC_API_KEY"
openai_key = "OPENAI_API_KEY"

[default_resources]
cpu = "4"
memory = "8G"
"#;
        let cfg = SmeltConfig::parse(toml, Path::new("test.toml")).unwrap();
        assert_eq!(cfg.default_image, "node:20-slim");
        assert_eq!(cfg.default_timeout, 900);
        assert_eq!(cfg.credential_sources.len(), 2);
        assert_eq!(
            cfg.credential_sources.get("api_key").unwrap(),
            "ANTHROPIC_API_KEY"
        );
        assert_eq!(cfg.default_resources.get("cpu").unwrap(), "4");
    }

    #[test]
    fn parse_minimal_config() {
        let toml = r#"
default_image = "alpine:3.19"
"#;
        let cfg = SmeltConfig::parse(toml, Path::new("test.toml")).unwrap();
        assert_eq!(cfg.default_image, "alpine:3.19");
        assert_eq!(cfg.default_timeout, 600); // default
        assert!(cfg.credential_sources.is_empty()); // default
    }

    #[test]
    fn parse_empty_config() {
        let cfg = SmeltConfig::parse("", Path::new("test.toml")).unwrap();
        assert_eq!(cfg.default_image, "ubuntu:22.04");
        assert_eq!(cfg.default_timeout, 600);
    }

    #[test]
    fn reject_unknown_fields_in_config() {
        let toml = r#"
default_image = "ubuntu:22.04"
bogus_field = "oops"
"#;
        let err = SmeltConfig::parse(toml, Path::new("test.toml")).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bogus_field") || msg.contains("unknown field"),
            "should reject unknown field: {msg}"
        );
    }

    #[test]
    fn load_from_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let smelt_dir = dir.path().join(".smelt");
        std::fs::create_dir_all(&smelt_dir).unwrap();
        std::fs::write(
            smelt_dir.join("config.toml"),
            "default_image = \"rust:1.85\"\n",
        )
        .unwrap();

        let cfg = SmeltConfig::load(dir.path()).expect("should load from .smelt/config.toml");
        assert_eq!(cfg.default_image, "rust:1.85");
    }

    #[test]
    fn load_nonexistent_project_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        // No .smelt/ directory at all
        let cfg = SmeltConfig::load(dir.path()).expect("should return defaults");
        assert_eq!(cfg.default_image, "ubuntu:22.04");
    }

    #[test]
    fn config_path_helper() {
        let root = Path::new("/home/user/project");
        let expected = PathBuf::from("/home/user/project/.smelt/config.toml");
        assert_eq!(SmeltConfig::config_path(root), expected);
    }
}
