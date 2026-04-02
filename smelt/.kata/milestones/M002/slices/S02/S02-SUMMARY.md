---
id: S02
parent: M002
milestone: M002
provides:
  - Phase 5.5 assay setup block wired into execute_run() (config write, specs dir, per-session spec files)
  - build_linux_assay_binary() helper — builds Linux aarch64 assay ELF via docker run rust:alpine, cached at target/smelt-test-cache/assay-linux-aarch64
  - inject_binary_to_container() helper — large binary injection via docker cp subprocess
  - workspace_root() helper in docker_lifecycle.rs
  - test_build_linux_assay_binary_caches smoke test — verifies builder or skips gracefully
  - test_real_assay_manifest_parsing integration test — proves real Linux assay binary reaches "Manifest loaded: 2 session(s)" without schema errors
requires:
  - slice: S01
    provides: AssayInvoker static methods (build_write_assay_config_command, build_ensure_specs_dir_command, write_spec_file_to_container, build_run_manifest_toml, write_manifest_to_container, build_run_command, sanitize_session_name, build_spec_toml)
affects:
  - S03
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - crates/smelt-cli/tests/dry_run.rs
key_decisions:
  - D047: Linux assay binary build via docker run rust:alpine with separate CARGO_TARGET_DIR; cache-first pattern
  - D048: Large binary injection via docker cp subprocess (base64 exec unreliable for 50-100 MB binaries)
  - Phase 5.5 teardown pattern mirrors Phase 6 exactly (set_phase Failed → TearingDown → teardown → cleanup → return Err)
  - build_linux_assay_binary() reordered to cache-first: cache check before source detection (T03 fix)
patterns_established:
  - D039 phase-chaining in integration tests: call AssayInvoker static methods directly, mirroring execute_run() Phase 5.5 sequence
  - Teardown-before-assert: provider.teardown() called unconditionally before assertions so container is removed even on panic
  - Unconditional output: assay stdout/stderr printed before assertions for diagnosability without --nocapture reruns
  - Exec-then-check-exit-code for direct provider.exec calls: match Err + Ok(handle) if exit_code != 0 + Ok(_)
  - Per-session spec write loop handles only Err (write_spec_file_to_container internalizes exit_code checking)
observability_surfaces:
  - eprintln!("Writing assay config...") before config exec
  - eprintln!("Writing specs dir...") before mkdir exec
  - eprintln!("Writing spec: {spec_name}...") per session before write_spec_file_to_container
  - Phase 5.5 failures surface as SmeltError::Provider with container ID, spec name, and stderr from failed exec
  - test_real_assay_manifest_parsing prints full assay stdout/stderr unconditionally — no --nocapture required to diagnose
  - "Manifest loaded: N session(s)" is the parse-phase success signal in assay stderr
drill_down_paths:
  - .kata/milestones/M002/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S02/tasks/T03-SUMMARY.md
duration: ~1 hour 15 minutes total (T01: ~15m, T02: ~45m including Docker build, T03: ~15m)
verification_result: passed
completed_at: 2026-03-17
---

# S02: Real Assay Binary + Production Wiring

**Phase 5.5 is wired into `execute_run()` and a real Linux assay binary confirms Smelt-generated TOML files pass `deny_unknown_fields` validation — assay reaches "Manifest loaded: 2 session(s)" without schema errors.**

## What Happened

Three tasks shipped in sequence, each building on the last.

**T01 — Phase 5.5 wiring** inserted the assay setup block into `execute_run()` between Phase 5 (container provision) and Phase 6 (write assay run manifest). The block executes three steps: (1) write `.assay/config.toml` idempotently via `build_write_assay_config_command`; (2) ensure `.assay/specs/` exists via `build_ensure_specs_dir_command`; (3) loop over sessions writing per-session spec TOML files via `write_spec_file_to_container`. Each step uses the teardown-and-return-error pattern matching Phase 6. The existing `run_without_dry_run_attempts_docker` test passes unchanged — Phase 5.5 steps now execute in alpine:3 and all succeed before assay exits 127.

**T02 — Linux binary builder** added `build_linux_assay_binary()` and `inject_binary_to_container()` to `docker_lifecycle.rs`. The builder detects assay source via `ASSAY_SOURCE_DIR` env var or `../../assay` sibling, then runs `docker run --rm --platform linux/arm64 rust:alpine sh -c "apk add --no-cache musl-dev && cargo build --bin assay"` with a separate `CARGO_TARGET_DIR` volume. The cache-first check prevents rebuilds. One deviation: `apk add --no-cache musl-dev` was needed in the Docker command because `rust:alpine` lacks musl libc headers; the plan's Docker command omitted this. The smoke test `test_build_linux_assay_binary_caches` verifies the end-to-end path or skips gracefully. A `workspace_root()` helper was also added (missing from the file despite being listed as existing in the plan). The 130 MB ELF binary is cached at `target/smelt-test-cache/assay-linux-aarch64`.

**T03 — Integration test** added `test_real_assay_manifest_parsing` to `docker_lifecycle.rs`. The test builds a two-session manifest (`"parse-test-alpha"` / `"parse-test-beta"` with dependency), provisions an Alpine container, injects the cached Linux aarch64 binary via `inject_binary_to_container()`, runs Phase 5.5 + Phase 6 directly via AssayInvoker static methods (D039 phase-chaining), execs `assay run`, tears down unconditionally, then asserts five conditions. The primary success signal — `"Manifest loaded: 2 session(s)"` in assay stderr — confirms the real binary passed manifest + spec TOML parsing without `deny_unknown_fields` errors.

One fix made in T03: `build_linux_assay_binary()` was reordered to check the cache path before attempting source detection. The T02 implementation had the source check first, causing the test to skip when the assay source repo was absent but the cached binary existed.

## Verification

```bash
# Full workspace — all 7 test suites pass
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# → 7 × "test result: ok." — no FAILED, no error[]

# Smoke test — binary builder (graceful skip without ASSAY_SOURCE_DIR, or build if set)
cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture
# → test result: ok. 1 passed

# Integration test — real assay binary parses Smelt-generated TOML
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture
# → "Manifest loaded: 2 session(s)" in assay stderr
# → test result: ok. 1 passed
```

Assay progresses past manifest/spec parse phase, enters Phase 1 (session execution), and exits at Phase 2 (git checkout) due to no real git repo in the test container — expected and not asserted.

## Requirements Advanced

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode.

## Requirements Validated

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode.

## New Requirements Surfaced

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode.

## Requirements Invalidated or Re-scoped

No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode.

## Deviations

1. **`workspace_root()` was missing** from `docker_lifecycle.rs` despite being listed as "already exists" in the plan. Added as a free function.
2. **`apk add --no-cache musl-dev`** prepended to the Docker build command — `rust:alpine` doesn't ship musl libc dev headers; plan's command would have failed at link time.
3. **`build_linux_assay_binary()` cache-first reorder** — T02 implemented source-check first; T03 identified this caused unnecessary skips when cache existed; fixed in T03.

## Known Limitations

- Assay binary build requires ~5–15 min with Docker and the assay source repo; tests skip gracefully when unavailable, but CI without assay source will not run the integration test.
- The test container exits non-zero (git checkout failure at Phase 2 of assay's pipeline) — test only asserts parse-phase success, not full assay execution.
- `.assay/` directory may be written to the bind-mounted host repo during live runs; no `.gitignore` entry exists yet.

## Follow-ups

- S03: Add `exec_streaming()` to `RuntimeProvider`/`DockerProvider` and wire Phase 7 to emit assay output incrementally.
- S04: Distinguish exit code 2 (gate failures) from exit code 1 (pipeline error); verify `ResultCollector` against Assay's post-merge state.
- Consider adding `.assay/` to the repo `.gitignore` to prevent accidental commits of ephemeral Assay project state.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Phase 5.5 block inserted between Phase 5 and Phase 6 with config write, specs dir ensure, per-session spec file writes; each step has teardown-and-return-error path
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `workspace_root()`, `build_linux_assay_binary()`, `inject_binary_to_container()`, `test_build_linux_assay_binary_caches`, `test_real_assay_manifest_parsing`; reordered `build_linux_assay_binary()` to cache-first
- `crates/smelt-cli/tests/dry_run.rs` — clarifying comment on `run_without_dry_run_attempts_docker` documenting Phase 5.5 behavior

## Forward Intelligence

### What the next slice should know
- `exec_streaming()` needs a new method on the `RuntimeProvider` trait (D046). S03 adds `exec_streaming(container, command, stdout_cb)` alongside the existing `exec()` — buffered exec is retained for Phase 5.5 setup commands where full output is needed at once; streaming is only needed for Phase 7 (assay run).
- Phase 7 in `execute_run()` currently calls `provider.exec(&container, assay_run_cmd)` and waits for the full result. The streaming replacement needs to handle the `ExecHandle` return semantics — consider whether `exec_streaming()` returns an `ExecHandle` (with final exit code) after the stream completes or a separate streaming handle type.
- The test container in `test_real_assay_manifest_parsing` exits at Phase 2 (git checkout) — this is expected; do not add a git repo to fix it unless the goal is to test further phases.

### What's fragile
- `build_linux_assay_binary()` cache path is `target/smelt-test-cache/assay-linux-aarch64` — Cargo cleans this directory on `cargo clean`; the Docker build must re-run after a clean.
- The Docker build platform flag `--platform linux/arm64` hardcodes aarch64; if tests run on x86_64 hosts, this flag and the binary arch would need to change.
- `inject_binary_to_container()` uses a bare `docker cp` subprocess — if the Docker daemon is not on PATH or is accessed via a non-default socket, this fails silently (returns `false`).

### Authoritative diagnostics
- `"Manifest loaded: N session(s)"` in assay stderr = parse phase passed; look here first for integration test success/failure.
- `"unknown field"` in assay stderr = TOML schema mismatch; assay's `deny_unknown_fields` rejected a field — check the exact field name in the error message.
- `"No Assay project found"` = Phase 5.5 config write (step 5a) failed or was not reached — check container ID and stderr from the config_cmd assertion.
- `eprintln!("Writing assay config...")` etc. in `smelt run` output confirms Phase 5.5 was entered; absence means Phase 5 (provision) failed.

### What assumptions changed
- "assay source sibling detection works from workspace root" — the initial T02 implementation checked source first, but the correct behavior is cache-first (reordered in T03). If both source and cache are present, the cache is used and the source is not touched.
- "rust:alpine has musl-dev headers" — false; `apk add --no-cache musl-dev` is required before any Rust build in that image.
