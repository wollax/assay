# Kata State

**Active Milestone:** M002 — Real Assay Integration
**Active Slice:** S02 — Real Assay Binary + Production Wiring
**Active Task:** S02 not yet started
**Status:** S01 complete. All 13 AssayInvoker contract unit tests pass; cargo test --workspace exits 0 (110 smelt-core tests, 0 failed). Ready for S02.
**Phase:** Implementation

## Recent Decisions
- D043: Assay manifest translation (supersedes D029) — Option A: Smelt writes spec files + RunManifest
- D044: Direct writes for `.assay/` setup, never `assay init` — avoids AlreadyInitialized error and host repo side-effects
- D045: `.assay/` idempotency guard — check for config.toml before writing; mkdir -p always safe; spec files always overwrite
- D046: `exec_streaming()` alongside `exec()` — buffered exec retained for setup phases, streaming for assay run phase

## Progress (M002)
- S01: ✅ Fix AssayInvoker — Real Assay Contract (T01 ✅, T02 ✅)
- S02: ⬜ Real Assay Binary + Production Wiring
- S03: ⬜ Streaming Assay Output
- S04: ⬜ Exit Code 2 + Result Collection Compatibility

## Blockers
- None

## Known Issues
- `run_without_dry_run_attempts_docker` in `dry_run.rs` is a pre-existing failing test — scheduled for fix in S02
- Phase 5.5 methods in `AssayInvoker` exist but are not yet wired into `execute_run()` — wiring is S02's job

## Next Action
Begin S02: Real Assay Binary + Production Wiring. Key tasks: wire Phase 5.5 into execute_run(), add test_real_assay_manifest_parsing integration test (D039 phase-chaining + D040 binary injection), fix run_without_dry_run_attempts_docker.
