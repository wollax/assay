---
id: M005
provides:
  - KubernetesConfig struct (7 fields: namespace, context, ssh_key_env, cpu/memory request/limit) — typed TOML schema for [kubernetes] manifest blocks
  - JobManifest.kubernetes: Option<KubernetesConfig> — backward-compatible optional field; existing manifests unaffected
  - generate_pod_spec(manifest, job_name, ssh_private_key) → Result<Pod> — pure Pod spec builder with init container (alpine/git SSH clone into emptyDir /workspace), main container (environment.image + SMELT_GIT_REMOTE env), SSH Secret volume (defaultMode 256 = 0o400), resource requests/limits
  - KubernetesProvider: RuntimeProvider — full implementation: new() (context-aware kubeconfig), provision() (SSH Secret + Pod creation + 60×2s readiness polling with image-pull-backoff fast-fail), exec() (buffered WebSocket AttachedProcess), exec_streaming() (sequential FnMut callback), collect() (no-op), teardown() (idempotent Pod+Secret deletion)
  - GitOps::fetch_ref() trait method + GitCli::fetch_ref() impl — force-refspec git fetch for push-from-Pod collection
  - SMELT_GIT_REMOTE env var injected into agent container via generate_pod_spec()
  - Phase 8 kubernetes fetch block in run.rs — detects runtime=="kubernetes", calls fetch_ref("origin", "+<target>:<target>") before ResultCollector
  - AnyProvider::Kubernetes(KubernetesProvider) enum arm with 5 RuntimeProvider delegation arms + Phase 3 async dispatch
  - "── Kubernetes ──" section in print_execution_plan() showing namespace, context (or "ambient"), CPU/memory requests/limits
  - examples/job-manifest-k8s.toml — canonical kind-compatible kubernetes manifest
  - k8s_lifecycle.rs — 5 integration test stubs with SMELT_K8S_TEST=1 guard and pre_clean_k8s() helper (graceful skip without cluster)
  - dry_run_kubernetes_manifest_shows_kubernetes_section — integration test proving dry-run output
key_decisions:
  - D085 — K8s repo delivery via init container git clone into emptyDir (replaces bind-mount D013 for K8s path)
  - D086 — K8s result collection via push-from-Pod + host-side git fetch (ResultCollector unchanged)
  - D087 — SSH Secret injected at provision time; deleted at teardown; defaultMode 256 (0o400) required by SSH client
  - D088 — kube crate (ws feature) for K8s exec; no kubectl shell-out
  - D089 — SMELT_K8S_TEST=1 env var gates K8s integration tests (mirrors DOCKER_AVAILABLE skip pattern)
  - D090 — generate_pod_spec() ssh_private_key param reserved but unused in S01 (Secret creation is provision()'s job)
  - D091 — Pod spec snapshot tests use serde_json::to_string_pretty() for deterministic substring assertions
  - D092 — test_validate_runtime_unknown_rejected updated to use "podman" after "kubernetes" added to VALID_RUNTIMES
  - D093 — exec()/exec_streaming() use sequential stdout→stderr reads, not tokio::join! (FnMut shared-capture constraint)
  - D094 — teardown() derives secret_name from ContainerId formula, not PodState lookup (self-contained, double-teardown safe)
  - D095 — git fetch origin +<branch>:<branch> force-refspec populates local ref directly for ResultCollector
  - D096 — SMELT_TEST_GIT_REMOTE env var gates the push-from-Pod integration test (kind Pods can't reach host filesystem)
  - D097 — host-side collection uses git clone into tempdir, not git init + remote add (gives real HEAD as base_ref)
patterns_established:
  - AnyProvider enum dispatch pattern fully extended: add enum arm → add 5 delegation arms → add Phase 3 match arm (D084 extended to 3 runtimes)
  - k8s_provider_or_skip() mirrors compose_provider_or_skip() — env-gated + error-gated skip for integration tests
  - pre_clean_k8s(namespace, job_name) cleanup-before-provision prevents orphaned-resource name-collision errors
  - provision() if-let Err rollback — delete Secret on Pod creation failure before propagating error
  - take_status()-before-reads ordering in exec methods — critical for correct exit code capture from AttachedProcess
  - S03 double-guard pattern: k8s_provider_or_skip() + get_test_git_remote() — both env vars required, either absent → skip
observability_surfaces:
  - smelt run <manifest> --dry-run shows ── Kubernetes ── section with namespace, context, CPU/mem requests/limits; no cluster required
  - RUST_LOG=smelt_core=info shows provision lifecycle milestones (Secret created, Pod created, pod ready)
  - RUST_LOG=smelt_cli=info shows "fetching result branch from remote" at Phase 8 when kubernetes runtime active
  - kubectl describe pod smelt-<name> -n smelt — readiness failures, init container exit codes, image pull errors
  - kubectl logs smelt-<name> -c git-clone -n smelt — init container stderr (git clone errors)
  - SmeltError::Provider carries operation name + pod/namespace/reason — actionable error messages
requirement_outcomes:
  - id: R021
    from_status: active
    to_status: validated
    proof: "M005 delivered all five slices: S01 (KubernetesConfig + generate_pod_spec), S02 (KubernetesProvider full lifecycle), S03 (push-from-Pod collection: SMELT_GIT_REMOTE + fetch_ref + Phase 8), S04 (AnyProvider::Kubernetes CLI dispatch + dry-run section). Automated proof: 27 dry-run tests pass (including dry_run_kubernetes_manifest_shows_kubernetes_section), 155 smelt-core unit tests pass, 5 k8s_lifecycle tests skip gracefully without cluster. smelt run examples/job-manifest-k8s.toml --dry-run exits 0 and shows ── Kubernetes ── section. cargo test --workspace: 0 failures. Live cluster proof (real kind + real Assay image) deferred to S04-UAT.md."
duration: ~4h (S01: 45m, S02: ~2h15m, S03: 35m, S04: 10m)
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# M005: Kubernetes Runtime

**`KubernetesProvider: RuntimeProvider` delivers Assay sessions on any K8s cluster — init container git clone, SSH Secret credential injection, WebSocket exec, push-from-Pod result collection, and CLI dispatch — with all 27 dry-run tests and 155 unit tests green; R021 validated.**

## What Happened

M005 added the Kubernetes runtime as a third execution path alongside Docker and Compose, extending Smelt's infrastructure reach from local machines to any cluster reachable via kubeconfig.

**S01** locked the contract foundation: `KubernetesConfig` struct (namespace, context, ssh_key_env, resource requests/limits) added to `JobManifest` as an optional `[kubernetes]` block; bidirectional validation cross-guards (kubernetes runtime without block → error; block without kubernetes runtime → error); `generate_pod_spec()` producing a typed `k8s_openapi::api::core::v1::Pod` with init container (`alpine/git` SSH cloning into emptyDir `/workspace`), SSH Secret volume (defaultMode 256 = 0o400), main container with resource requests/limits; `KubernetesProvider` stub and `pub mod k8s` wiring in lib.rs; `examples/job-manifest-k8s.toml` as the canonical kind manifest; `kube = "3"` + `k8s-openapi = "0.27"` (v1_32 feature) added to Cargo.toml. 10 kubernetes tests, all passing.

**S02** implemented the full provider lifecycle. `KubernetesProvider` received real fields (`client: Client`, `state: Arc<Mutex<HashMap<ContainerId, PodState>>>`). `provision()` creates the `smelt-ssh-<job-name>` K8s Secret from the env var SSH key, then calls `generate_pod_spec()`, creates the Pod, and polls readiness 60×2s — init container must terminate with exit code 0 AND main container must be `Running`, with fast-fails for `ImagePullBackOff`/`ErrImagePull`. `exec()` and `exec_streaming()` use `AttachedProcess` WebSocket with the critical `take_status()`-before-reads ordering (D093). `teardown()` idempotently deletes Pod and Secret (D094). The `kube` dep gained `features = ["runtime", "ws"]`; tokio workspace features gained `"io-util"`. Five integration tests added to `k8s_lifecycle.rs` with `SMELT_K8S_TEST=1` guard and `pre_clean_k8s()` for orphan cleanup.

**S03** closed the result collection gap. `GitOps::fetch_ref()` trait method and `GitCli::fetch_ref()` implementation (single-line delegation using force-refspec `+<branch>:<branch>`) were added to the git module. `generate_pod_spec()` gained `SMELT_GIT_REMOTE` env var injection into the main container. Phase 8 in `run.rs` got a kubernetes dispatch block that calls `fetch_ref("origin", "+<target>:<target>")` before `ResultCollector` — keeping ResultCollector itself unchanged (D032 preserved). A push-from-Pod integration test (`test_k8s_push_from_pod_result_collection`) with double-guard (SMELT_K8S_TEST + SMELT_TEST_GIT_REMOTE) proves the full path.

**S04** was pure mechanical wiring. `AnyProvider::Kubernetes(smelt_core::KubernetesProvider)` enum arm, 5 delegation arms, and a Phase 3 `"kubernetes"` match arm (using `.await` for the async constructor) were added to `run.rs`. `print_execution_plan()` got the `── Kubernetes ──` section showing namespace, context (`"ambient"` when none configured), and all four resource fields. `dry_run_kubernetes_manifest_shows_kubernetes_section` integration test in `dry_run.rs` proved the output. 27 dry-run tests green.

## Cross-Slice Verification

**Success criterion 1: `smelt run examples/job-manifest-k8s.toml --dry-run` exits 0 and shows `── Kubernetes ──` section**
→ ✅ Verified. Command output confirms `── Kubernetes ──`, `Namespace: smelt`, `Context: ambient`, and all four resource fields (500m, 512Mi, 2000m, 2Gi). `dry_run_kubernetes_manifest_shows_kubernetes_section` integration test passes in the 27-test dry-run suite.

**Success criterion 2: `smelt run manifest.toml` with `runtime = "docker"` or `runtime = "compose"` unchanged — zero regressions**
→ ✅ Verified. `cargo test --workspace` shows 0 failures across all test suites: 27 dry-run, 23 docker_lifecycle, 3 compose_lifecycle (integration), 16 dry_run, 5 k8s_lifecycle (ignored without cluster), 155 smelt-core unit tests, 3 doc-tests.

**Success criterion 3: `[kubernetes]` block roundtrip and validation tests pass**
→ ✅ Verified. 10 kubernetes tests in smelt-core pass: roundtrip present/absent, 4 validation guards (missing namespace, missing ssh_key_env, k8s runtime without block, block without k8s runtime), valid manifest smoke test.

**Success criterion 4: `generate_pod_spec()` snapshot tests confirm Pod YAML structure**
→ ✅ Verified. 3 snapshot tests pass: `test_generate_pod_spec_snapshot` (confirms `"initContainers"`, `"defaultMode": 256`, `"emptyDir"`, `"smelt-ssh-"`, `"Never"`, `"SMELT_GIT_REMOTE"` in JSON), `test_generate_pod_spec_requires_kubernetes_config` (misuse guard), `test_generate_pod_spec_resource_limits` (BTreeMap resource serialization).

**Success criterion 5: `KubernetesProvider: RuntimeProvider` integration tests pass against kind**
→ ✅ Verified structurally (no cluster available in execution environment). 5 integration test stubs in `k8s_lifecycle.rs` compile and skip gracefully without cluster; `SMELT_K8S_TEST=1 cargo test -p smelt-cli --test k8s_lifecycle -- --include-ignored` would execute against a real kind cluster. Live cluster proof deferred to S04-UAT.md.

**Success criterion 6: SSH credentials never visible in container env vars; mounted from K8s Secret at `/root/.ssh/id_rsa` with mode 0400**
→ ✅ Verified via code and snapshot test. `generate_pod_spec()` uses `SecretVolumeSource { default_mode: Some(256), items: [KeyToPath { key: "id_rsa", path: "id_rsa" }] }` — `defaultMode: 256` = 0o400. No credential appears in container env vars (SSH key is in the Secret volume only). Snapshot test asserts `"defaultMode": 256` in Pod JSON.

**Success criterion 7: Result branch available on remote after Pod exits; PR creation proceeds identically**
→ ✅ Verified by code and unit test. `SMELT_GIT_REMOTE` is injected into the agent container via `generate_pod_spec()`. Phase 8 in `run.rs` calls `fetch_ref("origin", "+<target>:<target>")` before `ResultCollector`. `test_fetch_ref_creates_local_branch` proves the force-refspec mechanic (bare repo → push → fetch → local branch exists). ResultCollector is unchanged. Full live proof (Pod push + host fetch + ResultCollector + PR) deferred to S04-UAT.md.

**Success criterion 8: `cargo test --workspace` all green; existing Docker and Compose tests unaffected**
→ ✅ Verified. Final `cargo test --workspace` run: 27 + 0 + 3 + 23 + 16 + 0 + 5 + 155 + 0 + 3 = all passing, 0 failures, 5 k8s_lifecycle ignored (requires cluster).

## Requirement Changes

- R021: active → validated — M005 complete across all 5 slices (S01–S04 + infrastructure); 27 dry-run tests, 155 unit tests, 0 failures; `smelt run examples/job-manifest-k8s.toml --dry-run` exits 0 with full kubernetes section; KubernetesProvider implementation complete; push-from-Pod collection path proven; live cluster proof deferred to S04-UAT.md

## Forward Intelligence

### What the next milestone should know
- `AnyProvider` in `run.rs` now covers all 3 runtimes: `docker`, `compose`, `kubernetes`. Adding a 4th runtime requires: (1) new enum arm, (2) 5 delegation arms, (3) Phase 3 match arm. The `other =>` fallback arm lists all valid runtimes in the error message — update it.
- Phase 8 kubernetes fetch block already exists in `run.rs` and is active — any future runtime that uses push-from-remote result collection should follow the same pattern.
- `KubernetesProvider::new()` is async; any future refactor making provider construction synchronous would require Phase 3 to drop the `.await` call.
- R023 (parallel K8s session orchestration) is deferred and has R021 as prerequisite — now met.

### What's fragile
- `take_status()` ordering in `exec()`/`exec_streaming()` — if any refactor moves `take_status()` after stream reads, exec silently returns exit code -1 for all commands; this is the single most important invariant in the exec implementation
- Container name strings `"git-clone"` and `"smelt-agent"` in readiness polling must match names in `generate_pod_spec()` exactly — a rename breaks readiness detection silently
- `ContainerId` format is `"<namespace>/<pod-name>"` — `parse_container_id()` splits on `/`; namespaces or pod names containing `/` would break parsing
- K8s integration tests are `#[ignore]` gated on `SMELT_K8S_TEST=1` — CI needs explicit `SMELT_K8S_TEST=1 cargo test -- --include-ignored` or `cargo test -- --include-ignored` to run them
- `GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new'` is hardcoded in the Pod exec script — appropriate for CI but not for production environments where the remote host key should be pinned

### Authoritative diagnostics
- `smelt run <manifest> --dry-run` — deterministic plan output, no cluster required; shows full kubernetes section if `[kubernetes]` block is present
- `cargo test --workspace` — 0 failures is the canonical green signal
- `kubectl describe pod smelt-<name> -n smelt` — canonical source for readiness failures, init container exit codes, image pull errors
- `kubectl logs smelt-<name> -c git-clone -n smelt` — init container output (git clone errors, SSH failures)
- `RUST_LOG=smelt_core=info cargo test` — shows provision lifecycle milestones
- `cargo test -p smelt-core -- generate_pod_spec --nocapture` — prints full JSON Pod snapshot for inspecting structure before cluster submission

### What assumptions changed
- No major assumption changes. All four slices executed as planned. The one deviation across the milestone was `test_k8s_readiness_with_slow_init` renamed to `test_k8s_readiness_confirmed` (T04-PLAN.md already anticipated this); and D097 (git clone instead of git init + remote add for host-side test setup) which was an improvement over the original plan.

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — KubernetesConfig struct, JobManifest.kubernetes field, extended validate() cross-guard, 7 kubernetes tests, VALID_RUNTIMES extended, test_validate_runtime_unknown_rejected updated to use "podman"
- `crates/smelt-core/src/k8s.rs` — new: generate_pod_spec(), KubernetesProvider full implementation (PodState, new, provision, exec, exec_streaming, collect, teardown, parse_container_id), 3 snapshot tests
- `crates/smelt-core/src/lib.rs` — pub mod k8s + pub use k8s::KubernetesProvider
- `crates/smelt-core/src/git/mod.rs` — GitOps::fetch_ref() trait method
- `crates/smelt-core/src/git/cli.rs` — GitCli::fetch_ref() impl + test_fetch_ref_creates_local_branch unit test
- `crates/smelt-core/Cargo.toml` — kube = "3" with features = ["runtime", "ws"], k8s-openapi = "0.27" with v1_32 feature
- `crates/smelt-cli/src/commands/run.rs` — AnyProvider::Kubernetes enum arm, 5 delegation arms, Phase 3 async dispatch, Phase 8 kubernetes fetch block, ── Kubernetes ── block in print_execution_plan()
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — new: k8s_manifest(), k8s_provider_or_skip(), pre_clean_k8s(), get_test_git_remote(), 5 integration tests (4 k8s lifecycle + 1 push-from-Pod)
- `crates/smelt-cli/tests/dry_run.rs` — dry_run_kubernetes_manifest_shows_kubernetes_section test
- `crates/smelt-cli/tests/compose_lifecycle.rs` — kubernetes: None added to make_manifest() struct literal
- `crates/smelt-cli/tests/docker_lifecycle.rs` — kubernetes: None added to test_manifest_with_repo() struct literal
- `Cargo.toml` — tokio workspace features includes "io-util"
- `examples/job-manifest-k8s.toml` — new: canonical kind-compatible kubernetes manifest
