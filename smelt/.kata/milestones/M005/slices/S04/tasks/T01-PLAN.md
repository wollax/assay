---
estimated_steps: 6
estimated_files: 2
---

# T01: Wire AnyProvider::Kubernetes dispatch and Kubernetes dry-run section

**Slice:** S04 — CLI Integration + Dry-Run
**Milestone:** M005

## Description

Three mechanical wiring changes close the milestone's CLI gap. All three are in two files and fit comfortably in one context window:

1. **`AnyProvider::Kubernetes(KubernetesProvider)` variant** — add the enum arm and all 5 delegation arms to the `RuntimeProvider` impl in `run.rs`.
2. **Phase 3 dispatch** — add `"kubernetes"` arm to the `match manifest.environment.runtime.as_str()` block; `KubernetesProvider::new()` is `async` so the arm must `.await` it.
3. **`── Kubernetes ──` section in `print_execution_plan()`** — gated on `manifest.kubernetes.is_some()`; shows namespace, context (or `"ambient"`), and optional resource requests/limits.
4. **Dry-run integration test** — one new `#[test]` in `dry_run.rs` asserting the kubernetes section appears for `examples/job-manifest-k8s.toml`.

Key constraints from research (S04-RESEARCH.md):
- `KubernetesProvider::new()` is `async` — unlike `DockerProvider::new()` and `ComposeProvider::new()`. The Phase 3 arm must `.await`.
- The `── Kubernetes ──` section must be gated on `manifest.kubernetes.is_some()`, not on the runtime string (mirrors the `── Forge ──` and `── Compose Services ──` guard patterns).
- Phase 8 kubernetes fetch block already exists in `run.rs` (placed by S03) — do NOT add it again.
- The `other =>` fallback arm in Phase 3 may remain for truly unknown runtimes; only the `"kubernetes"` case needs to dispatch to `KubernetesProvider` instead of falling through.

## Steps

1. Open `crates/smelt-cli/src/commands/run.rs`. Add `Kubernetes(smelt_core::KubernetesProvider)` arm to the `AnyProvider` enum (after `Compose`).
2. Add delegation arms to all 5 `RuntimeProvider` methods in the `AnyProvider` impl: `provision`, `exec`, `exec_streaming`, `collect`, `teardown`. Pattern is identical to the existing `Docker` and `Compose` arms — `AnyProvider::Kubernetes(p) => p.<method>(...).await`.
3. In Phase 3 (`match manifest.environment.runtime.as_str()`), add:
   ```rust
   "kubernetes" => AnyProvider::Kubernetes(
       smelt_core::KubernetesProvider::new(&manifest)
           .await
           .with_context(|| "failed to connect to Kubernetes cluster")?,
   ),
   ```
4. In `print_execution_plan()`, add the `── Kubernetes ──` block after `── Compose Services ──` and before `── Forge ──`:
   ```rust
   if let Some(ref kube) = manifest.kubernetes {
       println!("── Kubernetes ──");
       println!("  Namespace:   {}", kube.namespace);
       println!("  Context:     {}", kube.context.as_deref().unwrap_or("ambient"));
       if let Some(ref v) = kube.cpu_request    { println!("  CPU req:     {v}"); }
       if let Some(ref v) = kube.memory_request  { println!("  Mem req:     {v}"); }
       if let Some(ref v) = kube.cpu_limit       { println!("  CPU limit:   {v}"); }
       if let Some(ref v) = kube.memory_limit    { println!("  Mem limit:   {v}"); }
       println!();
   }
   ```
5. In `crates/smelt-cli/tests/dry_run.rs`, add:
   ```rust
   #[test]
   fn dry_run_kubernetes_manifest_shows_kubernetes_section() {
       smelt()
           .args(["run", "examples/job-manifest-k8s.toml", "--dry-run"])
           .assert()
           .success()
           .stdout(
               predicate::str::contains("── Kubernetes ──")
                   .and(predicate::str::contains("smelt"))
                   .and(predicate::str::contains("ambient")),
           );
   }
   ```
6. Run `cargo test --workspace` and confirm zero failures.

## Must-Haves

- [ ] `AnyProvider` enum has `Kubernetes(smelt_core::KubernetesProvider)` arm
- [ ] All 5 `RuntimeProvider` methods in the `AnyProvider` impl have a `Kubernetes(p)` delegation arm
- [ ] Phase 3 match dispatches `"kubernetes"` to `KubernetesProvider::new(&manifest).await` with error context
- [ ] `print_execution_plan()` prints `── Kubernetes ──` with namespace, context, and resource fields when `manifest.kubernetes.is_some()`
- [ ] `dry_run_kubernetes_manifest_shows_kubernetes_section` test passes (success exit, stdout contains `── Kubernetes ──`, `smelt`, `ambient`)
- [ ] `cargo test --workspace` all green — zero regressions in docker or compose tests

## Verification

- `cargo test -p smelt-cli --test dry_run -- dry_run_kubernetes_manifest_shows_kubernetes_section` passes
- `cargo test --workspace` exits 0 with zero failures
- `cargo run --bin smelt -- run examples/job-manifest-k8s.toml --dry-run` stdout contains `── Kubernetes ──`, `Namespace:   smelt`, `Context:     ambient`
- `cargo build -p smelt-cli` produces zero warnings from the two modified files

## Observability Impact

- Signals added/changed: `── Kubernetes ──` section in dry-run output (namespace, context, resource requests/limits) — gives users a human-readable summary of the K8s execution plan before live dispatch
- How a future agent inspects this: `smelt run <manifest> --dry-run` — no cluster required; output is deterministic
- Failure state exposed: live `KubernetesProvider::new()` failures propagate as `"failed to connect to Kubernetes cluster: <cause>"` on stderr; the existing `other =>` arm still catches truly unknown runtimes

## Inputs

- `crates/smelt-cli/src/commands/run.rs` — `AnyProvider` enum (lines ~38–42), `RuntimeProvider` delegation impl (lines ~43–100), Phase 3 match (lines ~191–200), `print_execution_plan()` with `── Compose Services ──` and `── Forge ──` sections (lines ~598–620)
- `crates/smelt-cli/tests/dry_run.rs` — `smelt()` helper + `test_dry_run_with_forge_shows_forge_section` as template for the new test
- `examples/job-manifest-k8s.toml` — the test subject; has `namespace = "smelt"`, no `context` field (→ `"ambient"`)
- S04-RESEARCH.md — confirmed: Phase 8 fetch block already in `run.rs`; `KubernetesProvider::new()` is async; context display uses `unwrap_or("ambient")`

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — `AnyProvider::Kubernetes` arm added; 5 delegation arms added; Phase 3 kubernetes dispatch arm added; `── Kubernetes ──` block added to `print_execution_plan()`
- `crates/smelt-cli/tests/dry_run.rs` — `dry_run_kubernetes_manifest_shows_kubernetes_section` test added; all dry-run tests pass
