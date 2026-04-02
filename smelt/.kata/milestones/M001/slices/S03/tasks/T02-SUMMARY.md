---
id: T02
parent: S03
milestone: M001
provides:
  - AssayInvoker with build_manifest_toml(), write_manifest_to_container(), build_run_command()
  - AssayManifest/AssaySession serde types for Assay TOML format
key_files:
  - crates/smelt-core/src/assay.rs
  - crates/smelt-core/src/lib.rs
key_decisions:
  - base64 crate (v0.22) added for encoding manifest content written into containers
  - AssaySession.depends_on skipped in TOML output when empty (skip_serializing_if)
  - Job-level --timeout uses max of all session timeouts
patterns_established:
  - AssayInvoker as stateless struct with associated functions — no instance state needed
  - Test helper test_manifest() builds minimal JobManifest from session TOML fragment
  - Round-trip test pattern: serialize to TOML, deserialize back, assert structural equality
observability_surfaces:
  - "tracing::info for manifest TOML generation (session count, bytes), manifest write result, command construction"
  - "tracing::debug logs full generated TOML content"
  - "SmeltError::Provider with operation write_manifest on exec failure or non-zero exit code"
duration: 12m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Create AssayInvoker with manifest translation and container file writing

**Added AssayInvoker with TOML manifest translation, base64 container file writing, and assay run command construction — verified with 6 unit tests.**

## What Happened

Created `crates/smelt-core/src/assay.rs` with the `AssayInvoker` struct — a stateless translation layer between Smelt's `JobManifest` and Assay's CLI contract.

Three core functions:
1. `build_manifest_toml()` — maps `SessionDef` fields to `AssaySession` serde structs and serializes to pretty-printed TOML. Empty `depends_on` arrays are omitted from output.
2. `write_manifest_to_container()` — base64-encodes the TOML string and writes it to `/tmp/smelt-manifest.toml` inside a running container via `provider.exec()`. Checks exit code and returns `SmeltError::Provider` on failure.
3. `build_run_command()` — constructs `["assay", "run", "/tmp/smelt-manifest.toml", "--timeout", "<max>"]` where the timeout is the maximum across all sessions.

Added `base64 = "0.22"` to workspace dependencies. Registered `pub mod assay` in lib.rs with `AssayInvoker` re-exported.

## Verification

- `cargo test -p smelt-core -- assay::tests` — 6 tests passed:
  - `test_single_session_manifest` — verifies TOML output structure and field mapping
  - `test_multi_session_with_deps` — verifies depends_on mapping across 3 sessions
  - `test_special_chars_in_spec` — verifies quotes, brackets, braces survive serialization
  - `test_build_command_single_session` — verifies command vector shape
  - `test_build_command_uses_max_timeout` — verifies max timeout selection across sessions
  - `test_manifest_toml_is_valid_toml` — full round-trip serialize/deserialize
- `cargo test --workspace` — 112 tests passed, zero failures, zero regressions

Slice-level verification status (intermediate task):
- ✅ `cargo test -p smelt-core -- assay::tests` — 6 passed
- ✅ `cargo test -p smelt-cli --test docker_lifecycle -- mount` — 3 passed (T01's tests)
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- assay` — not yet created (T03)
- ✅ `cargo test --workspace` — all green

## Diagnostics

- Unit tests verify exact TOML output structure by parsing back to `toml::Value`
- `tracing::debug` logs the full generated TOML content for runtime debugging
- `SmeltError::Provider { operation: "write_manifest", .. }` on container write failures includes exit code and stderr
- Non-zero exit code from the write command is explicitly checked and surfaced

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/assay.rs` — new: AssayInvoker, AssayManifest/AssaySession types, manifest translation, container writing, command construction, 6 unit tests
- `crates/smelt-core/src/lib.rs` — added `pub mod assay` and `pub use assay::AssayInvoker`
- `crates/smelt-core/Cargo.toml` — added `base64.workspace = true`
- `Cargo.toml` — added `base64 = "0.22"` to workspace dependencies
