# S04: CLI Integration + Dry-Run

**Goal:** Wire `AnyProvider::Kubernetes(KubernetesProvider)` into `run.rs`, add the `── Kubernetes ──` section to `print_execution_plan()`, and add a dry-run integration test — so `smelt run` dispatches to `KubernetesProvider` on `runtime = "kubernetes"` and `--dry-run` shows the kubernetes section.
**Demo:** `smelt run examples/job-manifest-k8s.toml --dry-run` exits 0 and stdout contains `── Kubernetes ──` with namespace, context, and resource fields; `cargo test --workspace` is all green including the new dry-run test; existing `docker` and `compose` runtime paths are unaffected.

## Must-Haves

- `AnyProvider::Kubernetes(smelt_core::KubernetesProvider)` variant exists in `run.rs` with delegation arms in all 5 `RuntimeProvider` methods
- Phase 3 match dispatches `"kubernetes"` → `AnyProvider::Kubernetes(KubernetesProvider::new(&manifest).await?)` and no longer falls through to the `other =>` error arm
- `print_execution_plan()` includes a `── Kubernetes ──` section (gated on `manifest.kubernetes.is_some()`) showing namespace, context (or `"ambient"`), and optional CPU/memory requests/limits
- `dry_run_kubernetes_manifest_shows_kubernetes_section` test in `dry_run.rs` passes: asserts `── Kubernetes ──`, `smelt` (namespace), and context display
- `cargo test --workspace` all green; no regression in docker or compose integration tests

## Proof Level

- This slice proves: final-assembly (closes the dispatch gap — `AnyProvider` enum now covers all three supported runtimes)
- Real runtime required: no (dry-run and unit tests only; live cluster dispatch tested by S02 integration tests)
- Human/UAT required: yes — deferred to S04-UAT.md; live `smelt run examples/job-manifest-k8s.toml` against a real kind cluster is not exercised in this slice

## Verification

- `cargo test -p smelt-cli --test dry_run` — all existing tests pass + new `dry_run_kubernetes_manifest_shows_kubernetes_section` passes
- `cargo test --workspace` — zero failures
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` exits 0 and stdout contains `── Kubernetes ──`, `smelt`, `ambient` (no context set in example manifest)
- `cargo build -p smelt-cli` — zero warnings from modified files

## Observability / Diagnostics

- Runtime signals: no new signals added — `── Kubernetes ──` section in dry-run output is the primary user-facing diagnostic surface; live-path errors from `KubernetesProvider::new()` (kubeconfig missing, cluster unreachable) propagate via `with_context(|| "failed to connect to Kubernetes cluster")` to CLI stderr
- Inspection surfaces: `smelt run <manifest> --dry-run` shows full execution plan including kubernetes section; `smelt run <manifest>` on a missing cluster prints the anyhow context chain to stderr
- Failure visibility: Phase 3 `other =>` fallback arm remains for truly unknown runtimes; kubernetes dispatch errors surface the full anyhow chain
- Redaction constraints: no secrets in dry-run output (namespace, context, resource strings only)

## Integration Closure

- Upstream surfaces consumed: `smelt_core::KubernetesProvider` (S01/S02 stub → full impl), `JobManifest.kubernetes: Option<KubernetesConfig>` (S01), Phase 8 kubernetes fetch block already in `run.rs` (S03)
- New wiring introduced in this slice: `AnyProvider::Kubernetes` variant and delegation arms; Phase 3 `"kubernetes"` dispatch arm; `── Kubernetes ──` block in `print_execution_plan()`
- What remains before the milestone is truly usable end-to-end: live cluster execution (S04-UAT.md — human verification with real kind cluster + real Assay image)

## Tasks

- [x] **T01: Wire AnyProvider::Kubernetes dispatch and Kubernetes dry-run section** `est:30m`
  - Why: All three production changes (AnyProvider variant, print_execution_plan section, dry-run test) are trivial mechanical wiring that fits one context window; splitting would add overhead without benefit
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/dry_run.rs`
  - Do: (1) Add `Kubernetes(smelt_core::KubernetesProvider)` arm to the `AnyProvider` enum; (2) Add delegation arms to all 5 `RuntimeProvider` methods in the `AnyProvider` impl; (3) Add `"kubernetes" => AnyProvider::Kubernetes(smelt_core::KubernetesProvider::new(&manifest).await.with_context(|| "failed to connect to Kubernetes cluster")?)` to Phase 3 match, removing the `other =>` fallback that previously caught it; (4) Add `── Kubernetes ──` section to `print_execution_plan()` after `── Compose Services ──` and before `── Forge ──`; (5) Add `dry_run_kubernetes_manifest_shows_kubernetes_section` test in `dry_run.rs`; (6) Run `cargo test --workspace`
  - Verify: `cargo test -p smelt-cli --test dry_run` shows new test passing; `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` stdout contains `── Kubernetes ──` and `smelt`; `cargo test --workspace` zero failures
  - Done when: `cargo test --workspace` is green and `smelt run examples/job-manifest-k8s.toml --dry-run` shows `── Kubernetes ──` section with correct field values

## Files Likely Touched

- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-cli/tests/dry_run.rs`
