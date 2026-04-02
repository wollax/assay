# S01: Fix AssayInvoker — Real Assay Contract — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01's proof goal is contract correctness at the TOML serialization level — no Docker, no real `assay` binary needed. Unit tests directly assert key names, field presence/absence, sanitization behavior, and `deny_unknown_fields` roundtrip safety. This is the full verification class defined in the slice plan.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Working directory: `smelt` repo root
- No Docker required

## Smoke Test

```bash
cargo test -p smelt-core 2>&1 | tail -5
# Expected: "test result: ok. 110 passed; 0 failed"
```

## Test Cases

### 1. Run all smelt-core unit tests

```bash
cargo test -p smelt-core 2>&1 | tail -10
```

**Expected:** `test result: ok. N passed; 0 failed; 0 ignored` — all assay contract tests appear in the output.

### 2. Verify sessions key is plural in generated RunManifest

```bash
cargo test -p smelt-core test_run_manifest_uses_sessions_key_plural -- --nocapture 2>&1
```

**Expected:** Test passes. Output includes generated TOML with `[[sessions]]` header (not `[[session]]`).

### 3. Verify spec field is a name reference, not inline description

```bash
cargo test -p smelt-core test_run_manifest_spec_is_sanitized_name_not_description -- --nocapture 2>&1
```

**Expected:** Test passes. The `spec` value in sessions TOML equals `"unit-tests"` (sanitized name), not the free-text description string.

### 4. Verify no harness/timeout fields in generated manifest

```bash
cargo test -p smelt-core test_run_manifest_no_unknown_fields -- --nocapture 2>&1
```

**Expected:** Test passes. Raw TOML contains no `harness =` or `timeout =` keys in any session entry.

### 5. Verify deny_unknown_fields roundtrip

```bash
cargo test -p smelt-core test_run_manifest_roundtrip_deny_unknown_fields -- --nocapture 2>&1
cargo test -p smelt-core test_spec_toml_deny_unknown_fields_roundtrip -- --nocapture 2>&1
```

**Expected:** Both pass. No `toml::from_str` parse errors — Smelt only emits fields its own schema defines.

### 6. Verify spec TOML structure

```bash
cargo test -p smelt-core test_spec_toml_structure -- --nocapture 2>&1
```

**Expected:** Test passes. Spec TOML contains `name`, `description`, and `[[criteria]]` array with a `cmd` matching the session harness.

### 7. Verify session name sanitization

```bash
cargo test -p smelt-core test_sanitize_session_name -- --nocapture 2>&1
```

**Expected:** Test passes. All 8 table-driven cases pass: slash→dash, space→dash, leading/trailing dashes trimmed, empty→"unnamed".

### 8. Verify --base-branch flag in run command

```bash
cargo test -p smelt-core test_build_run_command_includes_base_branch -- --nocapture 2>&1
```

**Expected:** Test passes. `--base-branch main` (or whatever `base_ref` is) present in the returned command vec.

### 9. Full workspace build

```bash
cargo build --workspace 2>&1 | tail -5
```

**Expected:** `Finished` with zero errors. `run.rs` compiles with the renamed `build_run_manifest_toml` call site.

## Edge Cases

### Empty session name falls back to "unnamed"

```bash
cargo test -p smelt-core test_sanitize_session_name -- --nocapture 2>&1 | grep "unnamed"
```

**Expected:** The all-dashes or empty-after-sanitization case maps to `"unnamed"`.

### Multi-session depends_on preserved

```bash
cargo test -p smelt-core test_multi_session_depends_on_preserved -- --nocapture 2>&1
```

**Expected:** Test passes. `depends_on` array serialized correctly for dependent session; absent (not `[]`) for independent sessions (due to `skip_serializing_if`).

### Idempotency guard in assay config command

```bash
cargo test -p smelt-core test_build_write_assay_config_command -- --nocapture 2>&1
```

**Expected:** Test passes. The `sh -c` string contains `if [ ! -f /workspace/.assay/config.toml ]` — never clobbers existing config.

## Failure Signals

- Any `FAILED` line in `cargo test -p smelt-core` output indicates a regression
- `error[E...]` in `cargo build --workspace` means a renamed call site was missed
- `toml::from_str` parse error mentioning a field name (e.g., `harness`, `timeout`, `session`) means a rogue field was reintroduced
- `[[session]]` (singular) appearing in generated TOML indicates the struct was renamed incorrectly

## Requirements Proved By This UAT

No `REQUIREMENTS.md` exists — operating in legacy compatibility mode.

This UAT proves the following M002 milestone definition-of-done items:
- ✅ `AssayInvoker` unit tests pass with correct `[[sessions]]` key, spec file format, and no unknown fields
- ✅ `execute_run()` Phase 5.5 methods (`build_spec_toml`, `write_spec_file_to_container`, `build_run_manifest_toml`) exist and are contract-correct (wiring into execute_run is S02)
- ✅ D029 superseded in `DECISIONS.md` by D043 (validated contract)

## Not Proven By This UAT

- Real `assay` binary accepting the generated TOML — requires a live container with a real binary (S02)
- Phase 5.5 wiring in `execute_run()` — the methods exist but aren't connected to the run pipeline yet (S02)
- Streaming gate output — deferred to S03
- Exit code 2 distinction — deferred to S04
- `run_without_dry_run_attempts_docker` pre-existing test failure — fix deferred to S02

## Notes for Tester

- All tests are pure unit tests — no Docker daemon required
- `RUST_LOG=smelt_core=debug cargo test -p smelt-core -- --nocapture` will print the full generated TOML for visual inspection
- The 13 new assay contract tests are all in `crates/smelt-core/src/assay.rs` in the `#[cfg(test)] mod tests` block
- `cargo test --workspace` runs the full suite including Docker integration tests — Docker daemon required for those; use `cargo test -p smelt-core` to skip Docker tests
