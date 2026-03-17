# Kata State

**Active Milestone:** None — M002 complete, awaiting M003 planning
**Status:** M002 fully closed — M002-SUMMARY.md written, PROJECT.md updated, all artifacts committed.
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

## Completed Milestones
- M001: ✅ Docker-First Infrastructure MVP (2026-03-17)
- M002: ✅ Real Assay Integration (2026-03-17)

## M002 Definition of Done — Status
- [x] AssayInvoker unit tests pass with correct `[[sessions]]` key, spec file format, no unknown fields
- [x] Integration test with real assay binary shows assay progressing past manifest/spec parse phase
- [x] execute_run() Phase 5.5 and Phase 6 use corrected AssayInvoker API
- [x] exec_streaming() exists on RuntimeProvider and DockerProvider; Phase 7 uses it for gate output
- [x] Exit code 2 from assay run distinguished from exit code 1 — "gate failures" vs "pipeline error"
- [x] run_without_dry_run_attempts_docker test failure resolved (S02)
- [x] D029 superseded by D043 in DECISIONS.md
- [ ] Manual UAT: real Docker + real assay binary + real Claude API key (operational proof, not automated)

## Known Issues
- Linux assay binary must be built via `docker run rust:alpine` (macOS binary is Mach-O, incompatible with Alpine containers); cached at `target/smelt-test-cache/assay-linux-aarch64`
- `.assay/` directory may be written to the bind-mounted host repo during live runs; no `.gitignore` entry added yet
- End-to-end operational proof requires manual UAT with a real Claude API key

## Next Action
Plan M003 (Docker Compose runtime / PR integration / multi-machine coordination — scope TBD). Consider: add `.assay/` to `.gitignore`; add manual UAT runbook.
