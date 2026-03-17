---
id: T03
parent: S02
milestone: M002
provides:
  - test_real_assay_manifest_parsing integration test in docker_lifecycle.rs — proves real Linux assay binary accepts Smelt-generated TOML files without schema errors; asserts "Manifest loaded: 2 session(s)" in assay stderr
key_files:
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - build_linux_assay_binary() was reordered to cache-first: cache path check now precedes source detection, so the cached binary at target/smelt-test-cache/assay-linux-aarch64 is used even when assay source repo is absent
patterns_established:
  - D039 phase-chaining pattern: call AssayInvoker static methods directly in test (config write → specs dir → per-session spec writes → manifest write → assay run), mirroring execute_run() Phase 5.5 sequence exactly
  - Teardown-before-assert pattern: provider.teardown() is called after capturing the assay exec handle but before running assertions, ensuring the container is removed even when an assertion panics
  - Unconditional output pattern: assay stdout and stderr printed via eprintln! before assertions so failures are diagnosable without --nocapture reruns
observability_surfaces:
  - Run test with --nocapture to see full assay stdout/stderr; "Manifest loaded: N session(s)" is the parse-phase success signal
  - Phase 5.5 step assertions include stderr in the failure message: assert_eq!(handle.exit_code, 0, "..., stderr: {}", handle.stderr)
  - If "No Assay project found" appears in assay stderr, the Phase 5.5 config write (5a) failed or was not reached
  - If "unknown field" appears, a TOML field in sessions or spec was rejected by assay's deny_unknown_fields
duration: ~15 minutes
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Add `test_real_assay_manifest_parsing` integration test

**Added `test_real_assay_manifest_parsing` — an integration test that runs the real Linux assay binary and asserts it reaches "Manifest loaded: 2 session(s)" without TOML schema errors.**

## What Happened

Added the `test_real_assay_manifest_parsing` test to `crates/smelt-cli/tests/docker_lifecycle.rs`. The test:

1. Builds a two-session `JobManifest` (`"parse-test-alpha"` with no deps, `"parse-test-beta"` depending on alpha) with a tempdir as repo path.
2. Skips gracefully if Docker is unavailable or `build_linux_assay_binary()` returns `None`.
3. Provisions an `alpine:3` container, injects the cached Linux aarch64 assay binary via `inject_binary_to_container()`, and makes it executable.
4. Runs Phase 5.5 exactly as `execute_run()` does: config write → specs dir → per-session spec writes → manifest write.
5. Executes `assay run` via `AssayInvoker::build_run_command()`, captures stdout/stderr, prints them unconditionally.
6. Tears down the container unconditionally, then asserts the five parse-phase conditions.

Also fixed `build_linux_assay_binary()` to check the cache path **before** attempting to locate the assay source repo. This matches the T02 summary's stated "cache-first pattern" but differed from the actual implementation, which caused the test to skip when the source repo was absent but a cached binary existed.

## Verification

```
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture
```

Output:
```
assay stderr: Loading manifest: /tmp/smelt-manifest.toml
Manifest loaded: 2 session(s)
Multi-session manifest detected (2 sessions) — using orchestrated execution
Phase 1: Executing sessions...
Phase 1 complete: 2 outcomes in 0.0s
Phase 2: Checking out base branch 'main'...
Error: git checkout failed: No such file or directory (os error 2)

test test_real_assay_manifest_parsing ... ok
```

Full suite:
```
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# All "test result: ok." — no FAILED, no error[]
```

## Diagnostics

- Run with `--nocapture` — assay's full stderr is printed unconditionally, showing exactly which phase it reached
- `"Manifest loaded: N session(s)"` in assay stderr = parse phase passed (primary success signal)
- `"No Assay project found"` = Phase 5.5 config write (step 5a) failed — check config_cmd assertion message for container stderr
- `"unknown field"` = TOML schema mismatch; assay's deny_unknown_fields rejected a field in sessions or spec
- `"ManifestParse"` / `"ManifestValidation"` = TOML parse or semantic validation error in the run manifest
- Assay exits non-zero at Phase 2 (git checkout) in the test environment — expected; test does NOT assert exit_code == 0

## Deviations

- **`build_linux_assay_binary()` reordered to cache-first**: The cache check was moved above the source directory detection. The T02 summary described this as the intended behavior, but the T02 implementation checked the source first. This fix was necessary so the test runs from the cached binary even when the assay source repo is not present in the default sibling location.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/tests/docker_lifecycle.rs` — Added `test_real_assay_manifest_parsing` test function; reordered `build_linux_assay_binary()` to cache-first
