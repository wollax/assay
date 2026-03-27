# S01: GuardDaemon backend plumbing and contract tests — Research

**Date:** 2026-03-27

## Summary

This slice wires `Arc<dyn StateBackend>` into `GuardDaemon` so that `try_save_checkpoint` routes checkpoint writes through the backend when `capabilities().supports_checkpoints` is true, falling back to the existing direct `save_checkpoint()` call when false. The scope is well-bounded: one struct gains a field, one method gains a conditional branch, one public API gains a parameter, and all call sites (CLI + tests) are updated. Contract tests with a spy backend prove the routing.

The primary complexity is the **feature-gate boundary**: `StateBackend` lives behind `#[cfg(feature = "orchestrate")]` in assay-core, but the `guard` module is unconditionally compiled. All real callers (`assay-cli`, `assay-mcp`, `assay-backends`) enable `orchestrate`, so the pragmatic approach is to feature-gate the backend-aware code within the guard module itself. The `GuardDaemon` struct gets the backend field behind `#[cfg(feature = "orchestrate")]`, and `try_save_checkpoint` conditionally routes through it.

The second concern is **async context + sync trait call** (D176): `GuardDaemon::run()` is async (tokio), and `save_checkpoint_summary` is sync. This is accepted risk per the roadmap — checkpoint saves are infrequent and `NoopBackend`/`LocalFsBackend` are instant. No `spawn_blocking` needed.

## Recommendation

Use feature-gated conditional compilation within the guard module:

1. **`GuardDaemon` struct** — add `#[cfg(feature = "orchestrate")] backend: Arc<dyn StateBackend>` field
2. **`GuardDaemon::new()`** — add backend parameter behind `#[cfg(feature = "orchestrate")]`; provide a second `new()` signature without it for non-orchestrate builds (or use a builder/default pattern)
3. **`try_save_checkpoint()`** — when `orchestrate` feature is on and `backend.capabilities().supports_checkpoints` is true, call `backend.save_checkpoint_summary()`; otherwise fall through to existing `crate::checkpoint::save_checkpoint()` call
4. **`start_guard()`** — extend signature with `backend: Arc<dyn StateBackend>` behind `#[cfg(feature = "orchestrate")]`; use conditional compilation to provide both signatures
5. **CLI `handle_guard_start`** — pass `Arc::new(LocalFsBackend::new(assay.clone()))`
6. **Contract tests** — create a `SpyBackend` that records `save_checkpoint_summary` calls via `Arc<Mutex<Vec<TeamCheckpoint>>>`; assert call happens when `supports_checkpoints = true`, does NOT happen when false

The simpler alternative: since ALL callers of `start_guard` come from `assay-cli` which always enables `orchestrate`, we could just require the `orchestrate` feature for the entire guard module. But this would be a larger scope change and could break `assay-tui` or `assay-harness` if they ever need guard status checks. The conditional compilation within the module is safer.

**Simplest viable approach**: Since `assay-harness` and `assay-tui` use assay-core WITHOUT `orchestrate` but neither calls `start_guard` or constructs a `GuardDaemon`, the cleanest path is to provide two `GuardDaemon::new()` signatures (one with backend, one without) via `cfg`. The `start_guard()` public function similarly gets two signatures. Tests in `daemon.rs` that don't need the backend continue to use the non-orchestrate `new()`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Spy/mock backend for tests | `NoopBackend` pattern in `state_backend.rs` | Follow the same `impl StateBackend` pattern; add `Mutex<Vec<TeamCheckpoint>>` for recording |
| Atomic checkpoint writes | `LocalFsBackend::save_checkpoint_summary()` delegates to `crate::checkpoint::persistence::save_checkpoint()` | Already battle-tested; the backend routing just adds a conditional before it |
| Object safety for `Arc<dyn StateBackend>` | `_assert_object_safe` compile guard in `state_backend.rs` | Already proven; `Arc<dyn StateBackend>` is the established pattern (D156) |

## Existing Code and Patterns

- `crates/assay-core/src/guard/daemon.rs:20-48` — `GuardDaemon` struct and `new()` constructor. 6 fields currently. Backend field goes here.
- `crates/assay-core/src/guard/daemon.rs:310-324` — `try_save_checkpoint()` method. Calls `extract_team_state()` then `save_checkpoint()`. The routing logic wraps around the `save_checkpoint()` call, replacing it with `backend.save_checkpoint_summary()` when appropriate.
- `crates/assay-core/src/guard/mod.rs:16-29` — `start_guard()` public function. Constructs `GuardDaemon` and calls `run()`. Signature must gain `backend` parameter.
- `crates/assay-cli/src/commands/context.rs:594-660` — `handle_guard_start()` CLI handler. Creates tokio runtime and calls `start_guard()`. Must pass `Arc::new(LocalFsBackend::new(assay.clone()))`.
- `crates/assay-core/src/state_backend.rs:172-210` — `NoopBackend` implementation. Pattern to follow for `SpyBackend` test helper.
- `crates/assay-core/src/state_backend.rs:395-401` — `LocalFsBackend::save_checkpoint_summary()` delegates to `crate::checkpoint::persistence::save_checkpoint()`. This is what the fallback path already does.
- `crates/assay-core/src/guard/daemon.rs:353-358` — `make_daemon()` test helper. Must be updated for the new constructor signature.
- `crates/assay-core/src/guard/daemon.rs:363-373` — `guard_daemon_new_creates_valid_struct` test. Must be updated.
- `crates/assay-core/src/guard/mod.rs:22-27` — `GuardDaemon::new()` call inside `start_guard()`. Must pass backend through.

## Constraints

- **Feature gate**: `StateBackend`, `Arc<dyn StateBackend>`, `LocalFsBackend`, `CapabilitySet` are all behind `#[cfg(feature = "orchestrate")]`. Guard module code that references them must also be gated.
- **D001 (zero-trait convention)**: `StateBackend` is the sole exception (D149). No new traits.
- **D007 (sync core)**: `save_checkpoint_summary` is sync; calling from async context is safe per D176.
- **D156**: `Arc<dyn StateBackend>` is the established ownership model (not `Box`).
- **D167 (capability guard pattern)**: Read `capabilities().supports_checkpoints` once, use the bool in the conditional. Don't call `capabilities()` repeatedly.
- **D175**: GuardDaemon does NOT read RunManifest to determine backend; the caller decides.
- **D176**: sync call inside async is accepted; no `spawn_blocking`.
- **All 6 `try_save_checkpoint` call sites** must route through the backend: `guard-soft`, `guard-hard`, `guard-circuit-trip` (×2), `guard-shutdown`.

## Common Pitfalls

- **Forgetting `#[cfg(feature = "orchestrate")]` on imports** — Any `use crate::state_backend::*` inside `guard/daemon.rs` or `guard/mod.rs` must be feature-gated, or compilation will fail when building without `orchestrate` (e.g. `assay-harness`, `assay-tui`).
- **Breaking existing daemon tests** — There are 9 tests in `daemon.rs` that use `make_daemon()` or `GuardDaemon::new()` directly. If the constructor signature changes unconditionally, all break. Use `cfg` to keep the non-orchestrate signature working, OR update `make_daemon()` to pass `Arc::new(NoopBackend)` when the feature is on.
- **Capability check ordering** — Per D167, capture `supports_checkpoints` once at `GuardDaemon` construction or at the start of `try_save_checkpoint`, not inside each call. Since `try_save_checkpoint` is called from multiple places but always on the same daemon instance, capturing in the method is fine (it's not a hot loop).
- **`Arc<dyn StateBackend>` needs `Send + Sync`** — The trait is already object-safe (compile guard exists). But `GuardDaemon` is used in an async context (`run()` is async). `Arc<dyn StateBackend>` is `Send + Sync` as long as `StateBackend` is. Verify existing impls (`LocalFsBackend`, `NoopBackend`) are `Send + Sync` — they are (no `Rc`, no `Cell`).
- **Test isolation** — The spy backend must use `Arc<Mutex<Vec<TeamCheckpoint>>>` so the test can read recorded calls after the daemon method returns. `TeamCheckpoint` must derive `Clone` or the spy must store a serialized representation.

## Open Risks

- **`TeamCheckpoint` may not be `Clone`** — The spy backend needs to store a copy. If `TeamCheckpoint` is not `Clone`, the spy must serialize to JSON string. Need to verify. (Mitigation: `TeamCheckpoint` derives `Serialize`; spy can `serde_json::to_string()` and store strings.)
- **Two constructor signatures via `cfg` may confuse IDE** — rust-analyzer may show errors depending on which feature set it analyzes. This is a cosmetic issue; CI validates both.
- **Non-orchestrate build of assay-core may never be tested in CI** — Need to verify `just ready` exercises both feature flag states, or at least that `cargo check -p assay-core` (without features) passes. The existing `cargo test --workspace` should cover this since `assay-harness` depends on assay-core without `orchestrate`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | — | No specialized skill needed; standard trait/cfg patterns |

No external technologies or frameworks are involved in this slice — it's pure Rust plumbing within the existing codebase.

## Sources

- `crates/assay-core/src/guard/daemon.rs` — Primary modification target (GuardDaemon struct, try_save_checkpoint, constructor)
- `crates/assay-core/src/guard/mod.rs` — `start_guard()` public API
- `crates/assay-cli/src/commands/context.rs:594-660` — CLI call site
- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait, `NoopBackend`, `LocalFsBackend`
- M012-ROADMAP.md, M012-CONTEXT.md — Milestone spec and context
- D149, D156, D167, D175, D176 — Governing decisions
