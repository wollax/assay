---
estimated_steps: 4
estimated_files: 2
---

# T01: Wire Phase 5.5 assay setup into `execute_run()`

**Slice:** S02 — Real Assay Binary + Production Wiring
**Milestone:** M002

## Description

All S01 `AssayInvoker` methods exist and are tested, but `execute_run()` in `run.rs` never calls them. This task adds a Phase 5.5 block between Phase 5 (provision) and Phase 6 (write manifest) that: writes the assay config, ensures the specs directory exists, and writes per-session spec TOML files into the container. Each step uses the same error-path pattern as Phase 6 (error → teardown → return Err). Without this wiring, `assay run` always fails with "No Assay project found" because `.assay/config.toml` is never created.

Also adds a clarifying comment to `run_without_dry_run_attempts_docker` in `dry_run.rs` to document that Phase 5.5 steps now execute before assay.

## Steps

1. **Add Phase 5.5 eprintln markers**: After the `eprintln!("Container provisioned: ...")` and before Phase 6 (`// Phase 6: Write assay manifest`), insert comment `// Phase 5.5: Assay setup — config, specs dir, per-session spec files`.

2. **Exec assay config write**: Add block that calls `smelt_core::AssayInvoker::build_write_assay_config_command(&manifest.job.name)`, execs it via `provider.exec(&container, &cmd).await`, and on error (both `Err` variant and `exit_code != 0`) sets monitor phase to Failed, tears down, and returns the error — same pattern as Phase 6's `write_result` block. Emit `eprintln!("Writing assay config...")` before the exec.

3. **Exec specs dir ensure**: Same pattern for `smelt_core::AssayInvoker::build_ensure_specs_dir_command()`. Emit `eprintln!("Writing specs dir...")` before exec.

4. **Per-session spec file writes**: Loop over `manifest.session.iter()`, building spec name via `smelt_core::AssayInvoker::sanitize_session_name(&s.name)` and TOML via `smelt_core::AssayInvoker::build_spec_toml(s)`, then awaiting `smelt_core::AssayInvoker::write_spec_file_to_container(&provider, &container, &spec_name, &spec_toml)`. On `Err`, same teardown pattern. Emit `eprintln!("Writing spec: {spec_name}...")` before each write.

5. **Update dry_run.rs comment**: In `run_without_dry_run_attempts_docker`, add a comment above the assertion explaining that Phase 5.5 steps (config write, specs dir, spec files) now execute before assay is invoked; all steps succeed in alpine:3; assay exits 127 (not found) which is still non-zero, so the test behavior is unchanged.

## Must-Haves

- [ ] Phase 5.5 block is positioned between Phase 5 and Phase 6 (after provision success, before manifest write)
- [ ] All three Phase 5.5 operations (config write, specs dir, per-session spec writes) are present in the correct order: config → specs_dir → spec files per session → then existing Phase 6
- [ ] Each exec result is checked: both `Err` and non-zero `exit_code` trigger the teardown-and-return-error path
- [ ] `cargo test --workspace` exits 0 — no regressions

## Verification

```bash
# Full test suite must still pass
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["

# Dry-run test still passes (Phase 5.5 steps execute in alpine:3 before assay 127)
cargo test -p smelt-cli --test dry_run run_without_dry_run_attempts_docker -- --nocapture

# Visually confirm Phase 5.5 block exists between Phase 5 and Phase 6 in run.rs
grep -n "Phase 5\|Phase 5.5\|Phase 6\|assay config\|specs dir\|write_spec_file" \
  crates/smelt-cli/src/commands/run.rs
```

## Observability Impact

- Signals added/changed: `eprintln!` emitted for "Writing assay config...", "Writing specs dir...", "Writing spec: <name>..." before each Phase 5.5 exec; failure stderr from failed execs surfaces via the existing `SmeltError::Provider` log (from `write_spec_file_to_container`'s error return)
- How a future agent inspects this: run `smelt run <manifest> 2>&1` and look for the Phase 5.5 eprintln messages; their presence confirms the block was entered; absence means Phase 5 (provision) failed before reaching Phase 5.5
- Failure state exposed: non-zero exit from any Phase 5.5 exec surfaces as an anyhow error returned from `execute_run()` with context message including container ID and spec name (from `write_spec_file_to_container`'s error text)

## Inputs

- `crates/smelt-cli/src/commands/run.rs` — current `execute_run()` with Phase 5 → Phase 6 gap; Phase 6's error-handling pattern is the model for Phase 5.5 blocks
- `crates/smelt-core/src/assay.rs` — all Phase 5.5 methods are available as public associated functions on `AssayInvoker`; `write_spec_file_to_container` is async and takes `&impl RuntimeProvider`
- S01 summary's Forward Intelligence: "Phase 5.5 sequence is: exec config write → exec specs dir → for each session: exec spec write → exec manifest write"

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — Phase 5.5 block added with correct ordering, eprintln markers, and error-path teardown for all three steps
- `crates/smelt-cli/tests/dry_run.rs` — clarifying comment in `run_without_dry_run_attempts_docker` documenting Phase 5.5 behavior
- `cargo test --workspace` exits 0
