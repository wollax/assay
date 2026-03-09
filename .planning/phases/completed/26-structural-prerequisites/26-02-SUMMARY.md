# Phase 26 Plan 02: Error Variant & TUI Import Summary

## Result

**Status:** Complete
**Tasks:** 2/2
**Duration:** ~4 minutes

## Tasks Completed

### Task 1: Add Json variant and constructor helpers to AssayError

- Added `Json` variant to `AssayError` enum (placed after `Io` variant)
- Added `AssayError::io()` and `AssayError::json()` ergonomic constructors as inherent methods
- Added 3 new tests: `json_error_display_includes_all_context`, `json_error_source_chain`, `io_constructor_matches_manual`
- All existing tests pass without modification

### Task 2: Update call sites and verify TUI import

- Updated `history/mod.rs` line ~124: `serde_json::to_string_pretty` error mapping now uses `AssayError::json()` instead of wrapping in `AssayError::Io` with `std::io::Error::other()`
- Updated `history/mod.rs` line ~191: `serde_json::from_str` error mapping now uses `AssayError::json()` instead of wrapping in `AssayError::Io` with `std::io::Error::new(InvalidData, ...)`
- Left `checkpoint/persistence.rs` line ~227 unchanged (correctly uses `CheckpointRead` variant — different error semantics)
- Added `use assay_core::AssayError;` to `assay-tui/src/main.rs` with `#[allow(unused_imports)]` to pass clippy `-D warnings`

## Deviations

- Added `#[allow(unused_imports)]` attribute on the TUI `AssayError` import. The plan specified a bare import, but `just lint` runs clippy with `-D warnings` which treats unused imports as errors. The allow attribute is necessary for the verification-only import to compile cleanly.

## Verification

- `cargo fmt --all -- --check`: Pass
- `cargo clippy --workspace --all-targets -- -D warnings`: Pass
- `cargo test --workspace`: Pass (all 329 core + 53 MCP + 48 types + 8 integration tests)
- `cargo deny check`: Pass
- `just ready`: Fails on pre-existing `check-plugin-version` step (plugin.json version 0.1.0 != workspace 0.2.0) — unrelated to this plan

## Commits

- `aed3355`: feat(26-02): add Json error variant and ergonomic constructors to AssayError
- `ac79b9c`: refactor(26-02): update history call sites to use Json variant and verify TUI import
