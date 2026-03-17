# Kata State

**Active Milestone:** M002 — Real Assay Integration
**Active Slice:** S03 — Streaming Assay Output (next)
**Active Task:** None — S02 complete, ready for S03
**Status:** S02 complete. All three tasks done: Phase 5.5 wired into execute_run(), Linux assay binary builder/injector implemented, test_real_assay_manifest_parsing integration test passes — real assay binary reaches "Manifest loaded: 2 session(s)" without schema errors. All 7 test suites pass.
**Phase:** Executing

## Recent Decisions
- D043: Assay manifest translation (supersedes D029) — Option A: Smelt writes spec files + RunManifest
- D044: Direct writes for `.assay/` setup, never `assay init` — avoids AlreadyInitialized error and host repo side-effects
- D045: `.assay/` idempotency guard — check for config.toml before writing; mkdir -p always safe; spec files always overwrite
- D046: `exec_streaming()` alongside `exec()` — buffered exec retained for setup phases, streaming for assay run phase
- D047: Linux assay binary built via `docker run rust:alpine` with separate CARGO_TARGET_DIR; cached at `target/smelt-test-cache/assay-linux-aarch64`; skip if source/Docker unavailable
- D048: Large binary injection via `docker cp` subprocess — avoids base64 exec unreliability for 50–100 MB debug binaries

## Progress (M002)
- S01: ✅ Fix AssayInvoker — Real Assay Contract (T01 ✅, T02 ✅)
- S02: ✅ Real Assay Binary + Production Wiring (T01 ✅, T02 ✅, T03 ✅)
- S03: ⬜ Streaming Assay Output
- S04: ⬜ Exit Code 2 + Result Collection Compatibility

## Blockers
- None

## Known Issues
- Linux assay binary must be built via `docker run rust:alpine` (macOS binary is Mach-O, incompatible with Alpine containers); cached at `target/smelt-test-cache/assay-linux-aarch64`
- `.assay/` directory may be written to the bind-mounted host repo during live runs; no `.gitignore` entry added yet
- Assay output is buffered until exec completes (not streamed); S03 adds `exec_streaming()` to fix this

## Next Action
Begin S03: Streaming Assay Output — implement `exec_streaming()` on `RuntimeProvider`/`DockerProvider` using bollard's multiplexed log stream; wire Phase 7 of `execute_run()` to emit chunks to stderr as they arrive.
