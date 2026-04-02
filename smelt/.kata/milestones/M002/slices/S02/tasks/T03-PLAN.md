---
estimated_steps: 4
estimated_files: 1
---

# T03: Add `test_real_assay_manifest_parsing` integration test

**Slice:** S02 — Real Assay Binary + Production Wiring
**Milestone:** M002

## Description

This is S02's primary deliverable: an integration test that runs a real Linux `assay` binary inside a Docker container, invokes the complete Phase 5.5 + Phase 6 sequence, and asserts that assay progresses past its manifest/spec parse phase without any TOML schema errors. The test uses the D039 phase-chaining pattern (direct method calls, not `run_with_cancellation()`) and the T02 helpers for binary building and injection.

The test proves that:
- Smelt's generated TOML files pass assay's `deny_unknown_fields` validation
- The `[[sessions]]` key is correct (not `[[session]]`)
- Spec files written to `/workspace/.assay/specs/<name>.toml` are found by assay's spec loader
- The `.assay/config.toml` created by Phase 5.5 satisfies assay's project root detection

Assay will fail after parsing (no Claude API key / worktree setup), but the test only asserts on the parse-phase outcome. Exit code is expected to be non-zero; the test does NOT assert `exit_code == 0`.

## Steps

1. **Build test manifest**: Two sessions — `"parse-test-alpha"` (no depends_on) and `"parse-test-beta"` (depends_on `["parse-test-alpha"]`) — with a temp dir as the repo path. Use `JobManifest { job: JobMeta { name: "real-assay-parse", ... }, ... }` directly.

2. **Provision + inject binary**: Call `docker_provider_or_skip()` (return if None); call `build_linux_assay_binary()` (return if None, with `eprintln!("Skipping test_real_assay_manifest_parsing — assay Linux binary unavailable")`). Provision container. Call `inject_binary_to_container(container.as_str(), &binary_path, "/usr/local/bin/assay")` — assert `true`; then exec `["chmod", "+x", "/usr/local/bin/assay"]` via `provider.exec()` — assert `exit_code == 0`.

3. **Run Phase 5.5 directly**: Following the D039 phase-chaining pattern:
   - `provider.exec(&container, &AssayInvoker::build_write_assay_config_command("real-assay-parse")).await` — assert `exit_code == 0`
   - `provider.exec(&container, &AssayInvoker::build_ensure_specs_dir_command()).await` — assert `exit_code == 0`
   - For each session in the manifest: `AssayInvoker::write_spec_file_to_container(&provider, &container, &sanitized_name, &spec_toml).await` — assert `Ok(handle)` and `handle.exit_code == 0`
   - `AssayInvoker::write_manifest_to_container(&provider, &container, &toml).await` — assert `Ok`

4. **Exec `assay run` and assert parse success**: Call `provider.exec(&container, &AssayInvoker::build_run_command(&manifest)).await`, capturing the handle. Print `"assay stdout: {}", handle.stdout` and `"assay stderr: {}", handle.stderr` unconditionally (visible with `--nocapture` for diagnostics). Then assert:
   - `handle.stderr.contains("Manifest loaded:")` — assay progressed past parse phase
   - `!handle.stderr.contains("No Assay project found")` — config write succeeded
   - `!handle.stderr.contains("unknown field")` — no `deny_unknown_fields` violations
   - `!handle.stderr.contains("ManifestParse")` — no manifest TOML parse errors
   - `!handle.stderr.contains("ManifestValidation")` — no manifest validation errors
   - Do NOT assert `exit_code == 0` — assay will fail after parse phase without Claude key
   - Teardown container in all cases (wrap assertions in a block so teardown runs even on panic via standard Rust test drop semantics, or use explicit teardown + then re-assert)

## Must-Haves

- [ ] Test returns early (skips) when `docker_provider_or_skip()` or `build_linux_assay_binary()` returns `None` — never panics in CI without Docker
- [ ] Phase 5.5 steps in the test exactly mirror the Phase 5.5 order in `execute_run()`: config write → specs dir → per-session spec writes → run manifest write
- [ ] Assertion on `"Manifest loaded:"` in assay stderr is the primary success signal
- [ ] All four negative assertions present: no "No Assay project found", no "unknown field", no "ManifestParse", no "ManifestValidation"
- [ ] Container is torn down unconditionally (teardown in both pass and fail paths)
- [ ] Assay stdout and stderr are printed unconditionally so failures are diagnosable without re-running with extra flags

## Verification

```bash
# Primary: integration test passes (or skips gracefully if binary/Docker unavailable)
cargo test -p smelt-cli --test docker_lifecycle test_real_assay_manifest_parsing -- --nocapture

# With binary built (second run, cache hit) — verify assay reached parse phase
# Expected in --nocapture output:
#   assay stderr: ... Manifest loaded: 2 session(s) ...

# Full suite still green
cargo test --workspace 2>&1 | grep -E "^test result|FAILED"
```

## Observability Impact

- Signals added/changed: assay's full stdout and stderr printed unconditionally in test output; this is the primary diagnostic surface for parse-phase failures; each Phase 5.5 exec result is checked with `assert_eq!(exit_code, 0, ...)` that includes the exec's stderr, giving immediate context on Phase 5.5 setup failures
- How a future agent inspects this: run the test with `--nocapture`; assay stderr shows exactly what step it reached (e.g., "Manifest loaded: 2 session(s)" or "No Assay project found"); Docker exec results for Phase 5.5 steps print their stderr on assertion failure
- Failure state exposed: if `"Manifest loaded:"` is absent, the full assay stderr output (printed unconditionally) shows the exact error; Phase 5.5 exec assertion failures include the exec stderr with the container command's error message

## Inputs

- `crates/smelt-cli/tests/docker_lifecycle.rs` — T02's helpers (`build_linux_assay_binary`, `inject_binary_to_container`) are available in the same file
- `crates/smelt-cli/src/commands/run.rs` — T01's Phase 5.5 wiring serves as the canonical sequence to mirror in the test's direct phase-chaining
- Research: Assay startup sequence steps 1–8 — "Manifest loaded: N session(s)" appears at step 8, after TOML parse (step 6) and validation (step 7) — this is the correct assertion target
- Research: "Assay requires `.assay/` directory at project root" — Phase 5.5 config write command (`mkdir -p /workspace/.assay`) creates it; the idempotency guard (`if [ ! -f ... ]`) is safe for fresh containers
- Research (pitfall): "Assay exits non-zero past parse phase in test" — do NOT assert `exit_code == 0`; assay will fail at worktree/agent launch without Claude key

## Expected Output

- `crates/smelt-cli/tests/docker_lifecycle.rs` — `test_real_assay_manifest_parsing` test function proving the real assay binary accepts Smelt-generated TOML files without schema errors
- Integration test either PASSES (assay reaches "Manifest loaded: N session(s)") or SKIPS (Docker/binary unavailable) — never FAILS due to missing infrastructure
