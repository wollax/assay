//! Docker runtime provider using the bollard crate.
//!
//! Implements [`RuntimeProvider`] against a local Docker daemon via Unix socket.

use std::collections::HashMap;

use bollard::Docker;
use bollard::container::LogOutput;
use bollard::exec::CreateExecOptions;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateImageOptionsBuilder, RemoveContainerOptionsBuilder, StopContainerOptionsBuilder,
};
use futures_util::{StreamExt, TryStreamExt};
use tracing::{debug, info, warn};

use crate::error::SmeltError;
use crate::manifest::{JobManifest, resolve_repo_path};
use crate::provider::{CollectResult, ContainerId, ExecHandle, RuntimeProvider};

/// Docker-backed runtime provider.
///
/// Holds a bollard [`Docker`] client connected to the local daemon.
pub struct DockerProvider {
    client: Docker,
}

impl DockerProvider {
    /// Connect to the local Docker daemon using default socket settings.
    ///
    /// Returns [`SmeltError::Provider`] if the connection fails (e.g., socket
    /// not found, permission denied).
    pub fn new() -> crate::Result<Self> {
        let client = Docker::connect_with_socket_defaults().map_err(|e| {
            SmeltError::provider_with_source(
                "connect",
                format!("failed to connect to Docker daemon: {e}"),
                e,
            )
        })?;
        Ok(Self { client })
    }

    /// Access the underlying bollard client (for tests or advanced usage).
    pub fn client(&self) -> &Docker {
        &self.client
    }
}

/// Detect the host address reachable from inside Docker containers.
///
/// Resolution order:
/// 1. `host_override` parameter (caller reads `SMELT_EVENT_HOST` env var).
/// 2. On macOS: `"host.docker.internal"` (Docker Desktop provides this).
/// 3. On Linux: inspect the Docker `bridge` network and extract the gateway IP.
///    Falls back to `"172.17.0.1"` with a warning if inspection fails or the
///    IPAM config is missing.
///
/// This function currently never returns `Err` — all fallback paths produce `Ok`.
pub async fn detect_host_address(docker: &Docker) -> crate::Result<String> {
    // Read the env override and delegate to the testable inner function.
    let host_override = std::env::var("SMELT_EVENT_HOST")
        .ok()
        .filter(|s| !s.is_empty());
    detect_host_address_with_override(docker, host_override.as_deref()).await
}

/// Inner implementation that accepts the override as a parameter for testability.
pub(crate) async fn detect_host_address_with_override(
    docker: &Docker,
    host_override: Option<&str>,
) -> crate::Result<String> {
    // 1. Explicit override — always respected.
    if let Some(host) = host_override {
        info!(host = %host, "using host address override");
        return Ok(host.to_string());
    }

    // 2. macOS — Docker Desktop provides host.docker.internal
    if cfg!(target_os = "macos") {
        return Ok("host.docker.internal".to_string());
    }

    // 3. Linux — inspect bridge network for gateway IP
    match docker.inspect_network("bridge", None).await {
        Ok(network) => {
            if let Some(ipam) = network.ipam
                && let Some(configs) = ipam.config
                && let Some(first) = configs.first()
                && let Some(gateway) = &first.gateway
                && !gateway.is_empty()
            {
                info!(gateway = %gateway, "detected Docker bridge gateway");
                return Ok(gateway.clone());
            }
            warn!(
                "bridge network IPAM config has no gateway; falling back to 172.17.0.1. \
                 Set SMELT_EVENT_HOST to override."
            );
            Ok("172.17.0.1".to_string())
        }
        Err(e) => {
            warn!(
                error = %e,
                "failed to inspect Docker bridge network; falling back to 172.17.0.1. \
                 Set SMELT_EVENT_HOST to override."
            );
            Ok("172.17.0.1".to_string())
        }
    }
}

impl RuntimeProvider for DockerProvider {
    async fn provision(&self, manifest: &JobManifest) -> crate::Result<ContainerId> {
        let image = &manifest.environment.image;

        // Pull image if not present locally
        if self.client.inspect_image(image).await.is_err() {
            info!(image = %image, "pulling image");
            let options = CreateImageOptionsBuilder::default()
                .from_image(image)
                .build();
            self.client
                .create_image(Some(options), None, None)
                .try_collect::<Vec<_>>()
                .await
                .map_err(|e| {
                    SmeltError::provider_with_source(
                        "provision",
                        format!("failed to pull image {image}: {e}"),
                        e,
                    )
                })?;
            info!(image = %image, "image pull complete");
        }

        // Build env vars from credentials
        let mut env: Vec<String> = manifest
            .credentials
            .env
            .iter()
            .filter_map(|(key, env_var)| {
                std::env::var(env_var)
                    .ok()
                    .map(|val| format!("{key}={val}"))
            })
            .collect();

        // Merge runtime_env (computed values like SMELT_EVENT_URL)
        for (key, val) in &manifest.runtime_env {
            env.push(format!("{key}={val}"));
        }

        // Parse resource limits
        let memory = manifest
            .environment
            .resources
            .get("memory")
            .map(|s| parse_memory_bytes(s))
            .transpose()?;
        let nano_cpus = manifest
            .environment
            .resources
            .get("cpu")
            .map(|s| parse_cpu_nanocpus(s))
            .transpose()?;

        // Labels
        let mut labels = HashMap::new();
        labels.insert("smelt.job".to_string(), manifest.job.name.clone());

        // Resolve and bind-mount the repo path
        let repo_path = resolve_repo_path(&manifest.job.repo)?;
        let bind_string = format!("{}:/workspace", repo_path.display());
        info!(repo_path = %repo_path.display(), bind = %bind_string, "resolved repo bind-mount");

        let host_config = HostConfig {
            memory,
            nano_cpus,
            binds: Some(vec![bind_string]),
            ..Default::default()
        };

        let config = ContainerCreateBody {
            image: Some(image.clone()),
            env: if env.is_empty() { None } else { Some(env) },
            cmd: Some(vec!["sleep".to_string(), "3600".to_string()]),
            labels: Some(labels),
            host_config: Some(host_config),
            ..Default::default()
        };

        let response = self
            .client
            .create_container(
                None::<bollard::query_parameters::CreateContainerOptions>,
                config,
            )
            .await
            .map_err(|e| {
                SmeltError::provider_with_source(
                    "provision",
                    format!("failed to create container: {e}"),
                    e,
                )
            })?;

        let container_id = response.id;
        info!(container_id = %container_id, "container created");

        self.client
            .start_container(
                &container_id,
                None::<bollard::query_parameters::StartContainerOptions>,
            )
            .await
            .map_err(|e| {
                SmeltError::provider_with_source(
                    "provision",
                    format!("failed to start container: {e}"),
                    e,
                )
            })?;

        info!(container_id = %container_id, "container started");

        Ok(ContainerId::new(container_id))
    }

    async fn exec(&self, container: &ContainerId, command: &[String]) -> crate::Result<ExecHandle> {
        let container_id = container.as_str();

        // Create exec instance with working_dir set to /workspace
        let exec_config = CreateExecOptions {
            cmd: Some(command.to_vec()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            working_dir: Some("/workspace".to_string()),
            ..Default::default()
        };

        let exec_created = self
            .client
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| {
                SmeltError::provider_with_source(
                    "exec",
                    format!("failed to create exec in container {container}: {e}"),
                    e,
                )
            })?;
        let exec_id = exec_created.id;
        info!(exec_id = %exec_id, container_id = %container_id, "exec created");

        // Start exec (attached mode)
        let start_result = self.client.start_exec(&exec_id, None).await.map_err(|e| {
            SmeltError::provider_with_source(
                "exec",
                format!("failed to start exec {exec_id}: {e}"),
                e,
            )
        })?;

        info!(exec_id = %exec_id, "exec started");

        // Consume the output stream
        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();

        match start_result {
            bollard::exec::StartExecResults::Attached { mut output, .. } => {
                while let Some(chunk) = output.next().await {
                    match chunk {
                        Ok(LogOutput::StdOut { message }) => {
                            let text = String::from_utf8_lossy(&message);
                            debug!(stream = "stdout", "{}", text.trim_end());
                            stdout_buf.push_str(&text);
                        }
                        Ok(LogOutput::StdErr { message }) => {
                            let text = String::from_utf8_lossy(&message);
                            debug!(stream = "stderr", "{}", text.trim_end());
                            stderr_buf.push_str(&text);
                        }
                        Ok(_) => {} // StdIn, Console — ignore
                        Err(e) => {
                            return Err(SmeltError::provider_with_source(
                                "exec",
                                format!("stream error during exec {exec_id}: {e}"),
                                e,
                            ));
                        }
                    }
                }
            }
            bollard::exec::StartExecResults::Detached => {
                return Err(SmeltError::provider(
                    "exec",
                    format!("exec {exec_id} unexpectedly started in detached mode"),
                ));
            }
        }

        // Retrieve exit code via inspect_exec
        let inspect = self.client.inspect_exec(&exec_id).await.map_err(|e| {
            SmeltError::provider_with_source(
                "exec",
                format!("failed to inspect exec {exec_id}: {e}"),
                e,
            )
        })?;

        let exit_code = inspect.exit_code.unwrap_or(-1) as i32;

        info!(exec_id = %exec_id, exit_code = exit_code, "exec complete");

        Ok(ExecHandle {
            container: container.clone(),
            exec_id,
            exit_code,
            stdout: stdout_buf,
            stderr: stderr_buf,
        })
    }

    async fn exec_streaming<F>(
        &self,
        container: &ContainerId,
        command: &[String],
        mut output_cb: F,
    ) -> crate::Result<ExecHandle>
    where
        F: FnMut(&str) + Send + 'static,
    {
        let container_id = container.as_str();

        // Create exec instance with working_dir set to /workspace
        let exec_config = CreateExecOptions {
            cmd: Some(command.to_vec()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            working_dir: Some("/workspace".to_string()),
            ..Default::default()
        };

        let exec_created = self
            .client
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| {
                SmeltError::provider_with_source(
                    "exec_streaming",
                    format!("failed to create exec in container {container}: {e}"),
                    e,
                )
            })?;
        let exec_id = exec_created.id;
        info!(exec_id = %exec_id, container_id = %container_id, "exec_streaming created");

        // Start exec (attached mode)
        let start_result = self.client.start_exec(&exec_id, None).await.map_err(|e| {
            SmeltError::provider_with_source(
                "exec_streaming",
                format!("failed to start exec {exec_id}: {e}"),
                e,
            )
        })?;

        info!(exec_id = %exec_id, "exec_streaming started");

        // Consume the output stream, calling output_cb per chunk
        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();

        match start_result {
            bollard::exec::StartExecResults::Attached { mut output, .. } => {
                while let Some(chunk) = output.next().await {
                    match chunk {
                        Ok(LogOutput::StdOut { message }) => {
                            let text = String::from_utf8_lossy(&message);
                            debug!(stream = "stdout", "{}", text.trim_end());
                            output_cb(&text);
                            stdout_buf.push_str(&text);
                        }
                        Ok(LogOutput::StdErr { message }) => {
                            let text = String::from_utf8_lossy(&message);
                            debug!(stream = "stderr", "{}", text.trim_end());
                            output_cb(&text);
                            stderr_buf.push_str(&text);
                        }
                        Ok(_) => {} // StdIn, Console — ignore
                        Err(e) => {
                            return Err(SmeltError::provider_with_source(
                                "exec_streaming",
                                format!("stream error during exec {exec_id}: {e}"),
                                e,
                            ));
                        }
                    }
                }
            }
            bollard::exec::StartExecResults::Detached => {
                return Err(SmeltError::provider(
                    "exec_streaming",
                    format!("exec {exec_id} unexpectedly started in detached mode"),
                ));
            }
        }

        // Retrieve exit code via inspect_exec
        let inspect = self.client.inspect_exec(&exec_id).await.map_err(|e| {
            SmeltError::provider_with_source(
                "exec_streaming",
                format!("failed to inspect exec {exec_id}: {e}"),
                e,
            )
        })?;

        let exit_code = inspect.exit_code.unwrap_or(-1) as i32;

        info!(exec_id = %exec_id, exit_code = exit_code, "exec_streaming complete");

        Ok(ExecHandle {
            container: container.clone(),
            exec_id,
            exit_code,
            stdout: stdout_buf,
            stderr: stderr_buf,
        })
    }

    async fn collect(
        &self,
        _container: &ContainerId,
        _manifest: &JobManifest,
    ) -> crate::Result<CollectResult> {
        Ok(CollectResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            artifacts: vec![],
        })
    }

    async fn teardown(&self, container: &ContainerId) -> crate::Result<()> {
        let id = container.as_str();

        // Stop — ignore "not running" errors
        let stop_opts = StopContainerOptionsBuilder::default().t(10).build();
        if let Err(e) = self.client.stop_container(id, Some(stop_opts)).await {
            // 304 = container already stopped, or NotFound — both are fine
            match &e {
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 304, ..
                }
                | bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                } => {
                    // Container already stopped or gone — continue to remove
                }
                _ => {
                    return Err(SmeltError::provider_with_source(
                        "teardown",
                        format!("failed to stop container {container}: {e}"),
                        e,
                    ));
                }
            }
        }

        // Force remove with anonymous volumes — tolerate 404 (already gone)
        let remove_opts = RemoveContainerOptionsBuilder::default()
            .force(true)
            .v(true)
            .build();
        if let Err(e) = self.client.remove_container(id, Some(remove_opts)).await {
            match &e {
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 404, ..
                } => {
                    // Container already removed — idempotent teardown
                }
                _ => {
                    return Err(SmeltError::provider_with_source(
                        "teardown",
                        format!("failed to remove container {container}: {e}"),
                        e,
                    ));
                }
            }
        }

        info!(container_id = %id, "container removed");

        Ok(())
    }
}

// ── Resource parsing utilities ──────────────────────────────────────────

/// Parse a human-readable memory string into bytes.
///
/// Supported suffixes (case-insensitive): `K` (KiB), `M` (MiB), `G` (GiB).
/// Plain integers are treated as bytes.
///
/// # Examples
///
/// ```
/// # use smelt_core::docker::parse_memory_bytes;
/// assert_eq!(parse_memory_bytes("4G").unwrap(), 4 * 1024 * 1024 * 1024);
/// assert_eq!(parse_memory_bytes("512M").unwrap(), 512 * 1024 * 1024);
/// assert_eq!(parse_memory_bytes("1024K").unwrap(), 1024 * 1024);
/// assert_eq!(parse_memory_bytes("65536").unwrap(), 65536);
/// ```
pub fn parse_memory_bytes(s: &str) -> crate::Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(SmeltError::provider("parse_memory", "empty memory string"));
    }

    let (num_str, multiplier) = match s.as_bytes().last() {
        Some(b'G' | b'g') => (&s[..s.len() - 1], 1024_i64 * 1024 * 1024),
        Some(b'M' | b'm') => (&s[..s.len() - 1], 1024_i64 * 1024),
        Some(b'K' | b'k') => (&s[..s.len() - 1], 1024_i64),
        _ => (s, 1_i64),
    };

    let value: f64 = num_str.trim().parse().map_err(|_| {
        SmeltError::provider("parse_memory", format!("invalid memory value: {s:?}"))
    })?;

    if value < 0.0 {
        return Err(SmeltError::provider(
            "parse_memory",
            format!("negative memory value: {s:?}"),
        ));
    }

    Ok((value * multiplier as f64) as i64)
}

/// Parse a CPU count string into Docker nanocpus (1 CPU = 1_000_000_000).
///
/// Accepts integer or decimal values (e.g., `"2"`, `"0.5"`, `"1.5"`).
///
/// # Examples
///
/// ```
/// # use smelt_core::docker::parse_cpu_nanocpus;
/// assert_eq!(parse_cpu_nanocpus("2").unwrap(), 2_000_000_000);
/// assert_eq!(parse_cpu_nanocpus("0.5").unwrap(), 500_000_000);
/// ```
pub fn parse_cpu_nanocpus(s: &str) -> crate::Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(SmeltError::provider("parse_cpu", "empty CPU string"));
    }

    let value: f64 = s
        .parse()
        .map_err(|_| SmeltError::provider("parse_cpu", format!("invalid CPU value: {s:?}")))?;

    if value <= 0.0 {
        return Err(SmeltError::provider(
            "parse_cpu",
            format!("CPU value must be positive: {s:?}"),
        ));
    }

    Ok((value * 1_000_000_000.0) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_memory_bytes ──────────────────────────────────────

    #[test]
    fn memory_gigabytes() {
        assert_eq!(parse_memory_bytes("4G").unwrap(), 4 * 1024 * 1024 * 1024);
        assert_eq!(parse_memory_bytes("1g").unwrap(), 1024 * 1024 * 1024);
    }

    #[test]
    fn memory_megabytes() {
        assert_eq!(parse_memory_bytes("512M").unwrap(), 512 * 1024 * 1024);
        assert_eq!(parse_memory_bytes("256m").unwrap(), 256 * 1024 * 1024);
    }

    #[test]
    fn memory_kilobytes() {
        assert_eq!(parse_memory_bytes("1024K").unwrap(), 1024 * 1024);
        assert_eq!(parse_memory_bytes("512k").unwrap(), 512 * 1024);
    }

    #[test]
    fn memory_plain_bytes() {
        assert_eq!(parse_memory_bytes("65536").unwrap(), 65536);
    }

    #[test]
    fn memory_fractional() {
        assert_eq!(
            parse_memory_bytes("1.5G").unwrap(),
            (1.5 * 1024.0 * 1024.0 * 1024.0) as i64
        );
    }

    #[test]
    fn memory_with_whitespace() {
        assert_eq!(
            parse_memory_bytes("  4G  ").unwrap(),
            4 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn memory_empty_string() {
        assert!(parse_memory_bytes("").is_err());
    }

    #[test]
    fn memory_invalid_string() {
        assert!(parse_memory_bytes("abc").is_err());
        assert!(parse_memory_bytes("G").is_err());
    }

    #[test]
    fn memory_negative() {
        assert!(parse_memory_bytes("-1G").is_err());
    }

    // ── parse_cpu_nanocpus ──────────────────────────────────────

    #[test]
    fn cpu_integer() {
        assert_eq!(parse_cpu_nanocpus("2").unwrap(), 2_000_000_000);
        assert_eq!(parse_cpu_nanocpus("1").unwrap(), 1_000_000_000);
    }

    #[test]
    fn cpu_fractional() {
        assert_eq!(parse_cpu_nanocpus("0.5").unwrap(), 500_000_000);
        assert_eq!(parse_cpu_nanocpus("1.5").unwrap(), 1_500_000_000);
        assert_eq!(parse_cpu_nanocpus("0.25").unwrap(), 250_000_000);
    }

    #[test]
    fn cpu_with_whitespace() {
        assert_eq!(parse_cpu_nanocpus("  2  ").unwrap(), 2_000_000_000);
    }

    #[test]
    fn cpu_empty_string() {
        assert!(parse_cpu_nanocpus("").is_err());
    }

    #[test]
    fn cpu_invalid_string() {
        assert!(parse_cpu_nanocpus("abc").is_err());
    }

    #[test]
    fn cpu_zero() {
        assert!(parse_cpu_nanocpus("0").is_err());
    }

    #[test]
    fn cpu_negative() {
        assert!(parse_cpu_nanocpus("-1").is_err());
    }

    // ── detect_host_address ─────────────────────────────────────

    /// Test override takes priority via the testable inner function.
    #[tokio::test]
    async fn detect_host_address_override() {
        let docker = Docker::connect_with_socket_defaults()
            .unwrap_or_else(|_| panic!("Docker client needed for test structure"));

        let result =
            super::detect_host_address_with_override(&docker, Some("custom-host.example.com"))
                .await
                .unwrap();
        assert_eq!(
            result, "custom-host.example.com",
            "explicit override must take priority"
        );
    }

    /// Without override, platform default is used.
    #[tokio::test]
    async fn detect_host_address_platform_default() {
        let docker = Docker::connect_with_socket_defaults()
            .unwrap_or_else(|_| panic!("Docker client needed for test structure"));

        let result = super::detect_host_address_with_override(&docker, None)
            .await
            .unwrap();

        #[cfg(target_os = "macos")]
        assert_eq!(
            result, "host.docker.internal",
            "macOS must return host.docker.internal"
        );

        #[cfg(target_os = "linux")]
        {
            // On Linux, should be a valid IP address (gateway or fallback)
            assert!(
                !result.is_empty(),
                "Linux must return a non-empty host address"
            );
        }
    }
}
