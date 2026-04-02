# S01: Fix AssayInvoker — Real Assay Contract

**Goal:** Rewrite `AssayInvoker` so every method it exposes generates TOML that satisfies Assay's real `deny_unknown_fields` schema — proved by unit tests that assert key names, spec reference semantics, field presence/absence, session name sanitization, and `--base-branch` flag — all without Docker.

**Demo:** `cargo test -p smelt-core` shows green unit tests asserting `[[sessions]]` (not `[[session]]`), spec-name references (not inline descriptions), no unknown fields, flat `[[criteria]]` with `cmd`, session name sanitization, and `--base-branch` in the run command.

## Must-Haves

- `cargo test -p smelt-core` passes with zero failures
- Generated `RunManifest` TOML uses `[[sessions]]` key (plural) — not `[[session]]`
- `SmeltManifestSession.spec` is set to the sanitized session name (a file reference), not the free-text `SessionDef.spec` description
- Generated `RunManifest` TOML contains no `harness`, `timeout`, or any field absent from Assay's real `ManifestSession` schema
- Generated spec TOML contains `name`, `description`, and at least one `[[criteria]]` entry with `cmd`
- `sanitize_session_name` replaces `/`, spaces, and non-`[a-zA-Z0-9_-]` chars with `-` and trims leading/trailing `-`
- `build_run_command` includes `--base-branch <manifest.job.base_ref>`
- `write_spec_file_to_container` writes to `/workspace/.assay/specs/<sanitized-name>.toml`
- `build_write_assay_config_command` is idempotent: only writes if `/workspace/.assay/config.toml` doesn't exist
- `crates/smelt-cli/src/commands/run.rs` compiles (call site updated to `build_run_manifest_toml`)

## Proof Level

- This slice proves: **contract** — generated TOML round-trips through Smelt's own mirror types with zero unknown fields and correct key names
- Real runtime required: no — all verification is unit tests and `cargo check`
- Human/UAT required: no

## Verification

```bash
cargo test -p smelt-core 2>&1 | tail -20
# All tests pass; look for: "test result: ok. N passed; 0 failed"
```

Specific test assertions (written in T02):
- `test_run_manifest_uses_sessions_key_plural` — parses generated TOML and asserts `parsed["sessions"]` exists, `parsed.get("session")` is None
- `test_manifest_session_has_no_harness_or_timeout` — deserializes to `SmeltManifestSession`; confirms `deny_unknown_fields` rejects harness/timeout on roundtrip
- `test_spec_toml_has_criteria_with_cmd` — parses spec TOML, checks `[[criteria]]` array present, first entry has `cmd = <harness>`
- `test_sanitize_session_name` — table-driven test covering slash, space, leading dash, trailing dash, already-clean name
- `test_build_run_command_includes_base_branch` — asserts `--base-branch` and `manifest.job.base_ref` present in returned vec
- `test_spec_toml_description_from_session_spec` — asserts spec TOML `description` equals `SessionDef.spec` free-text

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on spec writes (container, spec name, path) — consistent with existing `write_manifest_to_container` logging
- Inspection surfaces: unit tests directly assert TOML string content and deserialized struct values — no runtime needed
- Failure visibility: `toml::from_str` parse errors surface immediately with field name in message; `deny_unknown_fields` rejects rogue fields at parse time
- Redaction constraints: no secrets in test payloads; harness commands in tests are safe shell strings

## Integration Closure

- Upstream surfaces consumed: `crates/smelt-core/src/manifest.rs` (`JobManifest`, `SessionDef` — read-only, no changes)
- New wiring introduced in this slice:
  - `run.rs` call site: `build_manifest_toml` → `build_run_manifest_toml` (compile-time fix; Phase 5.5 async wiring is S02)
  - `smelt_core` public re-export unchanged (`pub use assay::AssayInvoker`)
- What remains before the milestone is truly usable end-to-end:
  - S02: `execute_run()` Phase 5.5 wired (assay config write + specs dir + per-session spec writes + run manifest write)
  - S02: Integration test with real `assay` binary injected into container via D040 pattern
  - S03: `exec_streaming()` for real-time gate output
  - S04: Exit code 2 distinction and `ResultCollector` compatibility

## Tasks

- [x] **T01: Rewrite AssayInvoker types and method implementations** `est:1h`
  - Why: The four serde types and eight methods are the entire unit under test; all must exist before any test can be written
  - Files: `crates/smelt-core/src/assay.rs`
  - Do:
    1. Remove `AssayManifest` and `AssaySession` structs entirely
    2. Define `SmeltRunManifest { sessions: Vec<SmeltManifestSession> }` with `deny_unknown_fields` and `Serialize`/`Deserialize`
    3. Define `SmeltManifestSession { spec: String, name: Option<String>, depends_on: Vec<String> }` with `deny_unknown_fields`, `skip_serializing_if` on optionals/empties
    4. Define `SmeltSpec { name: String, description: String, criteria: Vec<SmeltCriterion> }` — `description` with `skip_serializing_if = "String::is_empty"` default, `deny_unknown_fields`
    5. Define `SmeltCriterion { name: String, description: String, cmd: Option<String> }` — `cmd` with `skip_serializing_if = "Option::is_none"`, `deny_unknown_fields`
    6. Add `sanitize_session_name(name: &str) -> String` — replace any char not in `[a-zA-Z0-9_-]` with `-`, collapse consecutive `-`, trim leading/trailing `-`; return `"unnamed"` if result is empty
    7. Add `build_spec_toml(session: &SessionDef) -> String` — builds `SmeltSpec` with `name = sanitize_session_name(&session.name)`, `description = session.spec.clone()`, `criteria = vec![SmeltCriterion { name: "harness".into(), description: format!("Harness gate for {}", session.name), cmd: Some(session.harness.clone()) }]`; serialize with `toml::to_string_pretty`
    8. Add `build_run_manifest_toml(manifest: &JobManifest) -> String` — builds `SmeltRunManifest { sessions: manifest.session.iter().map(|s| SmeltManifestSession { spec: sanitize_session_name(&s.name), name: Some(s.name.clone()), depends_on: s.depends_on.clone() }).collect() }`; serialize with `toml::to_string_pretty`
    9. Add `build_ensure_specs_dir_command() -> Vec<String>` — returns `["mkdir", "-p", "/workspace/.assay/specs"]` (no shell wrapper needed; mkdir -p is idempotent)
    10. Add `build_write_assay_config_command(project_name: &str) -> Vec<String>` — base64-encodes a minimal `config.toml` string (`project_name = "<name>"\n`), returns a `sh -c "if [ ! -f /workspace/.assay/config.toml ]; then mkdir -p /workspace/.assay && echo '<b64>' | base64 -d > /workspace/.assay/config.toml; fi"` command vec
    11. Add `write_spec_file_to_container(provider, container, sanitized_name, toml_content) -> Result<ExecHandle>` — mirrors `write_manifest_to_container` exactly but target path is `/workspace/.assay/specs/<sanitized_name>.toml`; log `spec_name` and `spec_path`
    12. Update `build_run_command` — add `"--base-branch".to_string(), manifest.job.base_ref.clone()` after the `--timeout` args; remove old method body, keep the max-timeout logic
    13. Delete the existing `#[cfg(test)] mod tests` block entirely (tests are written in T02)
  - Verify: `cargo check -p smelt-core` passes with zero errors; `cargo check -p smelt-cli` passes (run.rs will have compile error on old name — fix that too in step below)
  - Done when: `cargo check --workspace` is error-free

- [x] **T02: Write contract unit tests and fix run.rs call site** `est:45m`
  - Why: Tests are the verification artifact for this slice; the run.rs rename must compile before any CI check passes
  - Files: `crates/smelt-core/src/assay.rs` (test module), `crates/smelt-cli/src/commands/run.rs`
  - Do:
    1. In `run.rs`, rename `AssayInvoker::build_manifest_toml` → `AssayInvoker::build_run_manifest_toml` at the Phase 6 call site (the only call)
    2. In `assay.rs`, add a `#[cfg(test)] mod tests` block with a shared `test_manifest(sessions_toml)` helper (same pattern as original, produces a `JobManifest`)
    3. Write `test_run_manifest_uses_sessions_key_plural`: call `build_run_manifest_toml`, parse with `toml::Value`, assert `parsed["sessions"].as_array().is_some()` and `parsed.get("session").is_none()`
    4. Write `test_run_manifest_spec_is_name_not_description`: assert `sessions[0]["spec"].as_str()` equals `sanitize_session_name("unit-tests")` not the free-text description
    5. Write `test_run_manifest_no_harness_or_timeout_fields`: parse sessions array, assert `s.get("harness").is_none()` and `s.get("timeout").is_none()` for all sessions
    6. Write `test_run_manifest_roundtrip_deny_unknown_fields`: deserialize generated TOML back to `SmeltRunManifest` — must not panic (if deny_unknown_fields fires on a field Smelt emitted, the test catches the regression)
    7. Write `test_spec_toml_has_criteria_with_cmd`: call `build_spec_toml`, parse with `toml::Value`, assert `parsed["criteria"].as_array().is_some()`, first criterion has `"cmd"` key matching `session.harness`
    8. Write `test_spec_toml_description_from_session_spec`: assert spec TOML `description` equals the `SessionDef.spec` free-text
    9. Write `test_spec_toml_name_is_sanitized`: session name `"my/session"` → spec TOML `name = "my-session"`
    10. Write `test_sanitize_session_name` (table-driven): `"frontend"` → `"frontend"`, `"my/session"` → `"my-session"`, `"my session"` → `"my-session"`, `"  leading"` → `"leading"` (after trim of spaces then dashes), `"trailing-"` → `"trailing"`, `"a/b/c"` → `"a-b-c"`
    11. Write `test_build_run_command_includes_base_branch`: call `build_run_command(&manifest)`, collect as vec, find `"--base-branch"` index, assert next element equals `manifest.job.base_ref`
    12. Write `test_build_run_command_includes_timeout`: assert `"--timeout"` present and value equals max session timeout (regression guard)
    13. Write `test_build_ensure_specs_dir_command`: assert returned vec equals `["mkdir", "-p", "/workspace/.assay/specs"]`
    14. Write `test_build_write_assay_config_command`: assert returned command is `["sh", "-c", ...]`; assert the `sh -c` string contains `"if [ ! -f /workspace/.assay/config.toml ]"` guard
    15. Write `test_multi_session_manifest_depends_on_preserved`: multi-session manifest; verify `sessions[1]["depends_on"].as_array()` contains the expected name
    16. Run `cargo test -p smelt-core` — all tests must pass
  - Verify: `cargo test -p smelt-core 2>&1 | tail -5` shows `test result: ok. N passed; 0 failed`; `cargo build --workspace` succeeds
  - Done when: all 13+ new unit tests pass, zero compilation errors across workspace

## Files Likely Touched

- `crates/smelt-core/src/assay.rs` — primary target: types replaced, methods added, tests written
- `crates/smelt-cli/src/commands/run.rs` — one call site renamed (`build_manifest_toml` → `build_run_manifest_toml`)
