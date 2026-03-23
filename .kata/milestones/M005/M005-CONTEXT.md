# M005: Kubernetes Runtime — Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. M001–M004 delivered single-container (DockerProvider) and multi-container (ComposeProvider) provisioning, real Assay integration, GitHub PR creation/tracking, and a stable `smelt-core` library API. M005 extends the runtime to Kubernetes, enabling Assay sessions to run on remote cluster nodes rather than the local machine.

## Why This Milestone

Single-machine execution (Docker and Docker Compose) is the limit of what M001–M004 provide. A `KubernetesProvider: RuntimeProvider` unblocks remote execution on any cluster (local via kind/minikube, or managed via EKS/GKE/AKS), enabling R021 (multi-machine coordination). This is the first step toward distributing Assay sessions across machines.

The bind-mount model (D013) that works for Docker does not extend to K8s — a Pod scheduled on a remote node cannot see the host filesystem. M005 replaces bind-mount with an init container that git-clones the repo into a shared `emptyDir`, and replaces host-side result collection (ResultCollector reading the local git tree) with push-from-Pod: the agent pushes the result branch from inside the container, and `run.rs` fetches the ref from the remote instead.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Add `runtime = "kubernetes"` to `[environment]` plus a `[kubernetes]` block (namespace, optional context, resource requests/limits) to a manifest, run `smelt run manifest.toml`, and have the Assay session execute inside a Kubernetes Pod on the configured cluster
- Have the result branch appear on the remote after the Pod exits — `smelt run` fetches it and creates the PR as normal
- Run `smelt run manifest.toml --dry-run` with a Kubernetes manifest and see the `── Kubernetes ──` section in the execution plan
- Test end-to-end against a local kind/minikube cluster before targeting a managed cluster

### Entry point / environment

- Entry point: `smelt run <manifest.toml>` CLI
- Environment: local dev with kubectl configured (kind/minikube) or a managed cluster; Docker daemon optional (not required for K8s path)
- Live dependencies: K8s cluster (kind for tests), `kubectl` / kubeconfig at `~/.kube/config`, SSH key for git push in a K8s Secret

## Completion Class

- Contract complete means: `[kubernetes]` block parses and validates; `generate_pod_spec()` snapshot tests confirm init container, SSH mount, volume, resource limits; validation tests for namespace/context/resource fields
- Integration complete means: `KubernetesProvider::provision()` creates a real Pod on kind, exec runs a command inside it, teardown deletes the Pod — confirmed by integration tests with `#[ignore]` skip when cluster unavailable
- Operational complete means: `smelt run examples/job-manifest-k8s.toml --dry-run` shows `── Kubernetes ──` section; result collection works via push-from-Pod → fetch remote ref in `run.rs`

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `smelt run examples/job-manifest-k8s.toml --dry-run` exits 0 and shows the `── Kubernetes ──` section
- `KubernetesProvider` integration tests pass against a real kind cluster: provision (Pod created, init container clones repo) → exec (command runs in Pod) → teardown (Pod deleted, namespace clean)
- Result branch push-from-Pod works: after Assay exits in the Pod, the result branch appears on the remote; `run.rs` fetches it with `git fetch origin` before handing off to ResultCollector
- `cargo test --workspace` is green with zero regressions — existing Docker and Compose tests are unaffected
- `docker ps` and Docker-path tests are unaffected — K8s is additive, not a replacement

## Risks and Unknowns

- **`kube` crate exec API** — Pod exec uses WebSocket (`AttachedProcess`, behind `ws` feature flag), not bollard's stream model. The streaming callback interface (`exec_streaming`) must be adapted from WebSocket chunks to the existing `FnMut(&str)` callback. Needs proof before building the rest of the provider. Retire in S02.
- **SSH file permissions in Secrets** — K8s Secrets mount with `0444` by default; SSH requires `0400`. Must set `defaultMode: 256` (octal 0400) on the Secret volume. Known gotcha — must be baked into `generate_pod_spec()`.
- **Push-from-Pod result collection** — The current `ResultCollector` reads the local host git tree (D032). For K8s, the agent container pushes the result branch; `run.rs` must detect `runtime == "kubernetes"` and `git fetch origin <branch>` instead of reading locally. Requires coordinating with AssayInvoker to ensure the agent knows the remote and has push credentials. Retire in S03.
- **Init container repo delivery** — git clone inside an init container requires SSH access to the repo. The same SSH Secret that allows push must allow clone. Key must be authorized for both read and write on the target repo.
- **Pod readiness vs container readiness** — `provision()` must wait for the init container to complete (repo clone done) and the main agent container to be `Running` before returning. `Pod.status.conditions` and container status inspection needed. Retire in S02.

## Existing Codebase / Prior Art

- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait with `provision/exec/exec_streaming/collect/teardown`; `ContainerId` is an opaque `String` wrapper; `KubernetesProvider` will be a new impl
- `crates/smelt-core/src/docker.rs` — `DockerProvider` reference impl; bollard exec model vs `kube` AttachedProcess is the key divergence
- `crates/smelt-core/src/compose.rs` — `ComposeProvider` reference impl; `ComposeProjectState` pattern (internal state map keyed by ContainerId) will be reused for `PodState { namespace, pod_name, secret_name }`
- `crates/smelt-core/src/manifest.rs` — `JobManifest` with `Environment { runtime, image }`; `deny_unknown_fields` on all structs; new `[kubernetes]` section must follow same pattern as `[forge]` (optional struct)
- `crates/smelt-cli/src/commands/run.rs` — `AnyProvider` enum dispatch on `manifest.environment.runtime`; Phase 8 result collection via `ResultCollector`; K8s will add a third `AnyProvider::Kubernetes` variant and modify Phase 8 to fetch remote ref first
- `crates/smelt-cli/tests/dry_run.rs` — reference for adding `── Kubernetes ──` dry-run test pattern
- `crates/smelt-cli/tests/compose_lifecycle.rs` — reference integration test pattern (`compose_provider_or_skip()`, `pre_clean_containers()`, `assert_no_containers_for_job()`)

> See `.kata/DECISIONS.md` for all architectural and pattern decisions. D004 (RuntimeProvider trait as the extension point), D013 (bind-mount revisable if K8s), D019 (RPITIT), D084 (AnyProvider enum) are the most relevant.

## Relevant Requirements

- R021 — Multi-machine coordination: K8s is the primary infrastructure to unblock this; M005 delivers the single-node proof; parallel multi-session scheduling comes in a later milestone

## Scope

### In Scope

- `runtime = "kubernetes"` in `[environment]` triggers `KubernetesProvider`
- `[kubernetes]` block in manifest: `namespace: String` (required), `context: Option<String>` (optional, falls back to ambient kubeconfig), `cpu_request: Option<String>`, `memory_request: Option<String>`, `cpu_limit: Option<String>`, `memory_limit: Option<String>`
- `KubernetesProvider: RuntimeProvider` impl using the `kube` crate (0.98+, `ws` feature for exec)
- Pod spec: init container (`alpine/git`) that git-clones the repo into `emptyDir` at `/workspace`; main container using `environment.image`, mounting the same `emptyDir`; SSH Secret mounted at `/root/.ssh/` with `defaultMode: 256`
- SSH Secret creation at provision time: Smelt creates a `smelt-ssh-<job-name>` Secret in the target namespace from an env var (name configured in `[kubernetes].ssh_key_env`), mounts it into the Pod, deletes it at teardown
- Pod readiness wait: poll `Pod.status.container_statuses` until `state.running` or detect `state.terminated` (error)
- `exec()` and `exec_streaming()` via `kube` AttachedProcess (WebSocket, `ws` feature flag)
- Push-from-Pod result collection: `run.rs` Phase 8 detects `runtime == "kubernetes"`, calls `git fetch origin <target_branch>` on the host before handing to `ResultCollector`; requires `SMELT_GIT_REMOTE` env var injected into agent container
- `AnyProvider::Kubernetes(KubernetesProvider)` variant added to enum in `run.rs`
- `── Kubernetes ──` section in `print_execution_plan()` when `runtime = "kubernetes"`
- `examples/job-manifest-k8s.toml` with a kind-compatible Pod spec
- Integration tests tagged `#[ignore]` unless `SMELT_K8S_TEST=1` env var is set (parallel to Docker `#[cfg(docker)]` skip pattern)
- `cargo test --workspace` remains green with zero regressions

### Out of Scope / Non-Goals

- Node selectors, tolerations, service accounts, RBAC — scheduling policy deferred to a later milestone
- Parallel multi-session K8s orchestration (Symphony-style backlog polling) — later milestone
- CronJob or batch Job resources — plain Pod is sufficient for M005
- PersistentVolumeClaims for the workspace — `emptyDir` is sufficient; PVC lifetime management is out of scope
- Multi-cluster routing — single kubeconfig context only
- `kubectl` CLI dependency — use `kube` crate API only; no shell-out to kubectl
- Helm chart or Operator for Smelt itself — deployment packaging deferred
- Windows host support for K8s (kubeconfig path handling)
- GitLab / Azure DevOps forge support
- crates.io publish

## Technical Constraints

- D004 firm: `RuntimeProvider` trait is the abstraction; `KubernetesProvider` is the new impl
- D013 revisable: bind-mount replaced by init container git clone for K8s (D013 was explicitly marked "Yes — if K8s needs volumes")
- D019 firm: RPITIT (no `async_trait`); `KubernetesProvider` must follow the same async fn in trait pattern
- D084 firm: `AnyProvider` enum in `run.rs` — add `Kubernetes` variant, implement delegation via match
- `kube` crate version: 0.98+ (current as of 2026-03); requires `ws` feature for exec/attach
- `k8s-openapi` crate: matching version for K8s 1.32 types
- Pod exec via `AttachedProcess` uses WebSocket — different from bollard's HTTP stream model; must adapt to `FnMut(&str)` callback
- SSH Secret `defaultMode: 256` (0o400) is required — K8s defaults to 0o444 which SSH rejects
- Push-from-Pod requires `SMELT_GIT_REMOTE` env var in the agent container so the agent knows where to push
- Test cluster: kind (Kubernetes in Docker) is the local test target; `SMELT_K8S_TEST=1` gates integration tests

## Integration Points

- **K8s cluster** — via `kube` crate; reads ambient `~/.kube/config` or in-cluster config; optional context override in manifest
- **`kube` crate** — `kube::Client`, `kube::api::Api<Pod>`, `kube::runtime` for watch/readiness; `ws` feature for AttachedProcess
- **`k8s-openapi`** — typed K8s resource definitions (Pod, Secret, ResourceRequirements, etc.)
- **SSH Secret** — K8s Secret containing private key; created by Smelt at provision time, deleted at teardown
- **git** — called from inside the init container (`alpine/git`) for clone, and inside the agent container for push; `SMELT_GIT_REMOTE` tells the agent where to push
- **Assay** — unchanged; runs inside the main agent container the same way as Docker/Compose; no K8s-specific Assay changes
- **`ResultCollector`** — Phase 8 in `run.rs` adds a `git fetch origin <branch>` step before calling ResultCollector when `runtime == "kubernetes"`

## Open Questions

- Should `smelt run` print progress during the init container clone? (e.g., `Cloning repository into Pod...`) — Yes, good UX; stderr during provision wait.
- What happens if the SSH key doesn't have push access? — The agent will fail to push; Assay will surface a git error; Smelt's Phase 8 `git fetch` will fail to find the ref; surfaces as a `ResultCollector` error. Acceptable for M005 — clean error path, no silent failure.
- Should the SSH Secret be automatically cleaned up even if provision fails (e.g., Pod create fails)? — Yes; teardown must be called in both success and error paths (D023); `KubernetesProvider::teardown()` deletes both the Pod and the Secret idempotently.
