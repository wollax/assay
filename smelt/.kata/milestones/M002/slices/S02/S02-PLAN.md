# S02: Real Assay Binary + Production Wiring

**Goal:** Wire Phase 5.5 assay setup into `execute_run()` and prove the generated files are accepted by a real `assay` binary via an integration test that asserts assay progresses past its manifest/spec parse phase without errors.
**Demo:** `cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture` passes: assay's stderr contains `"Manifest loaded:"` and does NOT contain `"No Assay project found"`, `"unknown field"`, `"ManifestParse"`, or `"ManifestValidation"`.

## Must-Haves

- Phase 5.5 is wired between Phase 5 (provision) and Phase 6 (write manifest) in `execute_run()`: assay config write → specs dir ensure → per-session spec file writes, each with error-path teardown
- `test_real_assay_manifest_parsing` passes with a real Linux assay binary, proving Smelt-generated TOML files are schema-compatible with `deny_unknown_fields` Assay types
- The Linux assay binary is built via `docker run rust:alpine` and cached at `target/smelt-test-cache/assay-linux-aarch64`; the test skips gracefully if assay source or Docker is unavailable
- Assay is injected into the container via `docker cp` and made executable via exec chmod
- `run_without_dry_run_attempts_docker` continues to pass after Phase 5.5 wiring

## Proof Level

- This slice proves: integration
- Real runtime required: yes (Docker daemon + real `assay` binary built from source)
- Human/UAT required: no (integration test provides automated proof)

## Verification

```bash
# T01 verification — Phase 5.5 wired and existing tests still pass
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# All "test result: ok." lines, no FAILED, no error[]

# T02 verification — Linux binary builder helper compiles + binary is cached after run
cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture
# Passes or skips; target/smelt-test-cache/assay-linux-aarch64 exists if Docker + assay source available

# T03 verification — real assay binary parses Smelt-generated files without errors
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture
# PASSES; stderr from assay contains "Manifest loaded:" and no schema error strings
```

## Observability / Diagnostics

- Runtime signals: Phase 5.5 emits `eprintln!` for each setup step ("Writing assay config...", "Writing spec: <name>..."); individual `write_spec_file_to_container` errors include container ID, spec name, and stderr from the failed exec (from S01's tracing instrumentation)
- Inspection surfaces: `RUST_LOG=smelt_core=debug cargo test ... -- --nocapture` shows full TOML content for each spec file and the run manifest; `docker exec <id> sh -c "ls -la /workspace/.assay/specs/ && cat /workspace/.assay/config.toml"` inspects in-container state mid-test
- Failure visibility: if `test_real_assay_manifest_parsing` fails, the test prints full assay stdout/stderr; schema errors from assay include the exact unknown field name; `"No Assay project found"` means Phase 5.5 config write was not called or failed
- Redaction constraints: none (no secrets in test containers; spec files contain only synthetic session data)

## Integration Closure

- Upstream surfaces consumed: all S01 `AssayInvoker` methods (`build_write_assay_config_command`, `build_ensure_specs_dir_command`, `write_spec_file_to_container`, `build_run_manifest_toml`, `write_manifest_to_container`, `build_run_command`, `sanitize_session_name`)
- New wiring introduced in this slice: Phase 5.5 block in `execute_run()` (run.rs); Linux assay binary builder + `docker cp` injection helper (docker_lifecycle.rs); `test_real_assay_manifest_parsing` integration test
- What remains before the milestone is truly usable end-to-end: S03 streaming exec (`exec_streaming()` on `RuntimeProvider`/`DockerProvider`; Phase 7 uses it for real-time assay output); S04 exit-code-2 distinction; manual UAT with real Claude API key and real manifest

## Tasks

- [x] **T01: Wire Phase 5.5 assay setup into `execute_run()`** `est:45m`
  - Why: Without Phase 5.5, `smelt run` never writes `.assay/config.toml` or spec files, so `assay run` always fails with "No Assay project found"; all S01 methods exist but are not yet called in the run pipeline
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/dry_run.rs`
  - Do: Between Phase 5 (provision) and Phase 6 (write manifest), add a Phase 5.5 block that: (1) execs `build_write_assay_config_command(&manifest.job.name)` — error triggers teardown + return Err; (2) execs `build_ensure_specs_dir_command()` — error triggers teardown; (3) loops over sessions calling `write_spec_file_to_container(&provider, &container, &sanitize_session_name(&s.name), &build_spec_toml(s))` for each — any error triggers teardown; emit `eprintln!` for "Writing assay config...", "Writing specs dir...", "Writing spec: <name>..." before each step; add clarifying comment to `run_without_dry_run_attempts_docker` in dry_run.rs noting Phase 5.5 steps run before assay
  - Verify: `cargo test --workspace` exits 0; confirm Phase 5.5 steps appear in output of `cargo test -p smelt-cli --test dry_run run_without_dry_run_attempts_docker -- --nocapture` (they execute in alpine:3 without error before assay 127)
  - Done when: `cargo test --workspace` exits 0 and `run.rs` contains Phase 5.5 block between Phase 5 and Phase 6 with all three steps and teardown error paths

- [x] **T02: Linux assay binary builder and container injection helper** `est:1h30m`
  - Why: The integration test needs a real `assay` binary that runs in a Linux aarch64 Alpine container; the macOS Mach-O binary at `assay/target/debug/assay` cannot run there; we need a reproducible build-and-cache mechanism that skips gracefully when unavailable; large binaries require `docker cp` injection, not base64 exec
  - Files: `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Add two helper functions at the top of `docker_lifecycle.rs`: (1) `build_linux_assay_binary() -> Option<PathBuf>` — detects assay source via `ASSAY_SOURCE_DIR` env var then `<workspace_root>/../../assay` sibling; returns `None` if not found; checks cache at `<workspace_root>/target/smelt-test-cache/assay-linux-aarch64` and returns it if it exists; otherwise runs `docker run --rm --platform linux/arm64 -v <assay_src>:/assay:ro -v <cache_dir>/build:/build -e CARGO_TARGET_DIR=/build -v $HOME/.cargo/registry:/usr/local/cargo/registry -w /assay rust:alpine sh -c "cargo build --bin assay 2>&1"` (`--bin assay` matches `[[bin]] name = "assay"` in assay-cli/Cargo.toml); after run, copies `/build/debug/assay` (from the CARGO_TARGET_DIR volume) to the cache path; returns `Some(cache_path)` or `None` on failure; (2) `inject_binary_to_container(container_id: &str, host_path: &Path, dest_path: &str) -> bool` — runs `docker cp <host_path> <container_id>:<dest_path>` via subprocess; returns `true` on success; also add a small smoke test `test_build_linux_assay_binary_caches` that calls the helper and either skips (None) or asserts the cache file exists and has non-zero size
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle test_build_linux_assay_binary_caches -- --nocapture` — if assay source + Docker available: passes and `ls -la target/smelt-test-cache/assay-linux-aarch64` shows the binary; if not available: test skips gracefully
  - Done when: `build_linux_assay_binary()` and `inject_binary_to_container()` are compiled and the smoke test passes or skips cleanly

- [x] **T03: Add `test_real_assay_manifest_parsing` integration test** `est:45m`
  - Why: S02's primary deliverable is integration proof that Smelt-generated TOML files are accepted by the real assay binary; the test exercises the complete Phase 5.5 + Phase 6 sequence and asserts assay progresses past its manifest/spec parse phase with no schema errors
  - Files: `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Add `test_real_assay_manifest_parsing` test that: (1) calls `docker_provider_or_skip()` and `build_linux_assay_binary()` — return early if either is None; (2) builds a test manifest with 2 sessions (e.g. "parse-test-alpha" / "parse-test-beta" with depends_on) pointing at a temp repo dir; (3) provisions container; (4) injects Linux assay binary via `inject_binary_to_container()` then execs chmod; (5) runs Phase 5.5 directly: exec `build_write_assay_config_command`, exec `build_ensure_specs_dir_command`, for each session exec `write_spec_file_to_container`; (6) runs Phase 6: `write_manifest_to_container`; (7) execs `build_run_command()` and captures stdout+stderr; (8) asserts: stderr contains `"Manifest loaded:"`, does NOT contain `"No Assay project found"`, `"unknown field"`, `"ManifestParse"`, `"ManifestValidation"`; (9) tears down container; print full assay stdout+stderr on assertion failure for diagnostics
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture` passes (or skips if Docker/assay source unavailable); assay stderr visible in output contains `"Manifest loaded: 2 session(s)"`
  - Done when: Test passes against a real Linux assay binary, proving Smelt-generated TOML is schema-compatible with assay's `deny_unknown_fields` types

## Files Likely Touched

- `crates/smelt-cli/src/commands/run.rs` — Phase 5.5 wiring
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Linux binary helper, injection helper, smoke test, integration test
- `crates/smelt-cli/tests/dry_run.rs` — clarifying comment on `run_without_dry_run_attempts_docker`
