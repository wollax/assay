---
estimated_steps: 5
estimated_files: 3
---

# T02: Create AssayInvoker with manifest translation and container file writing

**Slice:** S03 — Repo Mount & Assay Execution
**Milestone:** M001

## Description

Create the `assay.rs` module with `AssayInvoker` — the translation layer between Smelt's `JobManifest` and Assay's expected CLI contract. This includes: building an Assay-compatible TOML manifest string from Smelt sessions, writing that manifest into a running container via base64-encoded exec, and constructing the `assay run` CLI command. The module is pure logic + exec calls — wiring into the `run.rs` flow happens in T03.

## Steps

1. Create `crates/smelt-core/src/assay.rs` with `AssayInvoker` struct (no state — methods are associated functions or take provider/container refs). Define the Assay manifest TOML target format as serde `Serialize` structs: `AssayManifest { sessions: Vec<AssaySession> }`, `AssaySession { name, spec, depends_on, timeout }`. These are Smelt's view of what Assay expects — not importing Assay types directly (D002: no crate dependency).
2. Implement `build_manifest_toml(manifest: &JobManifest) -> String` — iterates `manifest.session`, maps each `SessionDef` to `AssaySession` (name → name, spec → spec, harness → harness, timeout → timeout, depends_on → depends_on), serializes to TOML via `toml::to_string_pretty()`.
3. Implement `write_manifest_to_container(provider: &DockerProvider, container: &ContainerId, toml_content: &str) -> Result<ExecHandle>` — base64-encodes the TOML content, builds exec command `["sh", "-c", "echo '<b64>' | base64 -d > /tmp/smelt-manifest.toml"]`, calls `provider.exec()`. Checks exit code and returns error on failure.
4. Implement `build_run_command(manifest: &JobManifest) -> Vec<String>` — constructs `["assay", "run", "/tmp/smelt-manifest.toml"]`. Adds `--timeout` from the first session's timeout as a job-level timeout (or max of all session timeouts). Returns the command vector.
5. Register module in `lib.rs`, add unit tests: `test_single_session_manifest` verifies TOML output structure, `test_multi_session_with_deps` verifies depends_on mapping, `test_special_chars_in_spec` verifies quotes/brackets survive serialization, `test_build_command` verifies command vector shape.

## Must-Haves

- [ ] `AssayManifest`/`AssaySession` serde structs serialize to valid TOML
- [ ] `build_manifest_toml()` correctly maps all `SessionDef` fields
- [ ] `write_manifest_to_container()` base64-encodes content and writes via exec
- [ ] `build_run_command()` produces correct `assay run` CLI invocation
- [ ] Unit tests cover single session, multi-session with deps, special characters

## Verification

- `cargo test -p smelt-core -- assay::tests` — all unit tests pass
- `cargo test --workspace` — no regressions

## Observability Impact

- Signals added/changed: `tracing::info` for manifest TOML generation (session count, total bytes), manifest write exec result, assay command construction
- How a future agent inspects this: unit tests verify exact TOML output; `tracing::debug` logs the generated TOML content for debugging
- Failure state exposed: `SmeltError::Provider` with "write_manifest" operation context if the file write exec fails; non-zero exit code from write command

## Inputs

- `crates/smelt-core/src/manifest.rs` — `JobManifest`, `SessionDef` structs (from S01)
- `crates/smelt-core/src/provider.rs` — `ContainerId`, `ExecHandle` types (from S01/S02)
- `crates/smelt-core/src/docker.rs` — `DockerProvider::exec()` with `working_dir` support (from T01)
- S03 research: base64-encode approach for writing files into container, Assay manifest schema details

## Expected Output

- `crates/smelt-core/src/assay.rs` — new module with `AssayInvoker`, manifest translation, file writing, command construction, unit tests
- `crates/smelt-core/src/lib.rs` — `pub mod assay` added, `AssayInvoker` re-exported
