//! `smelt run` subcommand — execute a job manifest.

mod dry_run;
mod helpers;
mod phases;

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

pub use phases::run_with_cancellation;

/// Run a job manifest.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Path to the job manifest TOML file.
    pub manifest: PathBuf,

    /// Validate and print the execution plan without running anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip PR creation even when a `[forge]` section is present in the manifest.
    #[arg(long)]
    pub no_pr: bool,

    /// Computed environment variables injected into the **container** at provision time.
    ///
    /// Set programmatically by `smelt serve` dispatch (not exposed on CLI).
    /// Merged into `manifest.runtime_env` before provision. Transient — never
    /// serialized to disk or manifest files. Values cross a security boundary
    /// (host → container).
    #[arg(skip)]
    pub runtime_env: std::collections::HashMap<String, String>,

    /// Optional channel to report the container IP discovered at provision time.
    ///
    /// Used by `smelt serve` dispatch to cache signal URLs in `ServerState`.
    /// `None` for `smelt run` (CLI) which doesn't need signal URL caching.
    /// Wrapped in `Mutex` because `RunArgs` is passed by shared reference to
    /// `run_with_cancellation`, and `Sender::send` consumes the sender.
    #[arg(skip)]
    pub container_ip_tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<String>>>,
}

// ── AnyProvider ──────────────────────────────────────────────────────────────

/// Dispatch enum that routes [`RuntimeProvider`](smelt_core::provider::RuntimeProvider)
/// calls to the concrete backend selected by `manifest.environment.runtime`.
///
/// A local enum avoids `Box<dyn RuntimeProvider>`, which is not object-safe
/// because `RuntimeProvider` has RPITIT `async fn` methods (see D019).
enum AnyProvider {
    /// Docker single-container runtime.
    Docker(smelt_core::DockerProvider),
    /// Docker Compose multi-service runtime.
    Compose(smelt_core::ComposeProvider),
    /// Kubernetes Pod-based runtime.
    Kubernetes(smelt_core::KubernetesProvider),
}

impl smelt_core::provider::RuntimeProvider for AnyProvider {
    async fn provision(
        &self,
        manifest: &smelt_core::manifest::JobManifest,
    ) -> smelt_core::Result<smelt_core::provider::ProvisionResult> {
        match self {
            AnyProvider::Docker(p) => p.provision(manifest).await,
            AnyProvider::Compose(p) => p.provision(manifest).await,
            AnyProvider::Kubernetes(p) => p.provision(manifest).await,
        }
    }

    async fn exec(
        &self,
        container: &smelt_core::provider::ContainerId,
        command: &[String],
    ) -> smelt_core::Result<smelt_core::provider::ExecHandle> {
        match self {
            AnyProvider::Docker(p) => p.exec(container, command).await,
            AnyProvider::Compose(p) => p.exec(container, command).await,
            AnyProvider::Kubernetes(p) => p.exec(container, command).await,
        }
    }

    async fn exec_streaming<F>(
        &self,
        container: &smelt_core::provider::ContainerId,
        command: &[String],
        output_cb: F,
    ) -> smelt_core::Result<smelt_core::provider::ExecHandle>
    where
        F: FnMut(&str) + Send + 'static,
    {
        match self {
            AnyProvider::Docker(p) => p.exec_streaming(container, command, output_cb).await,
            AnyProvider::Compose(p) => p.exec_streaming(container, command, output_cb).await,
            AnyProvider::Kubernetes(p) => p.exec_streaming(container, command, output_cb).await,
        }
    }

    async fn collect(
        &self,
        container: &smelt_core::provider::ContainerId,
        manifest: &smelt_core::manifest::JobManifest,
    ) -> smelt_core::Result<smelt_core::provider::CollectResult> {
        match self {
            AnyProvider::Docker(p) => p.collect(container, manifest).await,
            AnyProvider::Compose(p) => p.collect(container, manifest).await,
            AnyProvider::Kubernetes(p) => p.collect(container, manifest).await,
        }
    }

    async fn teardown(
        &self,
        container: &smelt_core::provider::ContainerId,
    ) -> smelt_core::Result<()> {
        match self {
            AnyProvider::Docker(p) => p.teardown(container).await,
            AnyProvider::Compose(p) => p.teardown(container).await,
            AnyProvider::Kubernetes(p) => p.teardown(container).await,
        }
    }
}

/// Execute the `run` subcommand.
pub async fn execute(args: &RunArgs) -> Result<i32> {
    if args.dry_run {
        dry_run::execute_dry_run(args)
    } else {
        phases::run_with_cancellation(args, tokio::signal::ctrl_c()).await
    }
}
