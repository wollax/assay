---
id: T01
parent: S02
milestone: M002
provides:
  - Phase 5.5 assay setup block wired into execute_run() (config write, specs dir, per-session spec files)
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
key_decisions:
  - Phase 5.5 teardown pattern mirrors Phase 6 exactly (set_phase Failed → TearingDown → teardown → cleanup → return Err)
  - write_spec_file_to_container already handles exit_code != 0 internally; only Err() check needed in the loop
  - provider.exec is called directly for config write and specs dir (not wrapped in a helper), matching the plan's model
patterns_established:
  - Exec-then-check-exit-code pattern for direct provider.exec calls: match on Err + Ok(handle) if exit_code != 0 + Ok(_)
  - Per-session loop uses write_spec_file_to_container which internalizes exit_code checking; outer code only handles Err
observability_surfaces:
  - eprintln!("Writing assay config...") before config exec
  - eprintln!("Writing specs dir...") before mkdir exec
  - eprintln!("Writing spec: {spec_name}...") per session before write_spec_file_to_container
  - SmeltError::Provider returned on any Phase 5.5 failure — includes container ID, spec name, and stderr from failed exec
duration: ~15 minutes
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Wire Phase 5.5 assay setup into `execute_run()`

**Inserted a Phase 5.5 block in `execute_run()` that writes the assay config, ensures the specs directory, and writes per-session spec TOML files into the container before invoking assay — eliminating the "No Assay project found" failure path.**

## What Happened

Added the Phase 5.5 block to `crates/smelt-cli/src/commands/run.rs` between Phase 5 (container provision) and Phase 6 (write assay run manifest). The three operations are:

1. **Assay config write** — calls `AssayInvoker::build_write_assay_config_command(&manifest.job.name)` and execs it via `provider.exec`. Uses `match` to handle both `Err` and `Ok(handle) if handle.exit_code != 0` with the standard teardown-and-return-error path.

2. **Specs dir creation** — calls `AssayInvoker::build_ensure_specs_dir_command()` and execs it the same way.

3. **Per-session spec file writes** — loops over `manifest.session.iter()`, derives `spec_name` via `sanitize_session_name` and `spec_toml` via `build_spec_toml`, then awaits `write_spec_file_to_container`. Since that function already checks `exit_code != 0` internally, only `Err` is handled in the loop.

Also added a clarifying comment to `run_without_dry_run_attempts_docker` in `dry_run.rs` documenting that Phase 5.5 steps now execute in alpine:3 before assay is invoked (all succeed; assay exits 127 which is still non-zero; test behavior unchanged).

## Verification

```
# Full workspace test suite — all 7 test suites pass, 0 FAILED
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# → 7 × "test result: ok." lines, no FAILED, no error[

# dry_run integration test still passes
cargo test -p smelt-cli --test dry_run run_without_dry_run_attempts_docker -- --nocapture
# → test run_without_dry_run_attempts_docker ... ok

# Structural grep confirms Phase 5.5 is between Phase 5 and Phase 6
grep -n "Phase 5\|Phase 5.5\|Phase 6\|assay config\|specs dir\|write_spec_file" \
  crates/smelt-cli/src/commands/run.rs
# → lines 103(Phase 5), 115(Phase 5.5), 118(assay config), 147(specs dir),
#    180(write_spec_file), 199(Phase 6)
```

## Diagnostics

- Run `smelt run <manifest> 2>&1` and look for "Writing assay config...", "Writing specs dir...", "Writing spec: <name>..." — their presence confirms Phase 5.5 was entered; absence means Phase 5 (provision) failed before reaching it.
- Any Phase 5.5 failure surfaces as an `anyhow::Error` wrapping a `SmeltError::Provider` with operation `"write_spec_file"`, container ID, spec name, and stderr from the failed exec.
- `RUST_LOG=smelt_core=debug cargo test ... -- --nocapture` shows full TOML content for each spec file via the `debug!` tracing macros in `write_spec_file_to_container`.

## Deviations

None. Implementation follows the plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — Phase 5.5 block inserted between Phase 5 and Phase 6 with config write, specs dir, and per-session spec file writes; each with teardown-and-return-error on failure
- `crates/smelt-cli/tests/dry_run.rs` — Added clarifying comment to `run_without_dry_run_attempts_docker` documenting Phase 5.5 behavior and why test behavior is unchanged
