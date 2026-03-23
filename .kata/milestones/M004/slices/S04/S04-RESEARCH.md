# S04: CLI Integration + Dry-Run — Research

**Date:** 2026-03-21
**Domain:** CLI dispatch, dry-run UX, example manifests
**Confidence:** HIGH

## Summary

S04 is the final wiring slice for M004. Three concrete deliverables: (1) dispatch to `ComposeProvider` in `run.rs` when `runtime = "compose"`, (2) extend `print_execution_plan()` with a `── Compose Services ──` section, and (3) add `examples/job-manifest-compose.toml`. The work is low-risk because `ComposeProvider` is fully implemented (S03), the manifest type is stable (S01), and the CLI patterns are well-established.

The only non-trivial design decision is **how to dispatch between `DockerProvider` and `ComposeProvider`** in `run_with_cancellation()`. `RuntimeProvider` uses RPITIT (`fn provision(..) -> impl Future`), which makes the trait non-object-safe — `Box<dyn RuntimeProvider>` cannot be used. The right solution is a local `enum AnyProvider` in `run.rs` that implements `RuntimeProvider` by delegating to the inner variant via `async fn` methods. This is clean, zero-overhead, and follows the existing RPITIT pattern (D019) without introducing `async_trait` or trait objects.

The dry-run side is straightforward: add a `── Compose Services ──` block to `print_execution_plan()` after the `── Environment ──` section when `runtime == "compose"`, listing each service name and image. A companion integration test in `dry_run.rs` verifies this with the new example manifest.

## Recommendation

Define `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` in `run.rs` with a full `RuntimeProvider` impl using `async fn` delegation. Replace the current Phase 3 "docker-only" guard with a match that constructs the right variant. Add the `── Compose Services ──` section to `print_execution_plan()`. Write `examples/job-manifest-compose.toml` using the same shape as `examples/job-manifest.toml`. Add dry-run integration tests.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|-----------------|------------|
| Provider dispatch without trait objects | `enum AnyProvider` delegating via `async fn` | RPITIT prevents `Box<dyn RuntimeProvider>`; enum delegation is idiomatic Rust for this pattern |
| Test harness for dry-run integration | `assert_cmd::Command` + `predicates` in `dry_run.rs` | All existing dry-run tests use this; reuse `smelt()` / `workspace_root()` helpers |
| Example manifest structure | `examples/job-manifest.toml` | Use as template for the compose variant; same schema, just swap runtime and add `[[services]]` |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs` — **Phase 3 (line ~91)** currently contains `if manifest.environment.runtime != "docker" { ... return Ok(1); }`. This is the exact block to replace with the match dispatch. Phase 4 constructs `DockerProvider::new()` — replace with `AnyProvider` construction.
- `crates/smelt-cli/src/commands/run.rs: print_execution_plan()` — currently prints `── Environment ──`, `── Credentials ──`, `── Sessions ──`, `── Merge ──`, `── Forge ──`. Insert `── Compose Services ──` after `── Environment ──` section.
- `smelt_core::compose::ComposeProvider::new()` — failable constructor, returns `crate::Result<Self>`. The error path should mirror the existing Docker "failed to connect to Docker daemon" handling: `.with_context(|| "failed to connect to Docker daemon")?`.
- `smelt_core::provider::RuntimeProvider` — RPITIT trait (D019). `exec_streaming` is generic over `F: FnMut(&str) + Send + 'static`. All five methods (`provision`, `exec`, `exec_streaming`, `collect`, `teardown`) must be covered in the enum impl.
- `crates/smelt-cli/tests/dry_run.rs` — integration test harness; `smelt()` helper builds the binary and runs from workspace root; `workspace_root()` resolves two levels above `CARGO_MANIFEST_DIR`. Follow the existing pattern for new tests.
- `crates/smelt-core/src/manifest.rs: ComposeService` — `name: String`, `image: String`, `extra: IndexMap<String, toml::Value>`. `JobManifest.services: Vec<ComposeService>` is already the stable type. Access in `print_execution_plan()` via `manifest.services.iter()`.
- `examples/job-manifest.toml` — canonical reference for example manifest structure and commenting style.

## Constraints

- **RPITIT blocks `Box<dyn RuntimeProvider>`** — D019 is firm. The `exec_streaming` method is generic (not object-safe), so trait objects are structurally impossible. The `enum AnyProvider` approach is the only clean solution.
- **Phase 3 check is intentionally transitional (D077)** — S01 added runtime validation to `validate()`, making the Phase 3 docker-only guard redundant. S04 replaces it with a match. The `_` arm should still return an error for defence-in-depth (though `validate()` will have already caught it).
- **`ComposeProvider` is already exported** — `pub use compose::ComposeProvider` in `lib.rs`; import as `smelt_core::ComposeProvider` in `run.rs`.
- **`#![deny(missing_docs)]` in smelt-core** — `AnyProvider` lives in `smelt-cli` (a binary), so this lint does not apply there. No doc comments required on the local enum.
- **D026 teardown guarantee** — the unconditional teardown block at the bottom of `run_with_cancellation` must call `provider.teardown()` regardless of outcome. `AnyProvider::teardown()` delegates correctly; no changes to teardown logic needed.
- **`run_with_cancellation` is `pub`** — it's tested directly in the integration test suite. Signature must remain `pub async fn run_with_cancellation<F>(args: &RunArgs, cancel: F) -> Result<i32>`.

## Common Pitfalls

- **Forgetting `collect()` in the `AnyProvider` impl** — `RuntimeProvider` has five methods including `collect()`. `ComposeProvider::collect()` is a no-op but must be present. Missing it will fail to compile.
- **Phase 3 guard still returning on "compose"** — currently Phase 3 returns `Ok(1)` for any non-docker runtime. If the match replaces Phase 3 but Phase 3 is not fully removed, a stale `if` could still block compose. Ensure the old `if manifest.environment.runtime != "docker"` block is completely replaced, not just augmented.
- **`exec_streaming` generic parameter in enum impl** — the match arms must forward `output_cb` by move (not borrow) because `F: FnMut + Send + 'static`. With `async fn` delegation, `output_cb` is moved into the arm that wins; Rust's exhaustive matching means only one arm executes. This compiles cleanly.
- **Dry-run path also needs services section** — `execute_dry_run()` calls `print_execution_plan()`. Once the function is updated, the dry-run path automatically benefits. No separate wiring needed. The existing `execute_dry_run()` also has no Phase 3 runtime check, so compose manifests already parse and validate; only the display is missing.
- **`examples/job-manifest-compose.toml` job.repo** — must be an absolute path for real runs (`smelt run`) but `"."` works for `--dry-run`. Use `"."` with the same comment as in `job-manifest.toml`.
- **Test: `── Compose Services ──` must appear in stdout, not stderr** — `print_execution_plan()` uses `println!` (stdout). `assert_cmd` predicates check stderr and stdout separately; use `.stdout(predicate::str::contains(...))`.

## Open Risks

- **`AnyProvider` compile complexity with RPITIT** — `async fn` delegation in an enum impl is idiomatic but not commonly seen in this codebase. If the compiler rejects it (unlikely given Rust 2024 edition), fallback is to box each future explicitly with `Box::pin(...)` in the impl. This would require changing the return type annotation but not the calling code.
- **Phase 3 removal regression** — existing `run_without_dry_run_attempts_docker` test in `dry_run.rs` verifies the docker path still works after S02/S03 changes. The same test will continue to pass since `"docker"` still dispatches to `DockerProvider`. But if the enum impl has a bug in the docker arm, this test catches it.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust CLI / Clap | — | none needed; existing patterns sufficient |

## Sources

- `crates/smelt-cli/src/commands/run.rs` — Phase 3 docker guard, Phase 4 provider construction, `print_execution_plan()` structure (direct read)
- `crates/smelt-core/src/provider.rs` — RPITIT trait definition, all five method signatures (direct read)
- `crates/smelt-core/src/compose.rs` — `ComposeProvider::new()` signature, confirmed failable constructor (direct read)
- `crates/smelt-cli/tests/dry_run.rs` — test harness pattern, `smelt()` / `workspace_root()` helpers (direct read)
- S03-SUMMARY.md — `ComposeProvider::new()` is failable, compose project name is `smelt-{job_name}`, label is `smelt.job=<job_name>` (preloaded context)
- D019, D023, D026, D075, D077 — RPITIT constraint, teardown guarantee, dispatch decision, Phase 3 guard status (decisions register)
