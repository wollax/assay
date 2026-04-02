---
estimated_steps: 13
estimated_files: 1
---

# T01: Rewrite AssayInvoker Types and Method Implementations

**Slice:** S01 — Fix AssayInvoker — Real Assay Contract
**Milestone:** M002

## Description

Replace the two broken serde types (`AssayManifest`, `AssaySession`) with four new ones that mirror Assay's real schema, and add/replace all eight `AssayInvoker` methods. No tests are written here — that's T02. The goal is a compilable `assay.rs` that exposes the correct API and generates correct TOML. Run `cargo check --workspace` to confirm no compile errors (including the renamed call site in `run.rs`).

## Steps

1. Open `crates/smelt-core/src/assay.rs`. Delete `AssayManifest` and `AssaySession` struct definitions entirely.
2. Define four new serde structs with `deny_unknown_fields` on each:
   - `SmeltRunManifest { sessions: Vec<SmeltManifestSession> }` — `Serialize`, `Deserialize`, `Debug`
   - `SmeltManifestSession { spec: String, name: Option<String>, depends_on: Vec<String> }` — optionals use `#[serde(default, skip_serializing_if = "Option::is_none")]`; `depends_on` uses `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
   - `SmeltSpec { name: String, description: String, criteria: Vec<SmeltCriterion> }` — `description` uses `#[serde(default, skip_serializing_if = "String::is_empty")]`
   - `SmeltCriterion { name: String, description: String, cmd: Option<String> }` — `cmd` uses `#[serde(default, skip_serializing_if = "Option::is_none")]`
3. Add `sanitize_session_name(name: &str) -> String` as a public associated function. Replace any character that is not `[a-zA-Z0-9_-]` with `-`. Collapse consecutive `-` into one. Trim leading and trailing `-`. If the result is empty, return `"unnamed"`.
4. Add `build_spec_toml(session: &SessionDef) -> String`. Build a `SmeltSpec`:
   - `name` = `Self::sanitize_session_name(&session.name)`
   - `description` = `session.spec.clone()`
   - `criteria` = one `SmeltCriterion` with `name = "harness"`, `description = format!("Harness gate for {}", session.name)`, `cmd = Some(session.harness.clone())`
   Serialize with `toml::to_string_pretty`. Add `info!` log with session name and toml byte count.
5. Add `build_run_manifest_toml(manifest: &JobManifest) -> String` (replaces the old `build_manifest_toml`). Build `SmeltRunManifest { sessions: manifest.session.iter().map(|s| SmeltManifestSession { spec: Self::sanitize_session_name(&s.name), name: Some(s.name.clone()), depends_on: s.depends_on.clone() }).collect() }`. Serialize with `toml::to_string_pretty`. Add `info!` log with session count and toml byte count. Keep the `debug!` log for TOML content.
6. Add `build_ensure_specs_dir_command() -> Vec<String>`. Returns `vec!["mkdir".to_string(), "-p".to_string(), "/workspace/.assay/specs".to_string()]`. No shell wrapper needed.
7. Add `build_write_assay_config_command(project_name: &str) -> Vec<String>`. Compose a minimal config TOML string: `format!("project_name = {:?}\n", project_name)`. Base64-encode it. Return `vec!["sh".to_string(), "-c".to_string(), format!("if [ ! -f /workspace/.assay/config.toml ]; then mkdir -p /workspace/.assay && echo '{}' | base64 -d > /workspace/.assay/config.toml; fi", encoded)]`.
8. Add `write_spec_file_to_container(provider, container, sanitized_name, toml_content) -> Result<ExecHandle>` (async). Mirror `write_manifest_to_container` exactly — base64-encode `toml_content`, exec `sh -c "echo '<b64>' | base64 -d > /workspace/.assay/specs/<sanitized_name>.toml"`. Error variant: `SmeltError::provider("write_spec_file", ...)`. Log `spec_name` and `spec_path`. Check `exit_code != 0` with same pattern.
9. Update `build_run_command(manifest)`. Keep max-timeout logic. After `CONTAINER_MANIFEST_PATH`, add `"--timeout".to_string(), max_timeout.to_string()`, then `"--base-branch".to_string(), manifest.job.base_ref.clone()`. Full output: `["assay", "run", "/tmp/smelt-manifest.toml", "--timeout", "<n>", "--base-branch", "<ref>"]`.
10. Delete the entire `#[cfg(test)] mod tests { ... }` block (tests go in T02).
11. Open `crates/smelt-cli/src/commands/run.rs`. Find the one call to `AssayInvoker::build_manifest_toml` in Phase 6. Rename it to `AssayInvoker::build_run_manifest_toml`. No other changes.
12. Run `cargo check --workspace`. Fix any type or import errors.
13. Confirm `cargo check --workspace` exits 0 — no errors.

## Must-Haves

- [ ] `SmeltRunManifest`, `SmeltManifestSession`, `SmeltSpec`, `SmeltCriterion` are defined with `deny_unknown_fields` and both `Serialize`/`Deserialize`
- [ ] `build_run_manifest_toml` exists; old `build_manifest_toml` is gone
- [ ] `build_spec_toml` exists and uses `sanitize_session_name` for the spec `name` field
- [ ] `sanitize_session_name` handles slash, space, and edge cases (empty → `"unnamed"`)
- [ ] `build_run_command` includes `--base-branch <manifest.job.base_ref>` in output
- [ ] `write_spec_file_to_container` writes to `/workspace/.assay/specs/<sanitized_name>.toml`
- [ ] `build_write_assay_config_command` contains `if [ ! -f ... ]` guard
- [ ] `cargo check --workspace` exits 0

## Verification

```bash
cd /Users/wollax/Git/personal/smelt
cargo check --workspace 2>&1 | tail -5
# Expected: "warning: ... Finished ... (0 errors)"
# No "error[E...]" lines
```

```bash
# Spot-check the renamed call site in run.rs
grep "build_run_manifest_toml\|build_manifest_toml" crates/smelt-cli/src/commands/run.rs
# Must show: build_run_manifest_toml (not build_manifest_toml)
```

## Observability Impact

- Signals added/changed: `info!` logs added to `build_spec_toml` (session name, toml byte count) and to `write_spec_file_to_container` (container id, spec name, path, encoded byte count)
- How a future agent inspects this: `RUST_LOG=smelt_core=debug cargo test` surfaces all log lines including TOML content in debug builds
- Failure state exposed: `write_spec_file_to_container` error message includes container id, spec name, and stderr from failed exec — matches pattern of `write_manifest_to_container`

## Inputs

- `crates/smelt-core/src/assay.rs` — current (broken) implementation; read to understand what to preserve (`write_manifest_to_container` is reusable as-is; `CONTAINER_MANIFEST_PATH` constant is unchanged)
- `crates/smelt-core/src/manifest.rs` — `SessionDef` field names (`name`, `spec`, `harness`, `timeout`, `depends_on`); `JobManifest.job.base_ref` for `--base-branch`
- `crates/smelt-cli/src/commands/run.rs` — single call site to rename

## Expected Output

- `crates/smelt-core/src/assay.rs` — four new serde types, eight updated/new methods, no test module
- `crates/smelt-cli/src/commands/run.rs` — one renamed call site
- `cargo check --workspace` exits 0
