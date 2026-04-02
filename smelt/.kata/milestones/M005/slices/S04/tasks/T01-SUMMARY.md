---
id: T01
parent: S04
milestone: M005
provides:
  - AnyProvider::Kubernetes(KubernetesProvider) enum arm in run.rs
  - 5 RuntimeProvider delegation arms for Kubernetes in AnyProvider impl
  - Phase 3 dispatch arm for "kubernetes" runtime with async KubernetesProvider::new().await
  - "── Kubernetes ──" section in print_execution_plan() showing namespace, context, and resource fields
  - dry_run_kubernetes_manifest_shows_kubernetes_section integration test
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
key_decisions:
  - "KubernetesProvider::new() is async; Phase 3 arm uses .await unlike docker/compose arms"
  - "Kubernetes section gated on manifest.kubernetes.is_some() (not on runtime string), mirroring Forge/ComposeServices guard patterns"
  - "Updated unsupported runtime error message to include kubernetes in the supported list"
patterns_established:
  - "AnyProvider dispatch pattern extended: add enum arm, add delegation arm to all 5 RuntimeProvider methods, add Phase 3 match arm"
observability_surfaces:
  - "smelt run <manifest> --dry-run — shows ── Kubernetes ── section with namespace, context, CPU/mem requests/limits; no cluster required"
  - "Live path: KubernetesProvider::new() failures propagate as 'failed to connect to Kubernetes cluster: <cause>' on stderr via anyhow context chain"
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Wire AnyProvider::Kubernetes dispatch and Kubernetes dry-run section

**Wired KubernetesProvider into AnyProvider dispatch (5 delegation arms + Phase 3 async arm) and added `── Kubernetes ──` dry-run section; new integration test passes, all 27 dry-run tests green.**

## What Happened

Three mechanical wiring changes in two files:

1. Added `Kubernetes(smelt_core::KubernetesProvider)` variant to `AnyProvider` enum in `run.rs`.
2. Added `AnyProvider::Kubernetes(p) => p.<method>(...).await` delegation arms to all 5 `RuntimeProvider` methods (`provision`, `exec`, `exec_streaming`, `collect`, `teardown`).
3. Added `"kubernetes"` arm to the Phase 3 `match manifest.environment.runtime.as_str()` block, calling `KubernetesProvider::new(&manifest).await.with_context(|| "failed to connect to Kubernetes cluster")?`. This arm correctly uses `.await` since `KubernetesProvider::new()` is async (unlike the docker/compose constructors).
4. Added the `── Kubernetes ──` block to `print_execution_plan()` after `── Compose Services ──` and before `── Forge ──`, gated on `manifest.kubernetes.is_some()`. Shows namespace, context (falling back to `"ambient"` when not set), and optional cpu_request, memory_request, cpu_limit, memory_limit.
5. Added `dry_run_kubernetes_manifest_shows_kubernetes_section` integration test in `dry_run.rs` using `examples/job-manifest-k8s.toml` as the test subject.

## Verification

- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` exits 0 and stdout contains `── Kubernetes ──`, `Namespace:   smelt`, `Context:     ambient`, plus all 4 resource fields (500m, 512Mi, 2000m, 2Gi).
- `cargo test --workspace` exits 0: all test suites green (27 dry-run tests, 16 docker_lifecycle tests, 155 smelt-core tests, plus unit tests).
- New test `dry_run_kubernetes_manifest_shows_kubernetes_section` passes — confirms `── Kubernetes ──`, `smelt`, and `ambient` appear in stdout.

## Diagnostics

- `smelt run <manifest> --dry-run` — deterministic; shows full kubernetes section without requiring a live cluster
- Live dispatch failures: `KubernetesProvider::new()` errors surface as `"failed to connect to Kubernetes cluster: <cause>"` via anyhow chain on stderr

## Deviations

- Updated the `other =>` fallback arm error message to mention `kubernetes` alongside `docker, compose` — minor improvement not in the plan but consistent with the change.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Added `Kubernetes` enum arm, 5 delegation arms, Phase 3 dispatch arm, `── Kubernetes ──` block in `print_execution_plan()`
- `crates/smelt-cli/tests/dry_run.rs` — Added `dry_run_kubernetes_manifest_shows_kubernetes_section` test
