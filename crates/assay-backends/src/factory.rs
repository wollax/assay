//! Factory function that maps [`StateBackendConfig`] to a concrete
//! [`StateBackend`] implementation.

use std::path::PathBuf;
use std::sync::Arc;

use assay_core::{LocalFsBackend, NoopBackend, StateBackend};
use assay_types::StateBackendConfig;

/// Create a [`StateBackend`] from the given configuration.
///
/// - `LocalFs` → [`LocalFsBackend`] rooted at `assay_dir`.
/// - `Linear`, `GitHub`, `Ssh`, `Custom` → [`NoopBackend`] (stub — real
///   implementations are added by later slices behind feature flags).
pub fn backend_from_config(
    config: &StateBackendConfig,
    assay_dir: PathBuf,
) -> Arc<dyn StateBackend> {
    match config {
        StateBackendConfig::LocalFs => Arc::new(LocalFsBackend::new(assay_dir)),
        StateBackendConfig::Linear { .. }
        | StateBackendConfig::GitHub { .. }
        | StateBackendConfig::Ssh { .. }
        | StateBackendConfig::Custom { .. } => Arc::new(NoopBackend),
    }
}
