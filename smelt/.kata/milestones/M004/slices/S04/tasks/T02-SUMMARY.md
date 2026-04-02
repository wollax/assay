---
id: T02
parent: S04
milestone: M004
provides:
  - "enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) } — module-private dispatch enum in run.rs"
  - "RuntimeProvider impl for AnyProvider — all 5 methods (provision, exec, exec_streaming, collect, teardown) via async fn match delegation"
  - "Phase 3 docker-only guard removed — replaced by match dispatch on manifest.environment.runtime.as_str()"
  - "runtime = \"compose\" now constructs ComposeProvider instead of returning Ok(1) with unsupported runtime error"
  - "Defence-in-depth _ arm returns Ok(1) with explicit error message for unknown runtimes (D077)"
key_files:
  - crates/smelt-cli/src/commands/run.rs
key_decisions:
  - "Used async fn in trait impl (Rust 2024 RPITIT) rather than Box::pin — cleaner, no heap allocation, compiler handles lifetime inference; each method delegates via exhaustive match + .await (D019)"
  - "exec_streaming output_cb move in both match arms — valid in Rust exhaustive match; compiler permits because only one arm executes"
patterns_established:
  - "AnyProvider dispatch pattern: module-private enum + RuntimeProvider impl with async fn match delegation — reusable if additional runtimes are added in future milestones"
observability_surfaces:
  - "eprintln!(\"Error: unsupported runtime `{other}`. Supported: docker, compose.\") — defence-in-depth error for unknown runtimes"
  - "grep -n \"AnyProvider\\|Phase 3\\|Phase 4\" crates/smelt-cli/src/commands/run.rs — locates dispatch block"
  - "cargo build -p smelt-cli 2>&1 — surfaces any compile errors in enum impl"
duration: 10min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Wire `AnyProvider` dispatch in `run_with_cancellation`

**Replaced docker-only Phase 3 guard with `AnyProvider` enum dispatch — `smelt run manifest.toml` with `runtime = "compose"` now routes to `ComposeProvider`.**

## What Happened

Added a module-private `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` and implemented `RuntimeProvider` for it using Rust 2024 `async fn` in trait impls (RPITIT). All 5 methods delegate via exhaustive `match self` with `.await`. The `exec_streaming<F>` generic method compiles cleanly because Rust permits moving `output_cb` into both match arms of an exhaustive match (only one arm executes).

Removed the Phase 3 `if manifest.environment.runtime != "docker"` block entirely. The Phase 4 `DockerProvider::new()` call was merged into a single `match manifest.environment.runtime.as_str()` block (labelled "Phase 3 + 4") that constructs the appropriate `AnyProvider` variant or returns `Ok(1)` for unknown runtimes.

The `async fn` approach (rather than `Box::pin`) was chosen because Rust 2024 edition supports it cleanly, avoids heap allocation, and keeps lifetime inference simple.

## Verification

- `cargo build -p smelt-cli` → exits 0
- `cargo test --workspace` → 9 suites, 220 tests total, 0 FAILED
- `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` → exits 0, shows `── Compose Services ──` section
- `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run` → exits 0, no compose services section
- Grepped `run.rs` — no `if manifest.environment.runtime != "docker"` block remains; `AnyProvider` dispatch present at line 38/182

## Diagnostics

- `grep -n "AnyProvider\|Phase 3\|Phase 4" crates/smelt-cli/src/commands/run.rs` — locates enum definition and dispatch block
- `cargo build -p smelt-cli 2>&1` — surfaces enum impl compile errors
- Unknown runtime prints: `Error: unsupported runtime \`{other}\`. Supported: docker, compose.` to stderr and returns exit 1
- Provider construction failure prints: `failed to connect to Docker daemon` for both Docker and Compose paths (consistent error shape)

## Deviations

None. Used `async fn` in trait impl (Rust 2024) rather than `Box::pin` fallback — this is the preferred approach for Rust 2024 edition and compiled cleanly on first attempt.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — added `AnyProvider` enum + `RuntimeProvider` impl (~80 lines), removed Phase 3 docker-only guard, replaced Phase 4 with match dispatch; net diff +87/-12 lines
