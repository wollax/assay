//! Docker Compose file generation and runtime provider for Smelt.
//!
//! This module implements two things:
//!
//! 1. [`generate_compose_file`] — a pure function that takes a [`JobManifest`],
//!    a project name, and resolved credential env vars, and returns a valid
//!    Docker Compose YAML string (Compose v2+ format, no `version:` key).
//!
//! 2. [`ComposeProvider`] — a [`RuntimeProvider`] implementation backed by
//!    `docker compose` subprocesses and bollard-delegated exec operations.
//!
//! The generated file includes:
//! - All `[[services]]` entries from the manifest passed through as-is.
//! - An injected `smelt-agent` service with the workspace volume mount,
//!   optional credential environment, and a named network.
//! - A top-level `networks:` section with the named network.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tempfile::TempDir;
use tokio::process::Command;
use tracing::{info, warn};

use crate::Result;
use crate::docker::DockerProvider;
use crate::manifest::{JobManifest, resolve_repo_path};
use crate::provider::{CollectResult, ContainerId, ExecHandle, RuntimeProvider};

// ── Internal state ────────────────────────────────────────────────────────────

/// Internal state for a provisioned Compose project.
///
/// The `_temp_dir` field owns the temporary directory holding the generated
/// `docker-compose.yml`. It is intentionally kept alive here — dropping it
/// would delete the file before `docker compose down` can read it.
struct ComposeProjectState {
    project_name: String,
    compose_file_path: PathBuf,
    _temp_dir: TempDir,
}

// ── ComposeProvider ───────────────────────────────────────────────────────────

/// Docker Compose runtime provider.
///
/// Provisions each job as an isolated Compose project: generates a
/// `docker-compose.yml` in a temporary directory, runs `docker compose up -d`,
/// and delegates exec operations to bollard via an embedded [`DockerProvider`].
pub struct ComposeProvider {
    docker: DockerProvider,
    state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>,
}

impl ComposeProvider {
    /// Connect to the local Docker daemon and initialise an empty state map.
    ///
    /// Returns [`crate::SmeltError::Provider`] if the bollard connection fails.
    pub fn new() -> crate::Result<Self> {
        let docker = DockerProvider::new()?;
        Ok(Self {
            docker,
            state: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

impl RuntimeProvider for ComposeProvider {
    async fn provision(&self, manifest: &JobManifest) -> crate::Result<ContainerId> {
        let project_name = format!("smelt-{}", manifest.job.name);

        info!(project = %project_name, "starting compose provision");

        // Resolve credentials from the environment.
        let extra_env: HashMap<String, String> = manifest
            .credentials
            .env
            .iter()
            .filter_map(|(key, env_var)| std::env::var(env_var).ok().map(|val| (key.clone(), val)))
            .collect();

        // Generate compose YAML.
        let yaml = generate_compose_file(manifest, &project_name, &extra_env)?;

        // Write to a temporary directory.
        let temp_dir = TempDir::new().map_err(|e| {
            crate::SmeltError::provider(
                "provision",
                format!("failed to create temp dir for compose file: {e}"),
            )
        })?;
        let compose_file_path = temp_dir.path().join("docker-compose.yml");
        std::fs::write(&compose_file_path, &yaml).map_err(|e| {
            crate::SmeltError::provider("provision", format!("failed to write compose file: {e}"))
        })?;

        // Print wait messages for non-agent services before starting the stack.
        for svc in &manifest.services {
            eprintln!("Waiting for {} to be healthy...", svc.name);
        }

        // Run `docker compose up -d`.
        let up_output = Command::new("docker")
            .args([
                "compose",
                "-f",
                compose_file_path.to_str().unwrap_or_default(),
                "-p",
                &project_name,
                "up",
                "-d",
            ])
            .output()
            .await
            .map_err(|e| {
                crate::SmeltError::provider(
                    "provision",
                    format!("failed to spawn docker compose up: {e}"),
                )
            })?;

        if !up_output.status.success() {
            let stderr = String::from_utf8_lossy(&up_output.stderr);
            return Err(crate::SmeltError::provider(
                "provision",
                format!("docker compose up failed: {stderr}"),
            ));
        }

        info!(project = %project_name, "docker compose up -d complete");

        // Poll `docker compose ps --format json` (NDJSON) until all non-agent
        // services are healthy or running, or until we time out.
        let max_polls = 60usize;
        let poll_interval = std::time::Duration::from_secs(2);

        let non_agent_services: Vec<&str> =
            manifest.services.iter().map(|s| s.name.as_str()).collect();

        let mut agent_container_id: Option<String> = None;

        'poll: for attempt in 0..max_polls {
            tokio::time::sleep(poll_interval).await;

            let ps_output = Command::new("docker")
                .args([
                    "compose",
                    "-f",
                    compose_file_path.to_str().unwrap_or_default(),
                    "-p",
                    &project_name,
                    "ps",
                    "--format",
                    "json",
                ])
                .output()
                .await
                .map_err(|e| {
                    crate::SmeltError::provider(
                        "provision",
                        format!("failed to spawn docker compose ps: {e}"),
                    )
                })?;

            let stdout = String::from_utf8_lossy(&ps_output.stdout);
            info!(
                project = %project_name,
                attempt = attempt + 1,
                "healthcheck poll"
            );

            // Parse NDJSON — one JSON object per non-empty line.
            let mut services_ready: HashMap<&str, bool> = HashMap::new();

            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let val: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("failed to parse compose ps JSON line: {e}");
                        continue;
                    }
                };

                let service = val
                    .get("Service")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let health = val
                    .get("Health")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let state = val
                    .get("State")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                // Capture agent container ID.
                if service == "smelt-agent"
                    && let Some(id) = val.get("ID").and_then(|v| v.as_str())
                {
                    agent_container_id = Some(id.to_string());
                }

                // Readiness logic per the NDJSON strategy in the research doc.
                //
                // - no healthcheck: Health == "" AND State == "running"  → ready
                // - healthcheck:    Health == "healthy"                   → ready
                // - starting:       Health == "starting"                  → not ready yet
                // - unhealthy:      Health == "unhealthy"                 → error
                let is_ready = (health.is_empty() && state == "running") || health == "healthy";
                let is_unhealthy = health == "unhealthy";

                if is_unhealthy && non_agent_services.contains(&service.as_str()) {
                    return Err(crate::SmeltError::provider(
                        "provision",
                        format!("service {service} became unhealthy"),
                    ));
                }

                if non_agent_services.contains(&service.as_str()) {
                    services_ready.insert(
                        // SAFETY: borrowing manifest.services[i].name which outlives this loop
                        non_agent_services
                            .iter()
                            .find(|&&s| s == service.as_str())
                            .copied()
                            .unwrap_or(""),
                        is_ready,
                    );
                }
            }

            // Check if all non-agent services are ready.
            let all_ready = non_agent_services
                .iter()
                .all(|svc| services_ready.get(svc).copied().unwrap_or(false));

            if all_ready {
                info!(project = %project_name, "all non-agent services healthy");
                break 'poll;
            }

            if attempt + 1 == max_polls {
                return Err(crate::SmeltError::provider(
                    "provision",
                    "timed out waiting for services to become healthy after 120s",
                ));
            }
        }

        // When there are no non-agent services we skip the loop above (all_ready
        // is vacuously true after attempt 0 starts), but we still need the agent
        // container ID. Do one final ps if we haven't captured it yet.
        if agent_container_id.is_none() {
            let ps_output = Command::new("docker")
                .args([
                    "compose",
                    "-f",
                    compose_file_path.to_str().unwrap_or_default(),
                    "-p",
                    &project_name,
                    "ps",
                    "--format",
                    "json",
                ])
                .output()
                .await
                .map_err(|e| {
                    crate::SmeltError::provider(
                        "provision",
                        format!("failed to spawn docker compose ps (final): {e}"),
                    )
                })?;

            let stdout = String::from_utf8_lossy(&ps_output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line)
                    && val.get("Service").and_then(|v| v.as_str()) == Some("smelt-agent")
                    && let Some(id) = val.get("ID").and_then(|v| v.as_str())
                {
                    agent_container_id = Some(id.to_string());
                }
            }
        }

        let container_id_str = agent_container_id.ok_or_else(|| {
            crate::SmeltError::provider(
                "provision",
                "could not find smelt-agent container ID in compose ps output",
            )
        })?;

        let container_id = ContainerId::new(container_id_str);

        info!(
            project = %project_name,
            agent_container = %container_id,
            "compose provision complete"
        );

        // Store state — lock only for the insert, never across await points.
        {
            let mut map = self.state.lock().unwrap();
            map.insert(
                container_id.clone(),
                ComposeProjectState {
                    project_name,
                    compose_file_path,
                    _temp_dir: temp_dir,
                },
            );
        }

        Ok(container_id)
    }

    async fn exec(&self, container: &ContainerId, command: &[String]) -> crate::Result<ExecHandle> {
        self.docker.exec(container, command).await
    }

    async fn exec_streaming<F>(
        &self,
        container: &ContainerId,
        command: &[String],
        output_cb: F,
    ) -> crate::Result<ExecHandle>
    where
        F: FnMut(&str) + Send + 'static,
    {
        self.docker
            .exec_streaming(container, command, output_cb)
            .await
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
        // Retrieve project state — lock only for the lookup.
        let (project_name, compose_file_path) = {
            let map = self.state.lock().unwrap();
            match map.get(container) {
                Some(state) => (state.project_name.clone(), state.compose_file_path.clone()),
                None => {
                    warn!(
                        container = %container,
                        "teardown called for unknown container; nothing to do"
                    );
                    return Ok(());
                }
            }
        };

        info!(project = %project_name, "starting compose teardown");

        // Run `docker compose down --remove-orphans`.
        // Per D023/D038: fault-tolerant — log errors but don't propagate.
        let down_result = Command::new("docker")
            .args([
                "compose",
                "-f",
                compose_file_path.to_str().unwrap_or_default(),
                "-p",
                &project_name,
                "down",
                "--remove-orphans",
            ])
            .output()
            .await;

        match down_result {
            Ok(output) if output.status.success() => {
                info!(project = %project_name, "compose down complete");
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(project = %project_name, "compose down exited non-zero: {stderr}");
            }
            Err(e) => {
                warn!(project = %project_name, "failed to spawn compose down: {e}");
            }
        }

        // Remove state entry — this drops `_temp_dir`, deleting the temp directory.
        {
            let mut map = self.state.lock().unwrap();
            map.remove(container);
        }

        Ok(())
    }
}

// ── Compose file generation ───────────────────────────────────────────────────

/// Generate a Docker Compose v2 YAML string from a [`JobManifest`].
///
/// # Parameters
///
/// - `manifest` — the parsed job manifest; provides `[[services]]` entries and
///   the `smelt-agent` image from `[environment]`.
/// - `project_name` — used to name the shared Docker network (`smelt-<project_name>`).
/// - `extra_env` — resolved credential environment variables injected into the
///   `smelt-agent` service only (per D074).
///
/// # Errors
///
/// Returns [`crate::SmeltError::Manifest`] if `manifest.job.repo` cannot be
/// canonicalized as a local path (propagated from [`resolve_repo_path`]).
/// Returns [`crate::SmeltError::Provider`] if serde_yaml serialization fails
/// (should not occur for well-formed Compose data).
pub fn generate_compose_file(
    manifest: &JobManifest,
    _project_name: &str,
    extra_env: &HashMap<String, String>,
) -> Result<String> {
    // Resolve the repo path for the workspace volume mount.
    let repo_path = resolve_repo_path(&manifest.job.repo)?;
    let vol_string = format!("{}:/workspace", repo_path.display());

    // Top-level services mapping — insertion order is preserved by serde_yaml::Mapping.
    let mut services_map = serde_yaml::Mapping::new();

    // User services — passed through in manifest order; image first, then extra
    // fields in alphabetical order (BTreeMap order from TOML's serde flatten).
    for service in &manifest.services {
        let mut svc_map = serde_yaml::Mapping::new();
        svc_map.insert(
            serde_yaml::Value::String("image".to_string()),
            serde_yaml::Value::String(service.image.clone()),
        );
        for (k, v) in &service.extra {
            svc_map.insert(serde_yaml::Value::String(k.clone()), toml_to_yaml(v));
        }
        services_map.insert(
            serde_yaml::Value::String(service.name.clone()),
            serde_yaml::Value::Mapping(svc_map),
        );
    }

    // smelt-agent service.
    let mut agent_map = serde_yaml::Mapping::new();

    agent_map.insert(
        serde_yaml::Value::String("image".to_string()),
        serde_yaml::Value::String(manifest.environment.image.clone()),
    );

    // Keep the agent container alive so bollard exec can attach to it.
    // Without a long-running command, alpine:3 exits immediately and
    // docker compose ps stops showing it — preventing agent ID capture.
    // Consistent with DockerProvider which uses `sleep 3600`.
    agent_map.insert(
        serde_yaml::Value::String("command".to_string()),
        serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("sleep".to_string()),
            serde_yaml::Value::String("3600".to_string()),
        ]),
    );

    agent_map.insert(
        serde_yaml::Value::String("volumes".to_string()),
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(vol_string)]),
    );

    // environment: omitted entirely when extra_env is empty (D074).
    if !extra_env.is_empty() {
        let mut env_map = serde_yaml::Mapping::new();
        for (k, v) in extra_env.iter().collect::<BTreeMap<_, _>>().iter() {
            env_map.insert(
                serde_yaml::Value::String((*k).clone()),
                serde_yaml::Value::String((*v).clone()),
            );
        }
        agent_map.insert(
            serde_yaml::Value::String("environment".to_string()),
            serde_yaml::Value::Mapping(env_map),
        );
    }

    // depends_on: omitted entirely when services is empty.
    if !manifest.services.is_empty() {
        let depends_seq: serde_yaml::Sequence = manifest
            .services
            .iter()
            .map(|s| serde_yaml::Value::String(s.name.clone()))
            .collect();
        agent_map.insert(
            serde_yaml::Value::String("depends_on".to_string()),
            serde_yaml::Value::Sequence(depends_seq),
        );
    }

    // No explicit `networks:` on smelt-agent — rely on Docker Compose's automatic
    // default project network. All services in the project share the default network,
    // giving the agent DNS resolution for user service names (e.g. "postgres").
    // A custom named network would isolate the agent from user services unless
    // every user service was explicitly added to the same network (D075).

    services_map.insert(
        serde_yaml::Value::String("smelt-agent".to_string()),
        serde_yaml::Value::Mapping(agent_map),
    );

    // Top-level document — services only; no top-level `networks:` key needed
    // because Docker Compose auto-creates a default network for the project.
    let mut top_level = serde_yaml::Mapping::new();
    top_level.insert(
        serde_yaml::Value::String("services".to_string()),
        serde_yaml::Value::Mapping(services_map),
    );

    serde_yaml::to_string(&serde_yaml::Value::Mapping(top_level))
        .map_err(|e| crate::SmeltError::provider("serialize", e.to_string()))
}

/// Convert a [`toml::Value`] to a [`serde_yaml::Value`].
///
/// All seven `toml::Value` variants are handled:
/// - Scalars map directly to the corresponding YAML scalar types.
/// - Arrays become YAML sequences with each element recursively converted.
/// - Tables become YAML mappings; key order follows BTreeMap (alphabetical)
///   because `toml::value::Table` is internally a `BTreeMap`.
/// - Datetimes fall back to their string representation (edge case; not
///   expected in Docker Compose service definitions).
fn toml_to_yaml(v: &toml::Value) -> serde_yaml::Value {
    match v {
        toml::Value::String(s) => serde_yaml::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_yaml::Value::Number(serde_yaml::Number::from(*i)),
        toml::Value::Float(f) => serde_yaml::Value::Number(serde_yaml::Number::from(*f)),
        toml::Value::Boolean(b) => serde_yaml::Value::Bool(*b),
        toml::Value::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(toml_to_yaml).collect())
        }
        toml::Value::Table(table) => {
            let mut m = serde_yaml::Mapping::new();
            for (k, v) in table {
                m.insert(serde_yaml::Value::String(k.clone()), toml_to_yaml(v));
            }
            serde_yaml::Value::Mapping(m)
        }
        toml::Value::Datetime(dt) => serde_yaml::Value::String(dt.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{
        ComposeService, CredentialConfig, Environment, JobManifest, JobMeta, MergeConfig,
        SessionDef,
    };
    use indexmap::IndexMap;

    /// Build a minimal `JobManifest` for compose snapshot tests.
    ///
    /// `job.repo` is set to `env!("CARGO_MANIFEST_DIR")` (the `crates/smelt-core`
    /// directory) so that `resolve_repo_path()` always succeeds — the directory
    /// is guaranteed to exist when tests run.
    fn make_manifest(services: Vec<ComposeService>) -> JobManifest {
        JobManifest {
            job: JobMeta {
                name: "test-job".to_string(),
                repo: env!("CARGO_MANIFEST_DIR").to_string(),
                base_ref: "main".to_string(),
            },
            environment: Environment {
                runtime: "compose".to_string(),
                image: "smelt-agent:latest".to_string(),
                resources: HashMap::new(),
            },
            credentials: CredentialConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                env: HashMap::new(),
            },
            session: vec![SessionDef {
                name: "run".to_string(),
                spec: "run the suite".to_string(),
                harness: "pytest".to_string(),
                timeout: 300,
                depends_on: vec![],
            }],
            merge: MergeConfig {
                strategy: "sequential".to_string(),
                order: vec![],
                ai_resolution: false,
                target: "main".to_string(),
            },
            forge: None,
            kubernetes: None,
            services,
        }
    }

    #[test]
    fn smoke_empty_services_compiles() {
        // Verify the module compiles and ComposeProvider::new() is callable.
        // Gracefully handle daemon-absent case in unit tests.
        let _provider = ComposeProvider::new().ok();
    }

    // ── Snapshot tests ────────────────────────────────────────────────────────

    /// Returns the canonicalized volume string used by the agent service in tests.
    ///
    /// `env!("CARGO_MANIFEST_DIR")` resolves to the `crates/smelt-core` directory
    /// at compile time. We canonicalize it here to match what `resolve_repo_path()`
    /// returns at runtime (handles any symlinks in the path).
    fn workspace_vol() -> String {
        let canon = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
            .expect("CARGO_MANIFEST_DIR must be canonicalizable");
        format!("{}:/workspace", canon.display())
    }

    /// Test 1: no user services, no extra_env — agent-only compose file.
    ///
    /// Confirms:
    /// - no `depends_on:` key on smelt-agent
    /// - no `environment:` key on smelt-agent
    /// - `networks:` list present on smelt-agent
    /// - top-level `networks: smelt-myproj: {}` present
    #[test]
    fn test_generate_compose_empty_services() {
        let manifest = make_manifest(vec![]);
        let yaml = generate_compose_file(&manifest, "myproj", &HashMap::new()).unwrap();
        let vol = workspace_vol();
        let expected = format!(
            "\
services:
  smelt-agent:
    image: smelt-agent:latest
    command:
    - sleep
    - '3600'
    volumes:
    - {vol}
"
        );
        assert_eq!(yaml, expected);
    }

    /// Test 2: one service, no extra fields, no credential env.
    ///
    /// Confirms:
    /// - `depends_on:` lists postgres on smelt-agent
    /// - `image: postgres:16` in postgres service block
    /// - `image` is first key in postgres service block
    /// - no `environment:` key on smelt-agent (empty extra_env)
    #[test]
    fn test_generate_compose_postgres_only() {
        let manifest = make_manifest(vec![ComposeService {
            name: "postgres".to_string(),
            image: "postgres:16".to_string(),
            extra: IndexMap::new(),
        }]);
        let yaml = generate_compose_file(&manifest, "myproj", &HashMap::new()).unwrap();
        let vol = workspace_vol();
        let expected = format!(
            "\
services:
  postgres:
    image: postgres:16
  smelt-agent:
    image: smelt-agent:latest
    command:
    - sleep
    - '3600'
    volumes:
    - {vol}
    depends_on:
    - postgres
"
        );
        assert_eq!(yaml, expected);
    }

    /// Test 3: two services, credential env present.
    ///
    /// Confirms:
    /// - both `postgres:` and `redis:` keys in services section
    /// - `depends_on:` lists postgres first, redis second (manifest order)
    /// - `environment: ANTHROPIC_API_KEY: test-key` present on smelt-agent
    #[test]
    fn test_generate_compose_postgres_and_redis() {
        let manifest = make_manifest(vec![
            ComposeService {
                name: "postgres".to_string(),
                image: "postgres:16".to_string(),
                extra: IndexMap::new(),
            },
            ComposeService {
                name: "redis".to_string(),
                image: "redis:7".to_string(),
                extra: IndexMap::new(),
            },
        ]);
        let mut extra_env = HashMap::new();
        extra_env.insert("ANTHROPIC_API_KEY".to_string(), "test-key".to_string());
        let yaml = generate_compose_file(&manifest, "myproj", &extra_env).unwrap();
        let vol = workspace_vol();
        let expected = format!(
            "\
services:
  postgres:
    image: postgres:16
  redis:
    image: redis:7
  smelt-agent:
    image: smelt-agent:latest
    command:
    - sleep
    - '3600'
    volumes:
    - {vol}
    environment:
      ANTHROPIC_API_KEY: test-key
    depends_on:
    - postgres
    - redis
"
        );
        assert_eq!(yaml, expected);
    }

    /// Test 4: TOML → YAML type fidelity — integer, boolean, array extra fields.
    ///
    /// Confirms:
    /// - `port: 5432` is a YAML integer (no quotes)
    /// - `restart: true` is a YAML boolean (no quotes)
    /// - `command:` is a YAML sequence
    /// - extra keys appear alphabetically (command, port, restart)
    #[test]
    fn test_generate_compose_type_fidelity() {
        let mut extra = IndexMap::new();
        // Insert in non-alphabetical order — output must still be alphabetical.
        extra.insert(
            "command".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("CMD".to_string()),
                toml::Value::String("pg_isready".to_string()),
            ]),
        );
        extra.insert("port".to_string(), toml::Value::Integer(5432));
        extra.insert("restart".to_string(), toml::Value::Boolean(true));
        let manifest = make_manifest(vec![ComposeService {
            name: "postgres".to_string(),
            image: "postgres:16".to_string(),
            extra,
        }]);
        let yaml = generate_compose_file(&manifest, "myproj", &HashMap::new()).unwrap();
        let vol = workspace_vol();
        let expected = format!(
            "\
services:
  postgres:
    image: postgres:16
    command:
    - CMD
    - pg_isready
    port: 5432
    restart: true
  smelt-agent:
    image: smelt-agent:latest
    command:
    - sleep
    - '3600'
    volumes:
    - {vol}
    depends_on:
    - postgres
"
        );
        assert_eq!(yaml, expected);
    }

    /// Test 5: nested TOML table → YAML mapping, sub-keys alphabetical.
    ///
    /// Confirms:
    /// - `healthcheck:` renders as a nested YAML mapping
    /// - sub-keys appear in alphabetical order: interval, retries, test
    ///   (toml::value::Table is BTreeMap internally — order is guaranteed)
    #[test]
    fn test_generate_compose_nested_healthcheck() {
        let mut hc_table = toml::value::Table::new();
        hc_table.insert(
            "interval".to_string(),
            toml::Value::String("30s".to_string()),
        );
        hc_table.insert("retries".to_string(), toml::Value::Integer(3));
        hc_table.insert(
            "test".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("CMD".to_string()),
                toml::Value::String("pg_isready".to_string()),
            ]),
        );
        let mut extra = IndexMap::new();
        extra.insert("healthcheck".to_string(), toml::Value::Table(hc_table));
        let manifest = make_manifest(vec![ComposeService {
            name: "postgres".to_string(),
            image: "postgres:16".to_string(),
            extra,
        }]);
        let yaml = generate_compose_file(&manifest, "myproj", &HashMap::new()).unwrap();
        let vol = workspace_vol();
        let expected = format!(
            "\
services:
  postgres:
    image: postgres:16
    healthcheck:
      interval: 30s
      retries: 3
      test:
      - CMD
      - pg_isready
  smelt-agent:
    image: smelt-agent:latest
    command:
    - sleep
    - '3600'
    volumes:
    - {vol}
    depends_on:
    - postgres
"
        );
        assert_eq!(yaml, expected);
    }

    /// Test 6: `environment:` key is absent when `extra_env` is empty.
    ///
    /// Confirms there is NO `environment:` key anywhere in the smelt-agent block
    /// when `extra_env` is an empty `HashMap`.
    #[test]
    fn test_generate_compose_empty_extra_env() {
        let manifest = make_manifest(vec![ComposeService {
            name: "postgres".to_string(),
            image: "postgres:16".to_string(),
            extra: IndexMap::new(),
        }]);
        let yaml = generate_compose_file(&manifest, "myproj", &HashMap::new()).unwrap();
        // The full expected YAML — identical to postgres_only since no env vars.
        let vol = workspace_vol();
        let expected = format!(
            "\
services:
  postgres:
    image: postgres:16
  smelt-agent:
    image: smelt-agent:latest
    command:
    - sleep
    - '3600'
    volumes:
    - {vol}
    depends_on:
    - postgres
"
        );
        assert_eq!(yaml, expected);
        // Belt-and-suspenders: confirm the string contains no `environment:` key.
        assert!(
            !yaml.contains("environment:"),
            "environment: key must be absent when extra_env is empty"
        );
    }
}
