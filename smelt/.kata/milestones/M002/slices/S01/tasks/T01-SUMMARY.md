---
id: T01
parent: S01
milestone: M002
provides:
  - Four new serde types mirroring Assay's real schema (SmeltRunManifest, SmeltManifestSession, SmeltSpec, SmeltCriterion)
  - sanitize_session_name associated function
  - build_spec_toml, build_run_manifest_toml, build_ensure_specs_dir_command, build_write_assay_config_command methods
  - write_spec_file_to_container async method
  - Updated build_run_command with --base-branch flag
  - Renamed call site in run.rs
key_files:
  - crates/smelt-core/src/assay.rs
  - crates/smelt-cli/src/commands/run.rs
key_decisions:
  - Replaced deny_unknown_fields-less AssayManifest/AssaySession with four new structs all annotated with deny_unknown_fields
  - SmeltManifestSession.spec holds the sanitized name (file reference), not the inline task text
  - Spec files written to /workspace/.assay/specs/<sanitized_name>.toml; run manifest stays at /tmp/smelt-manifest.toml
patterns_established:
  - sanitize_session_name: replace non-[a-zA-Z0-9_-] with '-', collapse consecutive dashes, trim, fallback to "unnamed"
  - write_spec_file_to_container mirrors write_manifest_to_container pattern exactly (base64 encode + sh -c echo pipe)
  - build_write_assay_config_command uses "if [ ! -f ... ]" guard to avoid clobbering existing config
observability_surfaces:
  - info! in build_spec_toml: session_name, toml_bytes
  - info! in build_run_manifest_toml: session_count, toml_bytes; debug! for toml_content
  - info! in write_spec_file_to_container: container, spec_name, spec_path, encoded_bytes (before) and exit_code (after)
  - Failure in write_spec_file_to_container includes container id, spec name, stderr from failed exec
duration: ~15min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Rewrite AssayInvoker Types and Method Implementations

**Replaced broken AssayManifest/AssaySession serde types with four deny_unknown_fields structs matching Assay's real schema, added all eight new/updated methods, and renamed the call site in run.rs — cargo check --workspace exits 0.**

## What Happened

Read the existing `assay.rs` and identified the two broken serde types (`AssayManifest`, `AssaySession`) and the stale test module. Read `manifest.rs` to confirm `SessionDef` field names (`name`, `spec`, `harness`, `timeout`, `depends_on`) and `JobManifest.job.base_ref`.

Rewrote `assay.rs` completely:

1. **Four new serde types** all annotated `#[serde(deny_unknown_fields)]`:
   - `SmeltRunManifest { sessions: Vec<SmeltManifestSession> }` — plural `sessions` key
   - `SmeltManifestSession { spec, name: Option<String>, depends_on: Vec<String> }` — skip_serializing_if on optionals
   - `SmeltSpec { name, description, criteria: Vec<SmeltCriterion> }` — description skipped when empty
   - `SmeltCriterion { name, description, cmd: Option<String> }` — cmd skipped when None

2. **`sanitize_session_name`**: replaces non-`[a-zA-Z0-9_-]` chars with `-`, collapses consecutive dashes, trims leading/trailing dashes, falls back to `"unnamed"`.

3. **`build_spec_toml`**: builds a `SmeltSpec` with one `"harness"` criterion containing the session's harness command; logs session name and toml byte count.

4. **`build_run_manifest_toml`** (replaces old `build_manifest_toml`): maps sessions to `SmeltManifestSession` with sanitized spec references; logs session count and byte count.

5. **`build_ensure_specs_dir_command`**: returns `["mkdir", "-p", "/workspace/.assay/specs"]`.

6. **`build_write_assay_config_command`**: builds base64-encoded `sh -c` with `if [ ! -f ... ]` guard.

7. **`build_run_command`**: updated to append `"--base-branch", manifest.job.base_ref.clone()`.

8. **`write_spec_file_to_container`**: mirrors `write_manifest_to_container` — base64-encodes, execs `sh -c echo | base64 -d > /workspace/.assay/specs/<name>.toml`, checks exit_code, logs container/spec_name/spec_path.

9. **Deleted the old test module** (tests go in T02).

In `run.rs`: renamed the single `AssayInvoker::build_manifest_toml` call to `AssayInvoker::build_run_manifest_toml`.

## Verification

```
cargo check --workspace
→ Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.01s  (0 errors)

grep "build_run_manifest_toml\|build_manifest_toml" crates/smelt-cli/src/commands/run.rs
→ let toml_content = smelt_core::AssayInvoker::build_run_manifest_toml(&manifest);
```

All must-haves confirmed:
- ✅ Four new types with `deny_unknown_fields` and both Serialize/Deserialize
- ✅ `build_run_manifest_toml` exists; old `build_manifest_toml` is gone
- ✅ `build_spec_toml` uses `sanitize_session_name` for the `name` field
- ✅ `sanitize_session_name` handles slash, space, edge cases (empty → "unnamed")
- ✅ `build_run_command` includes `--base-branch <manifest.job.base_ref>`
- ✅ `write_spec_file_to_container` writes to `/workspace/.assay/specs/<sanitized_name>.toml`
- ✅ `build_write_assay_config_command` contains `if [ ! -f ... ]` guard
- ✅ `cargo check --workspace` exits 0

## Diagnostics

- `RUST_LOG=smelt_core=debug cargo test` will surface all info!/debug! logs including TOML content
- Failures in `write_spec_file_to_container` include container id, spec name, and stderr
- `toml::from_str` with `deny_unknown_fields` will surface field name in error message immediately

## Deviations

None — implementation matched the task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/assay.rs` — complete rewrite: four new serde types, eight methods, no test module
- `crates/smelt-cli/src/commands/run.rs` — renamed build_manifest_toml → build_run_manifest_toml call site
