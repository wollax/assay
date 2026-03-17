# Kata State

**Active Milestone:** M002 — Real Assay Integration
**Active Slice:** S04 ✅ COMPLETE
**Status:** M002 complete — all four slices delivered; all success criteria met; 112 smelt-core tests, 0 failures across workspace.
**Phase:** Idle (awaiting next milestone)

## Recent Decisions
- D043: Assay manifest translation (supersedes D029) — Option A: Smelt writes spec files + RunManifest
- D044: Direct writes for `.assay/` setup, never `assay init` — avoids AlreadyInitialized error and host repo side-effects
- D045: `.assay/` idempotency guard — check for config.toml before writing; mkdir -p always safe; spec files always overwrite
- D046: `exec_streaming()` alongside `exec()` — buffered exec retained for setup phases, streaming for assay run phase
- D047: Linux assay binary built via `docker run rust:alpine` with separate CARGO_TARGET_DIR; cached at `target/smelt-test-cache/assay-linux-aarch64`; skip if source/Docker unavailable
- D048: Large binary injection via `docker cp` subprocess — avoids base64 exec unreliability for 50–100 MB debug binaries
- D049: `exec_streaming()` callback bound `FnMut(&str) + Send + 'static`; test accumulators use `Arc<Mutex<Vec<String>>>`
- D050: Save `assay_exit` binding before branching in Phase 7; `Ok(assay_exit)` at closure end; explicit `Ok(2)` arm before generic `Ok(code)` in outcome match
- D051: `test_collect_after_merge_commit` using `git merge --no-ff` — explicit invariant proof for ResultCollector post-Assay state

## Progress (M002)
- S01: ✅ Fix AssayInvoker — Real Assay Contract (T01 ✅, T02 ✅)
- S02: ✅ Real Assay Binary + Production Wiring (T01 ✅, T02 ✅, T03 ✅)
- S03: ✅ Streaming Assay Output (T01 ✅, T02 ✅)
- S04: ✅ Exit Code 2 + Result Collection Compatibility (T01 ✅, T02 ✅)

## M002 Definition of Done — Status
- [x] AssayInvoker unit tests pass with correct `[[sessions]]` key, spec file format, no unknown fields
- [x] Integration test with real assay binary shows assay progressing past manifest/spec parse phase
- [x] execute_run() Phase 5.5 and Phase 6 use corrected AssayInvoker API
- [x] exec_streaming() exists on RuntimeProvider and DockerProvider; Phase 7 uses it for gate output
- [x] Exit code 2 from assay run distinguished from exit code 1 — "gate failures" vs "pipeline error"
- [x] run_without_dry_run_attempts_docker test failure resolved (S02)
- [x] D029 superseded by D043 in DECISIONS.md
- [ ] Manual UAT: real Docker + real assay binary + real Claude API key (operational proof, not automated)

## Blockers
- None

## Known Issues
- Linux assay binary must be built via `docker run rust:alpine` (macOS binary is Mach-O, incompatible with Alpine containers); cached at `target/smelt-test-cache/assay-linux-aarch64`
- `.assay/` directory may be written to the bind-mounted host repo during live runs; no `.gitignore` entry added yet

## Next Action
Manual UAT: run `smelt run` with a real manifest, real Docker, real assay binary, and real Claude API key to demonstrate the complete M002 pipeline end-to-end. Then plan M003.
