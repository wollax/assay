//! Runtime provider trait — the extension point for container execution backends.
//!
//! The [`RuntimeProvider`] trait defines the lifecycle of a container-based
//! job execution: provision a container, execute commands, collect results,
//! and tear down. Implementations live in downstream crates (e.g., `DockerProvider`
//! in S02).

use crate::manifest::JobManifest;

/// Opaque handle to a provisioned container.
///
/// Wraps a provider-specific identifier (e.g., a Docker container ID).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContainerId(pub String);

impl ContainerId {
    /// Create a new container ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The raw identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Truncate to first 12 chars for display (like Docker short IDs).
        let short = if self.0.len() > 12 {
            &self.0[..12]
        } else {
            &self.0
        };
        write!(f, "{short}")
    }
}

/// Result of provisioning a container.
///
/// Contains the container ID and optional metadata discovered during
/// provisioning (e.g. the container's IP address for signal delivery).
#[derive(Debug, Clone)]
pub struct ProvisionResult {
    /// The provisioned container's identifier.
    pub container_id: ContainerId,
    /// Container's IP address on the Docker bridge network, if discovered.
    /// Used to cache signal endpoint URLs for HTTP-first delivery (D186).
    pub container_ip: Option<String>,
}

/// Result of executing a command inside a container.
///
/// Carries the container, execution identifier, exit code, and captured
/// stdout/stderr from the completed command.
#[derive(Debug, Clone)]
pub struct ExecHandle {
    /// The container this execution belongs to.
    pub container: ContainerId,
    /// Provider-specific execution identifier.
    pub exec_id: String,
    /// Exit code of the executed command (0 = success).
    pub exit_code: i32,
    /// Captured stdout from the command.
    pub stdout: String,
    /// Captured stderr from the command.
    pub stderr: String,
}

/// Result of collecting artifacts and outputs from a completed session.
#[derive(Debug)]
pub struct CollectResult {
    /// Exit code of the harness command (0 = success).
    pub exit_code: i32,
    /// Captured stdout from the harness.
    pub stdout: String,
    /// Captured stderr from the harness.
    pub stderr: String,
    /// Paths to collected artifact files (relative to workspace).
    pub artifacts: Vec<String>,
}

/// The runtime provider trait — lifecycle contract for container execution.
///
/// Implementors must be `Send + Sync` to allow concurrent session execution.
/// All methods are async and return [`crate::Result`].
///
/// # Lifecycle
///
/// ```text
/// provision() → exec() → collect() → teardown()
/// ```
///
/// `teardown()` must be called even if earlier steps fail, to avoid leaking
/// containers.
pub trait RuntimeProvider: Send + Sync {
    /// Provision a container for the given job manifest.
    ///
    /// Sets up the container image, mounts, resource limits, and environment
    /// variables as specified in the manifest's `[environment]` section.
    fn provision(
        &self,
        manifest: &JobManifest,
    ) -> impl std::future::Future<Output = crate::Result<ProvisionResult>> + Send;

    /// Execute a command inside the container.
    ///
    /// Returns an [`ExecHandle`] that identifies the running execution.
    fn exec(
        &self,
        container: &ContainerId,
        command: &[String],
    ) -> impl std::future::Future<Output = crate::Result<ExecHandle>> + Send;

    /// Execute a command inside the container, streaming output via a callback.
    ///
    /// Calls `output_cb` for each stdout/stderr chunk as it arrives.  The
    /// returned [`ExecHandle`] still carries the full buffered output after
    /// the stream completes.
    fn exec_streaming<F>(
        &self,
        container: &ContainerId,
        command: &[String],
        output_cb: F,
    ) -> impl std::future::Future<Output = crate::Result<ExecHandle>> + Send
    where
        F: FnMut(&str) + Send + 'static;

    /// Collect results and artifacts after a session completes.
    fn collect(
        &self,
        container: &ContainerId,
        manifest: &JobManifest,
    ) -> impl std::future::Future<Output = crate::Result<CollectResult>> + Send;

    /// Tear down the container and release resources.
    ///
    /// Must be called even if `exec()` or `collect()` failed.
    fn teardown(
        &self,
        container: &ContainerId,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;
}
