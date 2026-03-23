# M005: Kubernetes Runtime

**Vision:** Smelt provisions Assay sessions on Kubernetes — a `KubernetesProvider: RuntimeProvider` creates a Pod with an init container that git-clones the repo, runs the agent container using the configured image, execs Assay via WebSocket attach, and tears down cleanly. The agent pushes its result branch from inside the Pod; `run.rs` fetches it from the remote before handing off to `ResultCollector`. Any cluster reachable via kubeconfig works; kind/minikube is the local test target.

## Success Criteria

- `smelt run examples/job-manifest-k8s.toml` with `runtime = "kubernetes"` creates a Pod on the configured cluster, runs Assay in the agent container (which has the repo at `/workspace`), and tears down — `kubectl get pods -n <namespace>` shows nothing after completion
- The result branch is available on the remote after the Pod exits — PR creation proceeds identically to the Docker path
- `smelt run examples/job-manifest-k8s.toml --dry-run` exits 0 and shows a `── Kubernetes ──` section listing namespace, context, and resource requests — without touching the cluster
- SSH credentials are never visible in container env vars — they are mounted from a K8s Secret at `/root/.ssh/id_rsa` with file mode `0400`
- `smelt run manifest.toml` with `runtime = "docker"` or `runtime = "compose"` is completely unchanged — zero regressions in the workspace test suite
- Integration tests against a real kind cluster (`SMELT_K8S_TEST=1`) pass: provision (init clone + agent running) → exec → teardown (Pod + Secret deleted)

## Key Risks / Unknowns

- **`kube` crate exec via WebSocket** — `AttachedProcess` (behind `ws` feature flag) is different from bollard's HTTP stream model; adapting to `FnMut(&str)` streaming callback and collecting exit code needs proof before building the rest of the provider. No working example in existing Smelt code.
- **Push-from-Pod result collection** — `ResultCollector` currently reads the local bind-mounted git tree (D032). Switching to fetch-remote-ref on the K8s path requires coordinating with the git credential injection so the agent has push access, and detecting the right branch name after Assay exits inside the Pod.
- **Pod readiness detection** — `provision()` must reliably detect when the init container has completed and the main container is `Running` (not `Waiting` or `Terminated`). Polling `Pod.status.container_statuses` needs a correct state machine that handles init container failure, image pull errors, and race conditions.
- **SSH file permissions** — K8s Secrets mount as `0444` by default; SSH rejects this. `defaultMode: 256` (octal 0400) must be set explicitly in the Secret volume mount. One missed field breaks the entire git clone.

## Proof Strategy

- **`kube` exec WebSocket** → retire in S02 by integration-testing `exec("echo hello")` against a real kind Pod before building `exec_streaming` or the Assay invocation path.
- **Push-from-Pod result collection** → retire in S03 by running a full end-to-end integration test: provision Pod, exec a script that creates a git commit and pushes to a bare repo, verify `run.rs` Phase 8 fetches the ref and ResultCollector finds the branch.
- **Pod readiness detection** → retire in S02 by testing the readiness polling loop against a real kind Pod with an init container that sleeps for 2 seconds before completing — confirm provision() returns only after the main container is Running.
- **SSH file permissions** → retire in S02 by verifying the init container can actually run `git clone` via SSH using the mounted Secret (mode 0400 confirmed by `stat /root/.ssh/id_rsa` exec in tests).

## Verification Classes

- Contract verification: `[kubernetes]` block roundtrip and validation tests; `generate_pod_spec()` snapshot tests for init container, SSH Secret mount, resource limits, volume, command; `runtime = "kubernetes"` validation guard in `validate()`
- Integration verification: `KubernetesProvider` lifecycle tests (skip if `SMELT_K8S_TEST!=1`): provision + exec + teardown on real kind cluster; push-from-Pod test with bare git remote; SSH Secret cleanup confirmed by `kubectl get secret` after teardown
- Operational verification: `smelt run --dry-run` shows `── Kubernetes ──` section; Ctrl+C during provision/exec calls teardown (Pod + Secret deleted); `cargo test --workspace` green with no regressions
- UAT / human verification: `smelt run examples/job-manifest-k8s.toml` against a real cluster with a real Assay session — deferred to S04-UAT.md

## Milestone Definition of Done

This milestone is complete only when all are true:

- `[kubernetes]` block in `JobManifest` with `namespace`, `context`, `ssh_key_env`, and resource requests/limits; roundtrip and validation tests pass
- `generate_pod_spec()` produces a valid Pod spec with init container, emptyDir, SSH Secret volume (mode 0400), and resource requirements; snapshot tests confirm the Pod YAML structure
- `KubernetesProvider: RuntimeProvider` passes integration tests against real kind: provision (init clone + main Running) → exec (echo hello, exit 0) → exec_streaming (output callback fires) → teardown (Pod + Secret deleted, namespace clean)
- `smelt run examples/job-manifest-k8s.toml` runs end-to-end with a real cluster: Assay executes, result branch is pushed from Pod, `run.rs` fetches it, PR is created (or `--no-pr` skips PR)
- `smelt run examples/job-manifest-k8s.toml --dry-run` exits 0 and shows `── Kubernetes ──` section
- `cargo test --workspace` all green; existing Docker and Compose tests are unaffected

## Requirement Coverage

- Covers: R021 (multi-machine coordination — K8s single-node proof; parallel scheduling deferred to later milestone)
- Partially covers: none
- Leaves for later: R022 (budget tracking), parallel multi-session K8s scheduling
- Orphan risks: none

## Slices

- [x] **S01: Manifest Extension** `risk:low` `depends:[]`
  > After this: `cargo test -p smelt-core` proves `[kubernetes]` roundtrip and validation; `smelt run --dry-run` parses a kubernetes manifest without errors; `generate_pod_spec()` snapshot tests confirm init container, SSH Secret mount, volume, and resource limits in the Pod YAML; `cargo test --workspace` shows zero regressions.

- [x] **S02: KubernetesProvider Lifecycle** `risk:high` `depends:[S01]`
  > After this: `KubernetesProvider: RuntimeProvider` is integration-tested against a real kind cluster — provision (init clone + agent Running), exec (echo hello), exec_streaming (callback fires), teardown (Pod + Secret deleted); `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle` passes; `kube` exec WebSocket risk and Pod readiness risk retired.

- [x] **S03: Push-from-Pod Result Collection** `risk:high` `depends:[S01,S02]`
  > After this: Assay running inside a K8s Pod pushes its result branch to the remote; `run.rs` Phase 8 detects `runtime == "kubernetes"`, runs `git fetch origin` on the host, and hands the ref to ResultCollector; PR creation proceeds identically; push-from-Pod risk retired with integration test.

- [x] **S04: CLI Integration + Dry-Run** `risk:low` `depends:[S01,S02,S03]`
  > After this: `smelt run` dispatches to `KubernetesProvider` on `runtime = "kubernetes"` via `AnyProvider::Kubernetes` variant; `--dry-run` shows `── Kubernetes ──` section; `examples/job-manifest-k8s.toml` is a working kind-compatible example; `smelt run manifest.toml` for `runtime = "docker"` and `runtime = "compose"` are provably unchanged; `cargo test --workspace` all green.

## Boundary Map

### S01 → S02, S03, S04

Produces:
- `KubernetesConfig` struct: `namespace: String`, `context: Option<String>`, `ssh_key_env: String`, `cpu_request: Option<String>`, `memory_request: Option<String>`, `cpu_limit: Option<String>`, `memory_limit: Option<String>`
- `JobManifest.kubernetes: Option<KubernetesConfig>` — parsed from `[kubernetes]` TOML table; absent by default
- `Environment.runtime` allowlist extended to include `"kubernetes"`; validation guard: if `runtime == "kubernetes"` then `kubernetes` block must be present; if `runtime != "kubernetes"` and `kubernetes` block is present, validation error
- `validate()` per-field checks: `namespace` non-empty; `ssh_key_env` non-empty; resource values are valid Kubernetes quantity strings (non-empty if present; format validation deferred to K8s API)
- `pub fn generate_pod_spec(manifest: &JobManifest, job_name: &str, ssh_private_key: &str) -> crate::Result<Pod>` — produces a `k8s_openapi::api::core::v1::Pod` value with: init container (`alpine/git` cloning repo via SSH into `/workspace` emptyDir), main container (using `environment.image`, `/workspace` mounted, credentials env), SSH Secret volume (mode 0400), resource requests/limits from `kubernetes` block
- `pub struct KubernetesProvider {}` stub in `smelt_core::k8s` module (full impl deferred to S02)
- `pub mod k8s` + `pub use k8s::KubernetesProvider` wired into `lib.rs`
- Unit tests: manifest roundtrip with `[kubernetes]` present and absent; validation errors for missing namespace, missing ssh_key_env, runtime mismatch; `generate_pod_spec()` snapshot tests for Pod YAML structure

Consumes:
- nothing (independent)

### S02 → S03, S04

Produces:
- `KubernetesProvider` with: `client: kube::Client`, `state: Arc<Mutex<HashMap<ContainerId, PodState>>>`
- `PodState { namespace: String, pod_name: String, secret_name: String }` — private internal type
- `KubernetesProvider::new(manifest: &JobManifest) -> crate::Result<Self>` — connects to cluster via ambient kubeconfig or manifest-specified context
- `provision(manifest) -> ContainerId`: creates SSH Secret (`smelt-ssh-<job-name>`) in namespace, creates Pod from `generate_pod_spec()`, polls `Pod.status` until init container done AND main container Running (60×2s timeout), returns opaque ContainerId = `<namespace>/<pod-name>`
- `exec(container, command) -> ExecHandle`: WebSocket attach via `AttachedProcess`; reads stdout/stderr; extracts exit code from `TerminationMessage` or container status
- `exec_streaming(container, command, cb) -> ExecHandle`: same WebSocket attach; calls `cb` for each stdout/stderr chunk; buffers full output for ExecHandle
- `collect(container, manifest) -> CollectResult`: no-op (collection handled in Phase 8 of `run.rs`)
- `teardown(container) -> ()`: deletes Pod and SSH Secret by name; fault-tolerant (404 = already gone; errors logged, not propagated per D023)
- `kube`, `k8s-openapi`, `tokio-tungstenite` added to `smelt-core/Cargo.toml`
- Integration tests (skip if `SMELT_K8S_TEST!=1`): provision+exec+teardown; exec_streaming callback; SSH file permission verification; Pod readiness wait with slow init container

Consumes from S01:
- `KubernetesConfig`, `JobManifest.kubernetes`, `generate_pod_spec()`, `KubernetesProvider` stub

### S03 → S04

Produces:
- `run.rs` Phase 8 branch: `if manifest.environment.runtime == "kubernetes" { git fetch origin <target_branch> }` on the host before calling `ResultCollector` — adapts push-from-Pod to the existing collect flow
- `SMELT_GIT_REMOTE` env var injected into the agent container at provision time (value = `manifest.job.repo` resolved to an SSH remote URL, or read from a manifest field)
- Integration test: provision K8s Pod, exec script that commits a file and pushes to a bare git remote, verify `run.rs` Phase 8 fetches the ref, verify `ResultCollector` finds the branch — proves full collection path end-to-end
- Confirmed: `ResultCollector` is unchanged — it reads a local ref that `git fetch` has populated

Consumes from S01:
- `JobManifest.kubernetes`, `KubernetesConfig.ssh_key_env`

Consumes from S02:
- `KubernetesProvider` full impl, `PodState`, credential injection at provision time

### S04 (final wiring — no new public surfaces)

Produces:
- `AnyProvider::Kubernetes(KubernetesProvider)` variant in `run.rs` enum + RuntimeProvider delegation impl
- `match manifest.environment.runtime.as_str() { "docker" => Docker, "compose" => Compose, "kubernetes" => Kubernetes, _ => error }`
- `print_execution_plan()` extension: `── Kubernetes ──` section listing namespace, context (or "ambient"), CPU/memory requests when `runtime = "kubernetes"`
- `examples/job-manifest-k8s.toml` with a kind-compatible namespace and resource requests
- `cargo test --workspace` all green; existing `runtime = "docker"` and `runtime = "compose"` integration tests unaffected

Consumes from S01, S02, S03:
- `JobManifest.kubernetes`, `KubernetesConfig`, `KubernetesProvider: RuntimeProvider`
