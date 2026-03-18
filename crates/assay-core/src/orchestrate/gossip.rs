//! Stub executor for Gossip mode.
//!
//! `run_gossip()` accepts the same signature as `run_orchestrated()` but
//! returns an empty `OrchestratorResult` immediately. Sessions with non-empty
//! `depends_on` emit a `tracing::warn!` because dependency edges are ignored
//! in Gossip mode.

use std::time::Duration;

use ulid::Ulid;

use assay_types::ManifestSession;

use crate::error::AssayError;
use crate::orchestrate::executor::{OrchestratorConfig, OrchestratorResult};
use crate::pipeline::{PipelineConfig, PipelineError, PipelineResult};

/// Stub for Gossip-mode execution.
///
/// This is a placeholder that will be replaced by a real epidemic-gossip
/// executor in a future milestone. For now it:
/// - warns per session if `depends_on` is non-empty (dependency edges are
///   ignored in Gossip mode)
/// - returns a valid `OrchestratorResult` with zero outcomes
pub fn run_gossip<F>(
    manifest: &assay_types::RunManifest,
    config: &OrchestratorConfig,
    _pipeline_config: &PipelineConfig,
    _session_runner: &F,
) -> Result<OrchestratorResult, AssayError>
where
    F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync,
{
    for session in &manifest.sessions {
        if !session.depends_on.is_empty() {
            tracing::warn!(
                session = session.name.as_deref().unwrap_or(&session.spec),
                "depends_on is ignored in Gossip mode"
            );
        }
    }
    Ok(OrchestratorResult {
        run_id: Ulid::new().to_string(),
        outcomes: vec![],
        duration: Duration::ZERO,
        failure_policy: config.failure_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{ManifestSession, OrchestratorMode, RunManifest};

    fn make_runner()
    -> impl Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync
    {
        |_session, _config| panic!("stub should not call session_runner")
    }

    fn make_session(spec: &str, depends_on: Vec<&str>) -> ManifestSession {
        ManifestSession {
            spec: spec.to_string(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: depends_on.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_manifest(sessions: Vec<ManifestSession>) -> RunManifest {
        RunManifest {
            sessions,
            mode: OrchestratorMode::Gossip,
            mesh_config: None,
            gossip_config: None,
        }
    }

    fn make_pipeline_config() -> PipelineConfig {
        PipelineConfig {
            project_root: std::path::PathBuf::from("/tmp"),
            assay_dir: std::path::PathBuf::from("/tmp/.assay"),
            specs_dir: std::path::PathBuf::from("/tmp/.assay/specs"),
            worktree_base: std::path::PathBuf::from("/tmp/worktrees"),
            timeout_secs: 60,
            base_branch: None,
        }
    }

    #[test]
    fn run_gossip_returns_empty_result() {
        let manifest = make_manifest(vec![make_session("some-spec", vec![])]);
        let config = OrchestratorConfig::default();
        let result =
            run_gossip(&manifest, &config, &make_pipeline_config(), &make_runner()).unwrap();
        assert!(result.outcomes.is_empty());
        assert_eq!(result.duration, Duration::ZERO);
    }

    #[test]
    fn run_gossip_emits_warn_for_depends_on() {
        // Constructing a session with depends_on — warn is emitted but stub
        // still returns Ok with empty outcomes (observable via tracing subscriber
        // in integration contexts).
        let manifest = make_manifest(vec![make_session("some-spec", vec!["other"])]);
        let config = OrchestratorConfig::default();
        let result =
            run_gossip(&manifest, &config, &make_pipeline_config(), &make_runner()).unwrap();
        assert!(
            result.outcomes.is_empty(),
            "stub always returns zero outcomes"
        );
    }
}
