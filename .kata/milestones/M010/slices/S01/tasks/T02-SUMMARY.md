---
id: T02
parent: S01
milestone: M010
provides:
  - StateBackend trait (7 sync methods, object-safe) in assay_core::state_backend
  - CapabilitySet struct with all()/none() constructors, feature-gated behind orchestrate
  - LocalFsBackend skeleton implementing StateBackend with all-stub method bodies
  - _assert_object_safe compile guard proving Box<dyn StateBackend> works
  - 6 passing contract tests in crates/assay-core/tests/state_backend.rs
key_files:
  - crates/assay-core/src/state_backend.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/tests/state_backend.rs
key_decisions:
  - CapabilitySet derives Eq in addition to PartialEq (clippy derive_partial_eq_without_eq lint requirement)
  - _assert_object_safe uses #[allow(dead_code)] so the unused function doesn't trigger warnings
  - Module and re-exports both gated behind #[cfg(feature = "orchestrate")] consistent with assay_core::orchestrate
patterns_established:
  - Feature-gated module: #[cfg(feature = "orchestrate")] pub mod state_backend + pub use at lib root
  - Object-safety compile guard: private fn taking Box<dyn Trait> at module level
observability_surfaces:
  - backend.capabilities() returns CapabilitySet — all four bool fields inspectable at backend construction time
duration: 20min
verification_result: passed
completed_at: 2026-03-26T00:00:00Z
blocker_discovered: false
---

# T02: Define StateBackend trait, CapabilitySet, and LocalFsBackend skeleton in assay-core

**`StateBackend` trait (7 sync methods) + `CapabilitySet` flags struct + `LocalFsBackend` stub implementation added to `assay_core`, feature-gated behind `orchestrate`, with 6 passing contract tests and `just ready` green.**

## What Happened

Created `crates/assay-core/src/state_backend.rs` with the full API surface S02 will wire into the orchestrator:

- `CapabilitySet` struct with 4 bool fields and `all()`/`none()` constructors, derives `Debug, Clone, Copy, PartialEq, Eq`
- `StateBackend` trait with 7 sync methods all returning `crate::Result<_>`, bounded `Send + Sync` for async task sharing
- `_assert_object_safe(_: Box<dyn StateBackend>)` private function as a compile-time object-safety proof
- `LocalFsBackend { pub assay_dir: PathBuf }` implementing `StateBackend` with all-stub bodies (`Ok(())`, `Ok(None)`, `Ok(vec![])`)

Updated `crates/assay-core/src/lib.rs` to add `#[cfg(feature = "orchestrate")] pub mod state_backend` and re-export `CapabilitySet`, `LocalFsBackend`, `StateBackend` behind the same gate.

Created `crates/assay-core/tests/state_backend.rs` with 6 contract tests gated by `#![cfg(feature = "orchestrate")]`.

Two minor deviations from the plan required during `just ready`:
1. `CapabilitySet` derives `Eq` in addition to `PartialEq` (clippy lint)
2. `_assert_object_safe` gets `#[allow(dead_code)]` to suppress the unused function warning (plan noted this wasn't needed, but clippy proved otherwise)
3. `cargo fmt` reformatted `send_message` signatures to single-line form

## Verification

- `cargo test -p assay-core --features orchestrate --test state_backend` — 6/6 pass
- `cargo test --workspace` — 1473 total tests, 0 failures (≥1466 requirement met)
- `just ready` — fmt + lint + test + deny all green
- `grep "_assert_object_safe" crates/assay-core/src/state_backend.rs` — confirms compile guard present

## Diagnostics

`backend.capabilities()` returns a `CapabilitySet` with 4 bool fields. S02+ callers should log this at backend construction time to diagnose capability mismatches before attempting operations that the backend may not support. All trait methods return `crate::Result<AssayError>` so failures carry structured context from the existing error hierarchy.

## Deviations

- Plan said `#[allow(dead_code)]` not needed for `_assert_object_safe`; clippy required it to suppress `dead_code` warning — added the attribute.
- `CapabilitySet` needed `Eq` derived alongside `PartialEq` (clippy `derive_partial_eq_without_eq`).
- `cargo fmt` collapsed multi-line `send_message` signatures to single-line in both trait and impl.

## Known Issues

None. All stubs are intentional — real implementations land in S02.

## Files Created/Modified

- `crates/assay-core/src/state_backend.rs` — new: StateBackend trait, CapabilitySet, LocalFsBackend, object-safety guard
- `crates/assay-core/src/lib.rs` — added feature-gated pub mod + re-exports
- `crates/assay-core/tests/state_backend.rs` — new: 6 contract tests
