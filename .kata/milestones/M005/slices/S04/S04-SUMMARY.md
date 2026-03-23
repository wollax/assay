---
id: S04
parent: M005
milestone: M005
provides:
  - AnyProvider::Kubernetes(KubernetesProvider) enum arm with 5 RuntimeProvider delegation arms
  - Phase 3 dispatch arm for "kubernetes" runtime with async KubernetesProvider::new().await
  - "── Kubernetes ──" section in print_execution_plan() showing namespace, context, CPU/memory requests/limits
  - dry_run_kubernetes_manifest_shows_kubernetes_section integration test in dry_run.rs
  - examples/job-manifest-k8s.toml as the canonical kind-compatible example manifest
  - R021 (multi-machine K8s coordination) validated — all five M005 slices complete
requires:
  - slice: S01
    provides: KubernetesConfig, JobManifest.kubernetes, generate_pod_spec(), KubernetesProvider stub
  - slice: S02
    provides: KubernetesProvider full impl (provision, exec, exec_streaming, collect, teardown)
  - slice: S03
    provides: Phase 8 kubernetes fetch block in run.rs, SMELT_GIT_REMOTE injection, GitOps::fetch_ref()
affects: []
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
  - examples/job-manifest-k8s.toml
key_decisions:
  - "AnyProvider::Kubernetes(KubernetesProvider) follows D084 pattern — enum arm + 5 delegation arms + Phase 3 match arm"
  - "KubernetesProvider::new() is async; Phase 3 arm must .await unlike Docker/Compose constructors (which are sync)"
  - "Kubernetes dry-run section gated on manifest.kubernetes.is_some() (not on runtime string), consistent with Forge/ComposeServices guard pattern"
  - "Unsupported runtime error message updated to mention kubernetes alongside docker, compose"
patterns_established:
  - "AnyProvider dispatch pattern fully extended: add enum arm → add 5 delegation arms → add Phase 3 match arm (D084 extended to cover all 3 runtimes)"
observability_surfaces:
  - "smelt run <manifest> --dry-run shows ── Kubernetes ── section with namespace, context, CPU/mem requests/limits; no cluster required"
  - "Live path: KubernetesProvider::new() errors surface as 'failed to connect to Kubernetes cluster: <cause>' via anyhow chain on stderr"
drill_down_paths:
  - .kata/milestones/M005/slices/S04/tasks/T01-SUMMARY.md
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S04: CLI Integration + Dry-Run

**Wired `AnyProvider::Kubernetes` dispatch and `── Kubernetes ──` dry-run section; all 27 dry-run tests green, 0 workspace failures; R021 validated.**

## What Happened

One task, three mechanical wiring changes in two files:

1. **`AnyProvider::Kubernetes(smelt_core::KubernetesProvider)` enum arm** added to the local enum in `run.rs`, alongside existing Docker and Compose variants.

2. **5 RuntimeProvider delegation arms** added to the `AnyProvider` impl — `provision`, `exec`, `exec_streaming`, `collect`, `teardown` — each delegating to the inner `KubernetesProvider` via `p.<method>(...).await`.

3. **Phase 3 dispatch arm** added: `"kubernetes" => AnyProvider::Kubernetes(smelt_core::KubernetesProvider::new(&manifest).await.with_context(|| "failed to connect to Kubernetes cluster")?)`. This arm correctly uses `.await` since `KubernetesProvider::new()` is async, unlike the sync Docker and Compose constructors.

4. **`── Kubernetes ──` block** added to `print_execution_plan()` after `── Compose Services ──` and before `── Forge ──`, gated on `manifest.kubernetes.is_some()`. Shows: Namespace, Context (or `"ambient"` when no context is set), and optional CPU request, memory request, CPU limit, memory limit.

5. **`dry_run_kubernetes_manifest_shows_kubernetes_section`** integration test added to `dry_run.rs`, using `examples/job-manifest-k8s.toml` as input. Asserts `── Kubernetes ──`, `smelt` (namespace), and `ambient` (no context configured) appear in stdout.

## Verification

- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` exits 0; stdout contains `── Kubernetes ──`, `Namespace:   smelt`, `Context:     ambient`, and all 4 resource fields (500m, 512Mi, 2000m, 2Gi).
- `cargo test -p smelt-cli --test dry_run` — 27 tests, 0 failures; new `dry_run_kubernetes_manifest_shows_kubernetes_section` passes.
- `cargo test --workspace` — 0 failures across all test suites (27 dry-run, 16 docker_lifecycle, 155 smelt-core, plus unit tests).
- `cargo build -p smelt-cli` — zero warnings from modified files.

## Requirements Advanced

- R021 (Multi-machine coordination via Kubernetes) — S04 is the final slice; all five M005 slices are now complete. K8s dispatch is live in `smelt run`, dry-run surfaces the kubernetes plan section, and `examples/job-manifest-k8s.toml` is the canonical kind-compatible example.

## Requirements Validated

- R021 — Validated by M005 as a whole: S01 (manifest + generate_pod_spec), S02 (KubernetesProvider lifecycle integration tests against kind), S03 (push-from-Pod collection + Phase 8 host-side fetch), S04 (CLI dispatch + dry-run). Live end-to-end proof (real kind cluster + real Assay image) deferred to S04-UAT.md.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- Updated the `other =>` fallback arm error message to list `kubernetes` alongside `docker, compose` — minor improvement not in the plan but consistent with the change (trivial).

## Known Limitations

- Live cluster execution is not exercised in this slice — deferred to S04-UAT.md (human verification with real kind cluster + real Assay image).
- Parallel multi-session K8s scheduling (R023) deferred to a later milestone.

## Follow-ups

- Human UAT: `smelt run examples/job-manifest-k8s.toml` against a real kind cluster with a real Assay session — see S04-UAT.md.
- R023 (parallel K8s orchestration) remains deferred — Symphony-style scheduling requires R021 as prerequisite (now met).

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Added `Kubernetes` enum arm, 5 delegation arms, Phase 3 dispatch arm, `── Kubernetes ──` block in `print_execution_plan()`
- `crates/smelt-cli/tests/dry_run.rs` — Added `dry_run_kubernetes_manifest_shows_kubernetes_section` test

## Forward Intelligence

### What the next slice should know
- M005 is fully complete. The next milestone (R023/Symphony-style parallel orchestration) inherits a clean K8s provider with integration-tested lifecycle, push-from-Pod collection, and CLI dispatch.
- `AnyProvider` in `run.rs` now covers all 3 supported runtimes (`docker`, `compose`, `kubernetes`). Adding a 4th runtime requires: (1) new enum arm, (2) 5 delegation arms, (3) Phase 3 match arm. The `other =>` fallback arm handles unknown runtimes with a user-facing error.

### What's fragile
- K8s integration tests are `#[ignore]` gated on `SMELT_K8S_TEST=1` — they require a kind cluster to be running. If CI gains a kind cluster, these tests need explicit `cargo test -- --include-ignored` or `SMELT_K8S_TEST=1 cargo test` invocation.
- `KubernetesProvider::new()` is async; any future refactor that makes provider construction synchronous would require Phase 3 to drop the `.await` call.

### Authoritative diagnostics
- `smelt run <manifest> --dry-run` — deterministic plan output, no cluster required; shows full kubernetes section if `[kubernetes]` block is present in manifest.
- `cargo test --workspace` — 0 failures is the canonical green signal for this project.

### What assumptions changed
- No assumptions changed — S04 was correctly scoped as pure mechanical wiring with no new architectural decisions.
