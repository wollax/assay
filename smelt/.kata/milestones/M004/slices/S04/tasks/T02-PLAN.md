---
estimated_steps: 7
estimated_files: 1
---

# T02: Wire `AnyProvider` dispatch in `run_with_cancellation`

**Slice:** S04 — CLI Integration + Dry-Run
**Milestone:** M004

## Description

Replace the Phase 3 docker-only guard in `run_with_cancellation()` with a `ComposeProvider`-aware `AnyProvider` enum dispatch. This is the final assembly step: `smelt run manifest.toml` with `runtime = "compose"` now routes to `ComposeProvider` instead of returning `Ok(1)` with "unsupported runtime".

The core challenge is RPITIT: `RuntimeProvider` uses `async fn` return types that are not object-safe, so `Box<dyn RuntimeProvider>` cannot be used. The solution is a local `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` with a `RuntimeProvider` impl that delegates all 5 methods via `async fn` match arms. The `exec_streaming` method is generic (`F: FnMut(&str) + Send + 'static`); each arm captures `output_cb` by move — Rust's exhaustive match ensures only one arm runs, so this compiles cleanly.

## Steps

1. At the top of `run_with_cancellation()` body (after the `use` imports at the top of the function), define the enum and its `RuntimeProvider` impl. Place the enum locally within the function or as a module-private type at the bottom of the file — either works since `run.rs` is a binary crate not subject to `#![deny(missing_docs)]`. Define:
   ```rust
   enum AnyProvider {
       Docker(DockerProvider),
       Compose(smelt_core::ComposeProvider),
   }
   ```

2. Implement `RuntimeProvider` for `AnyProvider`. Each method delegates via match. For `provision`:
   ```rust
   impl RuntimeProvider for AnyProvider {
       fn provision(&self, manifest: &JobManifest)
           -> impl std::future::Future<Output = crate::Result<ContainerId>> + Send
       {
           // can't use async fn directly in non-async trait impl block,
           // use Box::pin if the compiler rejects async fn here
           match self {
               AnyProvider::Docker(p) => Box::pin(p.provision(manifest)),
               AnyProvider::Compose(p) => Box::pin(p.provision(manifest)),
           }
       }
       // ... similarly for exec, exec_streaming, collect, teardown
   }
   ```
   Note: RPITIT in trait impls on an enum may require `Box::pin(async move { ... })` for the futures to have compatible types in match arms. If `async fn` in the impl block compiles (Rust 2024), use it. If the compiler complains about opaque type mismatches between match arms, fall back to:
   ```rust
   fn provision(&self, manifest: &JobManifest)
       -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<ContainerId>> + Send>>
   {
       match self {
           AnyProvider::Docker(p) => Box::pin(p.provision(manifest)),
           AnyProvider::Compose(p) => Box::pin(p.provision(manifest)),
       }
   }
   ```
   This fallback changes the return type annotation but not any calling code. For `exec_streaming`, `output_cb: F` is moved into the winning match arm — exhaustive match ensures only one arm executes, so the move is sound.

3. Replace Phase 3 (the existing `if manifest.environment.runtime != "docker" { ... return Ok(1); }` block) with:
   ```rust
   // Phase 3 + 4: Connect to runtime provider
   eprintln!("Provisioning container...");
   let provider: AnyProvider = match manifest.environment.runtime.as_str() {
       "docker" => AnyProvider::Docker(
           DockerProvider::new().with_context(|| "failed to connect to Docker daemon")?,
       ),
       "compose" => AnyProvider::Compose(
           smelt_core::ComposeProvider::new()
               .with_context(|| "failed to connect to Docker daemon")?,
       ),
       other => {
           eprintln!(
               "Error: unsupported runtime `{other}`. Supported: docker, compose."
           );
           return Ok(1);
       }
   };
   ```

4. Remove the now-superseded Phase 4 block that constructs `DockerProvider::new()` separately. After the match above, `provider` is already constructed. The `eprintln!("Provisioning container...")` line that was in Phase 4 can move to before the match (or stay inside the match arms — either is fine, but avoid double-printing).

5. Verify all references to `provider` in the rest of the function still compile. The method call signatures (`provider.provision(...)`, `provider.exec(...)`, `provider.exec_streaming(...)`, `provider.teardown(...)`) are identical via the trait — no other changes needed.

6. Run `cargo build -p smelt-cli` first to confirm the enum compiles. If the compiler rejects the `impl RuntimeProvider for AnyProvider` with RPITIT return types, apply the `Box::pin` fallback from Step 2 for any methods that cause type errors.

7. Run `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` — confirm zero failures across all 9 test suites. Specifically confirm:
   - `run_without_dry_run_attempts_docker` still passes (docker manifest → DockerProvider path)
   - All dry-run tests still pass (including the new compose tests from T01)
   - If Docker is available: run `cargo test -p smelt-cli --test compose_lifecycle` to confirm S03 integration tests still pass

## Must-Haves

- [ ] `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` defined and compiles in `run.rs`
- [ ] `RuntimeProvider` is implemented for `AnyProvider` covering all 5 methods: `provision`, `exec`, `exec_streaming`, `collect`, `teardown`
- [ ] Phase 3 docker-only guard is fully replaced — no `if manifest.environment.runtime != "docker"` block remains
- [ ] `runtime = "compose"` manifests no longer print "unsupported runtime" and instead construct `ComposeProvider`
- [ ] `runtime = "docker"` manifests still construct `DockerProvider` — existing behavior unchanged
- [ ] `_` arm returns `Ok(1)` with an explicit "unsupported runtime" error message (defence-in-depth per D077)
- [ ] `cargo test --workspace` passes with 0 failures

## Verification

- `cargo build -p smelt-cli` → exits 0 (no compile errors)
- `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` → all suites pass, 0 FAILED
- `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` → exits 0 (validate path unchanged)
- `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run` → exits 0, no regression
- Inspect `run_with_cancellation` for the absence of the old `if manifest.environment.runtime != "docker"` guard block
- (If Docker available) `cargo test -p smelt-cli --test compose_lifecycle` → 3 passed; `cargo test -p smelt-cli --test docker_lifecycle` → existing count passed

## Observability Impact

- Signals added/changed: eprintln dispatch path now says "failed to connect to Docker daemon" for both Docker and Compose provider failures — consistent error messaging; the `other` arm prints "unsupported runtime" for defence-in-depth
- How a future agent inspects this: `grep -n "AnyProvider\|Phase 3\|Phase 4" crates/smelt-cli/src/commands/run.rs` locates the dispatch block; `cargo build -p smelt-cli 2>&1` surfaces any compile errors in the enum impl
- Failure state exposed: `ComposeProvider::new()` failure shows "failed to connect to Docker daemon" — same error shape as DockerProvider, matching existing user expectations and test assertions

## Inputs

- `crates/smelt-cli/src/commands/run.rs` — current Phase 3 guard (line ~91), Phase 4 DockerProvider construction, existing `provider.provision/exec/teardown` call sites
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait: 5 method signatures including generic `exec_streaming<F>` (all must be covered)
- `crates/smelt-core/src/compose.rs` — `ComposeProvider::new()` signature: `pub fn new() -> crate::Result<Self>` (failable, same as DockerProvider)
- S03-SUMMARY.md Forward Intelligence: `ComposeProvider::new()` is failable; compose project name is `smelt-{job_name}`; label `smelt.job=<job_name>` is the stable identifier
- D019 (RPITIT) — if `impl Trait` return types in enum impl cause type mismatch, fall back to `Box::pin` explicitly
- D075 — dispatch contract: `"docker" → DockerProvider`, `"compose" → ComposeProvider`, `_` → error
- D077 — Phase 3 check superseded by match dispatch in S04

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — modified file:
  - `enum AnyProvider` and its `RuntimeProvider` impl (~40–60 lines of new code)
  - Phase 3 docker-only guard replaced by the `match manifest.environment.runtime.as_str()` block
  - Phase 4 separate `DockerProvider::new()` call removed
  - Net diff: +50 lines, -10 lines approximately
