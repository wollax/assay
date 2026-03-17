# Kata State

**Active Milestone:** M002 — Real Assay Integration
**Active Slice:** S04 — Exit Code 2 + Result Collection Compatibility
**Active Task:** —
**Status:** S03 complete. All verification passed: `cargo test --workspace` green, streaming integration test passes with live chunk delivery, `exec()` silent. Next: S04 — Exit Code 2 + Result Collection Compatibility.
**Phase:** Executing

## Recent Decisions
- D043: Assay manifest translation (supersedes D029) — Option A: Smelt writes spec files + RunManifest
- D044: Direct writes for `.assay/` setup, never `assay init` — avoids AlreadyInitialized error and host repo side-effects
- D045: `.assay/` idempotency guard — check for config.toml before writing; mkdir -p always safe; spec files always overwrite
- D046: `exec_streaming()` alongside `exec()` — buffered exec retained for setup phases, streaming for assay run phase
- D047: Linux assay binary built via `docker run rust:alpine` with separate CARGO_TARGET_DIR; cached at `target/smelt-test-cache/assay-linux-aarch64`; skip if source/Docker unavailable
- D048: Large binary injection via `docker cp` subprocess — avoids base64 exec unreliability for 50–100 MB debug binaries
- D049: `exec_streaming()` callback bound `FnMut(&str) + Send + 'static`; test accumulators use `Arc<Mutex<Vec<String>>>`

## Progress (M002)
- S01: ✅ Fix AssayInvoker — Real Assay Contract (T01 ✅, T02 ✅)
- S02: ✅ Real Assay Binary + Production Wiring (T01 ✅, T02 ✅, T03 ✅)
- S03: ✅ Streaming Assay Output (T01 ✅, T02 ✅)
- S04: ⬜ Exit Code 2 + Result Collection Compatibility

## Blockers
- None

## Known Issues
- Linux assay binary must be built via `docker run rust:alpine` (macOS binary is Mach-O, incompatible with Alpine containers); cached at `target/smelt-test-cache/assay-linux-aarch64`
- `.assay/` directory may be written to the bind-mounted host repo during live runs; no `.gitignore` entry added yet
- `exec_streaming()` callback requires `FnMut + Send + 'static`; stack-local accumulators require `Arc<Mutex<T>>` — see D049

## Next Action
Begin S04: Exit Code 2 + Result Collection Compatibility. Map `assay run` exit code 2 → `JobPhase::GatesFailed` (or equivalent), emit distinct stderr message, exit process with code 2, and verify `ResultCollector` handles Assay's post-merge state correctly (unit test).
