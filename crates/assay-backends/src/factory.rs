//! Factory function that maps [`StateBackendConfig`] to a concrete
//! [`StateBackend`] implementation.

use std::path::PathBuf;
use std::sync::Arc;

use assay_core::{LocalFsBackend, NoopBackend, StateBackend};
use assay_types::StateBackendConfig;

/// Create a [`StateBackend`] from the given configuration.
///
/// - `LocalFs` → [`LocalFsBackend`] rooted at `assay_dir`.
/// - `Linear` → `LinearBackend` when the `linear` feature is enabled (M011/S02);
///   falls back to [`NoopBackend`] otherwise — all state writes are discarded.
/// - `GitHub` → `GitHubBackend` when the `github` feature is enabled (M011/S03);
///   falls back to [`NoopBackend`] otherwise — all state writes are discarded.
/// - `Ssh` → `SshSyncBackend` when the `ssh` feature is enabled (M011/S04);
///   falls back to [`NoopBackend`] otherwise — all state writes are discarded.
/// - `Custom` → [`NoopBackend`] (no built-in implementation; all state writes are discarded).
pub fn backend_from_config(
    config: &StateBackendConfig,
    assay_dir: PathBuf,
) -> Arc<dyn StateBackend> {
    match config {
        StateBackendConfig::LocalFs => Arc::new(LocalFsBackend::new(assay_dir)),
        StateBackendConfig::Linear { .. } => {
            tracing::warn!(
                backend = "linear",
                "LinearBackend is not yet implemented — falling back to NoopBackend; \
                 all state writes will be discarded (stub pending M011/S02)"
            );
            Arc::new(NoopBackend)
        }
        StateBackendConfig::GitHub { .. } => {
            tracing::warn!(
                backend = "github",
                "GitHubBackend is not yet implemented — falling back to NoopBackend; \
                 all state writes will be discarded (stub pending M011/S03)"
            );
            Arc::new(NoopBackend)
        }
        StateBackendConfig::Ssh { .. } => {
            tracing::warn!(
                backend = "ssh",
                "SshSyncBackend is not yet implemented — falling back to NoopBackend; \
                 all state writes will be discarded (stub pending M011/S04)"
            );
            Arc::new(NoopBackend)
        }
        StateBackendConfig::Custom { name, .. } => {
            tracing::warn!(
                backend = %name,
                "Custom backend '{}' has no built-in implementation — falling back to NoopBackend; \
                 all state writes will be discarded",
                name
            );
            Arc::new(NoopBackend)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_core::CapabilitySet;

    #[test]
    fn factory_local_fs_returns_full_capabilities() {
        let dir = tempfile::tempdir().unwrap();
        let backend = backend_from_config(&StateBackendConfig::LocalFs, dir.path().to_path_buf());
        let caps = backend.capabilities();
        assert_eq!(caps, CapabilitySet::all());
    }

    #[test]
    fn factory_linear_returns_noop() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::Linear {
            team_id: "TEAM".into(),
            project_id: Some("PROJ".into()),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());
        assert_eq!(backend.capabilities(), CapabilitySet::none());
    }

    #[test]
    fn factory_github_returns_noop() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::GitHub {
            repo: "user/repo".into(),
            label: Some("assay".into()),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());
        assert_eq!(backend.capabilities(), CapabilitySet::none());
    }

    #[test]
    fn factory_ssh_returns_noop() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::Ssh {
            host: "server.example.com".into(),
            remote_assay_dir: "/home/user/.assay".into(),
            user: Some("deploy".into()),
            port: Some(2222),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());
        assert_eq!(backend.capabilities(), CapabilitySet::none());
    }

    #[test]
    fn factory_custom_returns_noop() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::Custom {
            name: "my-backend".into(),
            config: serde_json::json!({"key": "value"}),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());
        assert_eq!(backend.capabilities(), CapabilitySet::none());
    }
}
