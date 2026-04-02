---
id: T02
parent: S01
milestone: M002
provides:
  - 13 named unit tests in `assay.rs` covering all AssayInvoker contract invariants
  - Fixed `build_manifest_toml` → `build_run_manifest_toml` rename in `docker_lifecycle.rs` test file
key_files:
  - crates/smelt-core/src/assay.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Used `test_manifest()` helper that wraps sessions TOML in a valid full JobManifest header, matching T01's established pattern
patterns_established:
  - test_manifest(sessions_toml) helper produces a full JobManifest from inline TOML fragment
  - deny_unknown_fields roundtrip tests: serialize with toml::to_string_pretty, deserialize with toml::from_str::<SmeltRunManifest/SmeltSpec>
observability_surfaces:
  - none (tests exercise pure functions; no runtime signals added)
duration: short
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Write Contract Unit Tests and Verify Full Suite

**Added 13 unit tests to `assay.rs` asserting all AssayInvoker contract invariants; fixed stale `build_manifest_toml` call site in docker_lifecycle.rs; `cargo test --workspace` exits 0.**

## What Happened

Added a `#[cfg(test)] mod tests` block to `crates/smelt-core/src/assay.rs` with 13 named tests covering:

- `test_run_manifest_uses_sessions_key_plural` — parses generated TOML, asserts `sessions` key exists and `session` is absent
- `test_run_manifest_spec_is_sanitized_name_not_description` — asserts `sessions[0].spec == "unit-tests"` not the free-text description
- `test_run_manifest_no_unknown_fields` — roundtrip through `SmeltRunManifest` + raw check that `harness`/`timeout` are absent from sessions entries
- `test_spec_toml_structure` — asserts `name`, `description`, `criteria[0].cmd` are all correct
- `test_spec_toml_deny_unknown_fields_roundtrip` — roundtrip through `SmeltSpec`
- `test_sanitize_session_name` — table-driven, 8 cases covering slash, space, leading/trailing dash, empty, all-dashes fallback
- `test_build_run_command_includes_base_branch` — asserts `--base-branch main` present
- `test_build_run_command_includes_timeout` — two sessions with 300/900 timeout; asserts max (900) is used
- `test_build_ensure_specs_dir_command` — exact vec equality
- `test_build_write_assay_config_command` — `sh -c`, idempotency guard, `base64 -d`, config path
- `test_multi_session_depends_on_preserved` — three sessions; `depends_on` serialized for gamma, absent for alpha

During workspace compilation, discovered that `crates/smelt-cli/tests/docker_lifecycle.rs` still used the pre-T01 name `build_manifest_toml` (5 call sites). Updated all to `build_run_manifest_toml` with a sed replacement.

## Verification

```
cargo test -p smelt-core 2>&1 | tail -5
# test result: ok. 110 passed; 0 failed; 0 ignored
```

```
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["
# All "test result: ok." lines — no FAILED, no error[]
```

## Diagnostics

- `cargo test -p smelt-core -- --nocapture` prints tracing logs for each test including generated TOML content
- Test names directly identify which contract invariant is being asserted
- `toml::from_str` parse errors surface field names immediately via `deny_unknown_fields`

## Deviations

- Fixed stale `build_manifest_toml` call sites in `docker_lifecycle.rs` — this was a missed rename from T01 (T01 renamed the method but the integration test file was not updated). Fixed as part of making `cargo test --workspace` pass.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/assay.rs` — added `#[cfg(test)] mod tests` block with 13 unit tests
- `crates/smelt-cli/tests/docker_lifecycle.rs` — renamed 5 `build_manifest_toml` calls to `build_run_manifest_toml`
