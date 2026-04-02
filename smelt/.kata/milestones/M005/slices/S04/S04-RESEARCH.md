# S04: CLI Integration + Dry-Run — Research

**Date:** 2026-03-23
**Domain:** Rust CLI dispatch, dry-run output formatting, integration test wiring
**Confidence:** HIGH — S01–S03 provide complete foundations; this is pure wiring work

## Summary

S04 is mechanical wiring. Three distinct changes close the milestone:

1. **`AnyProvider::Kubernetes(KubernetesProvider)` variant** — add the enum arm and delegation arms in `run.rs`; update the Phase 3 `match` to dispatch `runtime = "kubernetes"` to `KubernetesProvider::new()`.
2. **`── Kubernetes ──` section in `print_execution_plan()`** — mirrors the existing `── Forge ──` and `── Compose Services ──` sections; shows namespace, context (or "ambient"), and resource requests.
3. **dry-run integration test** — add one test to `dry_run.rs` asserting `── Kubernetes ──` appears for `examples/job-manifest-k8s.toml`.

No new crate dependencies. No new architecture. All the hard work (KubernetesProvider impl, push-from-Pod collection, Phase 8 fetch) is already done.

## Recommendation

Execute the three changes top-to-bottom in one task. Start with `AnyProvider::Kubernetes` dispatch (requires `KubernetesProvider::new()` which is `async`, so the `match` arm must `await`), then `print_execution_plan()` extension, then the dry-run test. Run `cargo test --workspace` at the end; expect all existing tests to remain green.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Provider dispatch | `AnyProvider` enum in `run.rs` with `RuntimeProvider` delegation impl | Already exists with `Docker` and `Compose` arms — just add `Kubernetes` arm; follow the exact same pattern |
| Dry-run section | `── Forge ──` / `── Compose Services ──` blocks in `print_execution_plan()` | Both show `if let Some(ref x) = manifest.x { println!(...) }` — apply identical guard for `manifest.kubernetes` |
| Test scaffolding | `dry_run.rs` with `smelt()` helper and `assert_cmd::Command` | All existing dry-run tests follow the same pattern; add one more |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs` — **primary file**. `AnyProvider` enum (lines 42–46), `RuntimeProvider` delegation impl (lines 48–100), Phase 3 `match` dispatch (lines 177–191), `print_execution_plan()` (lines 378–457). The `── Forge ──` block (lines 436–444) and `── Compose Services ──` block (lines 428–434) are the templates to follow for `── Kubernetes ──`.
- `crates/smelt-cli/tests/dry_run.rs` — `smelt()` helper, `assert_cmd::Command`, `predicate::str::contains` pattern for all existing dry-run tests. `test_dry_run_with_forge_shows_forge_section` (line ~115) is the exact model for the new kubernetes test.
- `crates/smelt-core/src/k8s.rs` — `KubernetesProvider::new(manifest: &JobManifest) -> crate::Result<Self>` is `async`; the Phase 3 match must `await` it. Note: unlike `DockerProvider::new()` (synchronous) and `ComposeProvider::new()` (synchronous), K8s construction is async.
- `examples/job-manifest-k8s.toml` — already valid, parses cleanly, passes `--dry-run` today with `Runtime: kubernetes` in the environment block but no `── Kubernetes ──` section. This is the test subject.

## Constraints

- **RPITIT / D084 firm** — `Box<dyn RuntimeProvider>` is impossible; adding the `Kubernetes` variant to `AnyProvider` and adding delegation arms is the only correct pattern.
- **`KubernetesProvider::new()` is async** — the Phase 3 match block in `run_with_cancellation()` already awaits provider construction for Docker/Compose via `DockerProvider::new()` (sync but wrapped in context); the K8s arm must explicitly `.await` `KubernetesProvider::new(&manifest)`.
- **Phase 8 kubernetes fetch block already exists** — S03 already added `if manifest.environment.runtime == "kubernetes" { git.fetch_ref(...) }` in Phase 8. S04 must NOT add it again. Only Phase 3 dispatch is missing.
- **D017 / `deny_unknown_fields`** — `KubernetesConfig` fields are already locked; no manifest schema changes in S04.
- **`cargo test --workspace` must remain green** — existing Docker and Compose tests must be unaffected; the `runtime = "kubernetes"` path is additive only.

## Common Pitfalls

- **Forgetting `await` on `KubernetesProvider::new()`** — `DockerProvider::new()` and `ComposeProvider::new()` are sync; someone reading the pattern may forget K8s is async. The compiler will catch it, but worth noting.
- **Showing the kubernetes section for non-kubernetes manifests** — the `── Kubernetes ──` section must be gated on `manifest.kubernetes.is_some()`, not on `manifest.environment.runtime == "kubernetes"`. In practice these always co-occur (validation enforces it), but guard on the struct presence as with `── Forge ──` and `── Compose Services ──`.
- **Adding Phase 8 fetch again** — S03 already placed `if manifest.environment.runtime == "kubernetes" { git.fetch_ref(...) }` in Phase 8. Adding it again would double-fetch, which is harmless but noisy. Confirm it's already there before adding anything.
- **`KubernetesProvider` requires a cluster at `new()` time** — `KubernetesProvider::new()` calls `kube::Client::try_default()` which reads `~/.kube/config`. In CI or dev environments without a cluster this will fail. The `--dry-run` path goes through `execute_dry_run()` not `run_with_cancellation()`, so it never calls `KubernetesProvider::new()` — the dry-run test is cluster-free. The live path will fail at Phase 3 when no cluster is available, which is the correct behavior.
- **`context` field display** — `KubernetesConfig.context` is `Option<String>`; display as the context name or `"ambient"` (from kubeconfig) when `None`.

## Open Risks

- None — S04 is low-risk by design (D097 from milestone context: `risk:low`). All dependencies (KubernetesProvider, Phase 8 fetch, manifest schema) are proven by S01–S03.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust CLI / clap | N/A | no skill needed — pure internal wiring |

## Implementation Checklist

### 1. `AnyProvider::Kubernetes` variant (in `run.rs`)

```rust
enum AnyProvider {
    Docker(smelt_core::DockerProvider),
    Compose(smelt_core::ComposeProvider),
    Kubernetes(smelt_core::KubernetesProvider),   // ← ADD
}
```

Add delegation arms to all 5 `RuntimeProvider` methods:
```rust
AnyProvider::Kubernetes(p) => p.provision(manifest).await,
// etc.
```

Update Phase 3 match:
```rust
"kubernetes" => AnyProvider::Kubernetes(
    smelt_core::KubernetesProvider::new(&manifest)
        .await
        .with_context(|| "failed to connect to Kubernetes cluster")?,
),
```

### 2. `── Kubernetes ──` section in `print_execution_plan()`

Place after `── Compose Services ──` and before `── Forge ──` (or after — ordering is cosmetic; match the section order in the milestone success criteria: namespace, context, resources):

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

### 3. Dry-run test in `dry_run.rs`

```rust
#[test]
fn dry_run_kubernetes_manifest_shows_kubernetes_section() {
    smelt()
        .args(["run", "examples/job-manifest-k8s.toml", "--dry-run"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("── Kubernetes ──")
                .and(predicate::str::contains("smelt"))       // namespace
                .and(predicate::str::contains("ambient").or(predicate::str::contains("Context:"))),
        );
}
```

## Sources

- `run.rs` source read directly — AnyProvider enum and print_execution_plan() patterns
- `dry_run.rs` source read directly — test scaffold patterns
- `k8s.rs` source read directly — KubernetesProvider::new() is async (confirmed)
- S03 Forward Intelligence — "Phase 8 kubernetes fetch block already exists in run.rs — S04 does NOT need to add it"
- M005-CONTEXT.md — S04 scope confirmed: AnyProvider variant, dry-run section, example manifest, zero regressions
