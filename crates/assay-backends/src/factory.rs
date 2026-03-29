//! Factory function that maps [`StateBackendConfig`] to a concrete
//! [`StateBackend`] implementation.

use std::path::PathBuf;
use std::sync::Arc;

use assay_core::{LocalFsBackend, NoopBackend, StateBackend};
use assay_types::StateBackendConfig;

/// Create a [`StateBackend`] from the given configuration.
///
/// - `LocalFs` → [`LocalFsBackend`] rooted at `assay_dir`.
/// - `Linear` → `LinearBackend` when the `linear` feature is enabled;
///   falls back to [`NoopBackend`] otherwise — all state writes are discarded.
/// - `GitHub` → `GitHubBackend` when the `github` feature is enabled;
///   falls back to [`NoopBackend`] otherwise — all state writes are discarded.
/// - `Ssh` → `SshSyncBackend` when the `ssh` feature is enabled;
///   falls back to [`NoopBackend`] otherwise — all state writes are discarded.
/// - `Custom` → [`NoopBackend`] (no built-in implementation; all state writes are discarded).
pub fn backend_from_config(
    config: &StateBackendConfig,
    assay_dir: PathBuf,
) -> Arc<dyn StateBackend> {
    match config {
        StateBackendConfig::LocalFs => Arc::new(LocalFsBackend::new(assay_dir)),
        #[cfg(feature = "linear")]
        StateBackendConfig::Linear {
            team_id,
            project_id,
        } => match std::env::var("LINEAR_API_KEY") {
            Ok(api_key) => {
                let backend = crate::linear::LinearBackend::new(
                    api_key,
                    "https://api.linear.app".to_string(),
                    team_id.clone(),
                    project_id.clone(),
                );
                Arc::new(backend)
            }
            Err(_) => {
                tracing::warn!(
                    backend = "linear",
                    "LINEAR_API_KEY not set — falling back to NoopBackend; \
                         all state writes will be discarded"
                );
                Arc::new(NoopBackend)
            }
        },
        #[cfg(not(feature = "linear"))]
        StateBackendConfig::Linear { .. } => {
            tracing::warn!(
                backend = "linear",
                "LinearBackend requires the `linear` feature — falling back to NoopBackend; \
                 all state writes will be discarded"
            );
            Arc::new(NoopBackend)
        }
        #[cfg(feature = "github")]
        StateBackendConfig::GitHub { repo, label } => {
            let backend = crate::github::GitHubBackend::new(repo.clone(), label.clone());
            Arc::new(backend)
        }
        #[cfg(not(feature = "github"))]
        StateBackendConfig::GitHub { .. } => {
            tracing::warn!(
                backend = "github",
                "GitHubBackend requires the `github` feature — falling back to NoopBackend; \
                 all state writes will be discarded"
            );
            Arc::new(NoopBackend)
        }
        #[cfg(feature = "ssh")]
        StateBackendConfig::Ssh {
            host,
            remote_assay_dir,
            user,
            port,
        } => Arc::new(crate::ssh::SshSyncBackend::new(
            host.clone(),
            remote_assay_dir.clone(),
            user.clone(),
            *port,
            assay_dir,
        )),
        #[cfg(not(feature = "ssh"))]
        StateBackendConfig::Ssh { .. } => {
            tracing::warn!(
                backend = "ssh",
                "SshSyncBackend requires the `ssh` feature — falling back to NoopBackend; \
                 all state writes will be discarded"
            );
            Arc::new(NoopBackend)
        }
        #[cfg(feature = "smelt")]
        StateBackendConfig::Smelt { url, job_id, token } => Arc::new(
            crate::smelt::SmeltBackend::new(url.clone(), job_id.clone(), token.clone()),
        ),
        #[cfg(not(feature = "smelt"))]
        StateBackendConfig::Smelt { .. } => {
            tracing::warn!(
                backend = "smelt",
                "SmeltBackend requires the `smelt` feature — falling back to NoopBackend; \
                 all state writes will be discarded"
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
        // LocalFs supports everything except signals.
        assert_eq!(
            caps,
            CapabilitySet {
                supports_messaging: true,
                supports_gossip_manifest: true,
                supports_annotations: true,
                supports_checkpoints: true,
                supports_signals: false,
                supports_peer_registry: true,
            }
        );
    }

    #[test]
    fn factory_linear_capabilities() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::Linear {
            team_id: "TEAM".into(),
            project_id: Some("PROJ".into()),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());

        #[cfg(feature = "linear")]
        {
            if std::env::var("LINEAR_API_KEY").is_ok() {
                // D164 flags: messaging=false, gossip_manifest=false, annotations=true, checkpoints=false
                let caps = backend.capabilities();
                assert_eq!(
                    caps,
                    CapabilitySet {
                        supports_messaging: false,
                        supports_gossip_manifest: false,
                        supports_annotations: true,
                        supports_checkpoints: false,
                        supports_signals: false,
                        supports_peer_registry: false,
                    }
                );
            } else {
                // Falls back to NoopBackend when LINEAR_API_KEY is absent.
                assert_eq!(backend.capabilities(), CapabilitySet::none());
            }
        }
        #[cfg(not(feature = "linear"))]
        {
            assert_eq!(backend.capabilities(), CapabilitySet::none());
        }
    }

    #[test]
    fn factory_github_capabilities() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::GitHub {
            repo: "user/repo".into(),
            label: Some("assay".into()),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());
        // GitHubBackend has all-false capabilities (same as NoopBackend).
        // With the `github` feature enabled, the real GitHubBackend is used;
        // without it, NoopBackend is returned. Either way, capabilities are none().
        assert_eq!(backend.capabilities(), CapabilitySet::none());
    }

    #[test]
    fn factory_ssh_capabilities() {
        let dir = tempfile::tempdir().unwrap();
        let config = StateBackendConfig::Ssh {
            host: "server.example.com".into(),
            remote_assay_dir: "/home/user/.assay".into(),
            user: Some("deploy".into()),
            port: Some(2222),
        };
        let backend = backend_from_config(&config, dir.path().to_path_buf());
        #[cfg(feature = "ssh")]
        assert_eq!(
            backend.capabilities(),
            CapabilitySet {
                supports_messaging: true,
                supports_gossip_manifest: true,
                supports_annotations: true,
                supports_checkpoints: true,
                supports_signals: false,
                supports_peer_registry: false,
            }
        );
        #[cfg(not(feature = "ssh"))]
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
