# Kata State

**Active Milestone:** M002 — Real Assay Integration
**Active Slice:** none — roadmap written, ready to begin S01
**Status:** M002-ROADMAP.md written. 4 slices planned (S01–S04). Ready to implement.
**Phase:** Implementation

## Recent Decisions
- D043: Assay manifest translation (supersedes D029) — Option A: Smelt writes spec files + RunManifest
- D044: Direct writes for `.assay/` setup, never `assay init` — avoids AlreadyInitialized error and host repo side-effects
- D045: `.assay/` idempotency guard — check for config.toml before writing; mkdir -p always safe; spec files always overwrite
- D046: `exec_streaming()` alongside `exec()` — buffered exec retained for setup phases, streaming for assay run phase

## Progress (M002)
- S01: ⬜ Fix AssayInvoker — Real Assay Contract
- S02: ⬜ Real Assay Binary + Production Wiring
- S03: ⬜ Streaming Assay Output
- S04: ⬜ Exit Code 2 + Result Collection Compatibility

## Blockers
- None

## Known Issues
- `run_without_dry_run_attempts_docker` in `dry_run.rs` is a pre-existing failing test — scheduled for fix in S02

## Next Action
Start S01: rewrite `AssayInvoker` serde types and methods in `crates/smelt-core/src/assay.rs` to match real Assay schema. Replace `AssayManifest`/`AssaySession` with `SmeltRunManifest`/`SmeltManifestSession`/`SmeltSpec`/`SmeltCriterion`. Add `build_spec_toml()`, `write_spec_file_to_container()`, `build_ensure_specs_dir_command()`, `build_write_assay_config_command()`. Update `build_run_command()` to include `--base-branch`. Update all unit tests. Append D043 to DECISIONS.md.
