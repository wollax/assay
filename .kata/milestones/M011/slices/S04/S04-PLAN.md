# S04: SshSyncBackend and CLI/MCP factory wiring

**Goal:** Implement `SshSyncBackend` (all 7 `StateBackend` methods via `scp`/`ssh` shell commands, `CapabilitySet::all()`) and wire `backend_from_config()` into every `assay-cli` and `assay-mcp` construction site that currently hardcodes `LocalFsBackend::new(...)`.
**Demo:** `just ready` green with 1499+ tests; factory tests confirm all 4 `StateBackendConfig` variants dispatch to real backends; CLI/MCP construction sites use `backend_from_config()` with `manifest.state_backend`; a path with spaces in `remote_assay_dir` does not produce shell injection via `Command::arg()`.

## Must-Haves

- `crates/assay-backends/src/ssh.rs` exists behind `#[cfg(feature = "ssh")]`; implements all 7 `StateBackend` methods using `Command::arg()` chaining (never shell string interpolation) — D163
- `SshSyncBackend::capabilities()` returns `CapabilitySet::all()`
- `backend_from_config()` in `factory.rs`: `Ssh` arm replaced from `NoopBackend` stub → `SshSyncBackend::new(...)` behind `#[cfg(feature = "ssh")]`
- Contract tests in `crates/assay-backends/tests/ssh_backend.rs` use mock `scp`/`ssh` scripts with PATH override and `#[serial]` — cover all 7 methods, injection safety test (path with spaces), `CapabilitySet::all()` assertion
- `assay-backends = { workspace = true }` added to both `assay-cli/Cargo.toml` and `assay-mcp/Cargo.toml`
- All 3 `LocalFsBackend::new(...)` callsites in `run.rs` replaced with `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs), pipeline_config.assay_dir.clone())`
- All 3 `LocalFsBackend::new(...)` callsites in `server.rs` replaced with `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs), assay_dir.clone())`
- `use assay_core::state_backend::LocalFsBackend` removed from both files
- `just ready` green with 1499+ tests — zero regression
- D173–D174 decisions documented in `DECISIONS.md`

## Proof Level

- This slice proves: integration (contract tests with mock subprocess, CLI/MCP wiring compilation)
- Real runtime required: no (mock scp/ssh scripts for unit contracts; real SSH/SCP is UAT only)
- Human/UAT required: yes — real multi-machine SCP against a live SSH server is UAT only

## Verification

- `cargo test -p assay-backends --features ssh` — all `ssh_backend` contract tests pass
- `cargo test -p assay-cli --features orchestrate` — CLI tests pass (manifests with `state_backend: None` default to `LocalFsBackend` via factory; wiring is transparent)
- `cargo test -p assay-mcp` — MCP tests pass unchanged
- `just ready` — green with 1499+ tests

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` in factory when `ssh` feature is disabled at build time (existing pattern from Linear/GitHub arms); `tracing::debug!` in `ScpRunner` logging the constructed scp/ssh command args before each spawn (avoids logging user data content)
- Inspection surfaces: mock scp/ssh scripts capture invocation args to a temp file (`MOCK_SCP_ARGS_FILE` env var) — tests assert against that file to verify arg shapes; `cargo test -p assay-backends --features ssh -- --nocapture` shows tracing debug lines
- Failure visibility: `AssayError::io("scp push", path, e)` / `AssayError::io("ssh mkdir", path, e)` carry operation label + path for localization; non-zero scp/ssh exit produces `AssayError::io` with captured stderr
- Redaction constraints: scp args may include host/user values but not credentials (SSH key auth, no password in args)

## Integration Closure

- Upstream surfaces consumed: `assay_core::StateBackend`, `CapabilitySet`, `AssayError`; `assay_types::StateBackendConfig::Ssh { .. }`; `assay_backends::factory::backend_from_config` stub (S01); `assay_types::manifest::RunManifest.state_backend`
- New wiring introduced in this slice: `assay-backends` dep in `assay-cli` + `assay-mcp`; `backend_from_config()` call at all 6 OrchestratorConfig construction sites; `#[cfg(feature = "ssh")]` `ssh` module in `assay-backends/src/lib.rs`; live `Ssh` arm in `factory.rs`
- What remains before the milestone is truly usable end-to-end: UAT with a real SSH server + scp access; LinearBackend and GitHubBackend wiring (already complete in S02/S03) end-to-end UAT; D160–D165 decisions already documented in prior slices; D173–D174 new decisions for this slice

## Tasks

- [x] **T01: Write contract tests for SshSyncBackend (red state)** `est:45m`
  - Why: test-first discipline (D079 pattern) — tests define the contract before implementation, catching any API mismatch immediately at compile time when T02 ships
  - Files: `crates/assay-backends/tests/ssh_backend.rs`
  - Do: Create `tests/ssh_backend.rs` with `#![cfg(feature = "ssh")]`. Write `write_mock_scp(dir, handlers)` helper (same pattern as `write_mock_gh` in `github_backend.rs`) that produces a shell script dispatching on the last two position-significant args (push: local→remote; pull: remote→local). Write `write_mock_ssh(dir, handlers)` helper for `ssh` commands (`mkdir -p`, `ls`, `rm`). Write `with_mock_path(dir, f)` helper that prepends `dir` to `PATH` (same pattern as `with_mock_gh_path`). Write 9 tests all annotated `#[serial]`: (1) `capabilities_returns_all()`, (2) `push_session_event_first_call_creates_remote_dir_and_pushes_state()`, (3) `push_session_event_second_call_pushes_updated_state()`, (4) `read_run_state_returns_deserialized_status()`, (5) `read_run_state_returns_none_when_file_missing()`, (6) `send_message_pushes_to_remote_inbox()`, (7) `poll_inbox_pulls_and_removes_remote_files()`, (8) `annotate_run_pushes_annotation_file()`, (9) `injection_safety_spaces_in_remote_path_do_not_cause_shell_split()`. Reference `assay_backends::ssh::SshSyncBackend` — compile will fail until T02. The mock scp script for pull tests must write predefined content to the destination path argument. The injection safety test uses `remote_assay_dir = "/remote/assay dir with spaces"` and verifies `push_session_event` returns `Ok(())` without shell splitting the path.
  - Verify: `cargo test -p assay-backends --features ssh -- ssh_backend` fails to compile (expected — `SshSyncBackend` not yet implemented)
  - Done when: test file exists, compiles as much as it can (type errors only from missing SshSyncBackend), and every test's assertion logic is complete and correct

- [ ] **T02: Implement SshSyncBackend and wire into factory** `est:75m`
  - Why: closes the R078 requirement — `SshSyncBackend` implements all 7 `StateBackend` methods via `Command::arg()` chaining (D163); `CapabilitySet::all()` returned; factory dispatches `Ssh` variant to the real backend
  - Files: `crates/assay-backends/src/ssh.rs`, `crates/assay-backends/src/lib.rs`, `crates/assay-backends/src/factory.rs`
  - Do: Create `src/ssh.rs`. Define `ScpRunner { host: String, user: Option<String>, port: Option<u16> }` with methods: `fn remote_spec(&self, remote_path: &str) -> String` (builds `"user@host:path"` or `"host:path"` as a single string); `fn scp_push(&self, local: &Path, remote_path: &str) -> assay_core::Result<()>` (builds `Command::new("scp")` with `.arg("-P").arg(port_str)` when port is set — uppercase P — then `.arg(local).arg(self.remote_spec(remote_path))`; spawns, waits, fails on non-zero exit with stderr captured into `AssayError::io`); `fn scp_pull(&self, remote_path: &str, local: &Path) -> assay_core::Result<()>` (same pattern, args reversed); `fn ssh_run_cmd(&self, remote_cmd: &str) -> assay_core::Result<String>` (builds `Command::new("ssh")` with `.arg("-p").arg(port_str)` when port set — lowercase p — then `.arg(host_spec).arg(remote_cmd)`; returns stdout on success). Define `SshSyncBackend { runner: ScpRunner, remote_assay_dir: String, local_assay_dir: PathBuf }` with `pub fn new(host, remote_assay_dir, user, port, local_assay_dir)` constructor and private `fn to_remote_path(&self, local: &Path) -> String` (strips `local_assay_dir` prefix from local, joins with `remote_assay_dir`). Implement `StateBackend` for `SshSyncBackend`: `capabilities()` returns `CapabilitySet::all()`; `push_session_event` serializes status to JSON, writes to temp file, ssh mkdir -p remote run_dir, scp push temp→remote state.json, remove temp; `read_run_state` scp pull remote state.json → temp, deserialize, return; `send_message` writes contents to local temp, ssh mkdir -p remote inbox, scp push temp→remote inbox/name; `poll_inbox` ssh ls remote inbox (parse filenames), for each: scp pull → local bytes, ssh rm remote file; `annotate_run` writes manifest_path to temp, ssh mkdir -p remote run_dir, scp push temp→remote annotation.txt; `save_checkpoint_summary` serializes checkpoint to JSON temp, ssh mkdir -p remote checkpoints dir, scp push. In `src/lib.rs` add `#[cfg(feature = "ssh")] pub mod ssh;`. In `factory.rs` replace the `StateBackendConfig::Ssh { .. }` arm with `#[cfg(feature = "ssh")]` arm constructing `SshSyncBackend::new(host.clone(), remote_assay_dir.clone(), user.clone(), *port, assay_dir)` and `#[cfg(not(feature = "ssh"))]` warning arm.
  - Verify: `cargo test -p assay-backends --features ssh -- ssh_backend` — all 9 contract tests pass; `cargo test -p assay-backends` (no ssh feature) — no compile errors (Ssh arm falls to NoopBackend)
  - Done when: all 9 ssh_backend contract tests pass with `--features ssh`; factory dispatches Ssh to `SshSyncBackend` when feature enabled; no regression in existing tests

- [ ] **T03: Wire backend_from_config into CLI and MCP construction sites** `est:30m`
  - Why: closes the R079 CLI/MCP wiring requirement — manifest-dispatch callsites use `backend_from_config()` so users who specify `state_backend = { type = "ssh", ... }` in their manifest get the SSH backend automatically; no hardcoded `LocalFsBackend` at manifest-dispatch sites
  - Files: `crates/assay-cli/Cargo.toml`, `crates/assay-mcp/Cargo.toml`, `crates/assay-cli/src/commands/run.rs`, `crates/assay-mcp/src/server.rs`
  - Do: Add `assay-backends = { workspace = true }` to `[dependencies]` in both Cargo.toml files. In `run.rs`: remove `use assay_core::state_backend::LocalFsBackend;`; add `use assay_backends::factory::backend_from_config;` and verify `assay_types::StateBackendConfig` is accessible (it's already re-exported from assay_types, check existing imports); replace all 3 `Arc::new(LocalFsBackend::new(pipeline_config.assay_dir.clone()))` expressions with `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&assay_types::StateBackendConfig::LocalFs), pipeline_config.assay_dir.clone())`. In `server.rs`: remove `use assay_core::state_backend::LocalFsBackend;`; add `use assay_backends::factory::backend_from_config;`; replace all 3 `Arc::new(LocalFsBackend::new(assay_dir.clone()))` expressions with `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&assay_types::StateBackendConfig::LocalFs), assay_dir.clone())`. Run `cargo check -p assay-cli --features orchestrate` and `cargo check -p assay-mcp` to catch any import issues before running full tests.
  - Verify: `cargo test -p assay-cli --features orchestrate` passes; `cargo test -p assay-mcp` passes; `just ready` green with 1499+ tests; `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` returns no matches
  - Done when: zero `LocalFsBackend::new` references in CLI/MCP crates; `just ready` green; R078 + R079 fully validated

## Files Likely Touched

- `crates/assay-backends/tests/ssh_backend.rs` — new contract tests
- `crates/assay-backends/src/ssh.rs` — new SshSyncBackend implementation
- `crates/assay-backends/src/lib.rs` — add `pub mod ssh`
- `crates/assay-backends/src/factory.rs` — replace Ssh stub arm
- `crates/assay-cli/Cargo.toml` — add assay-backends dep
- `crates/assay-mcp/Cargo.toml` — add assay-backends dep
- `crates/assay-cli/src/commands/run.rs` — replace 3 LocalFsBackend callsites + update imports
- `crates/assay-mcp/src/server.rs` — replace 3 LocalFsBackend callsites + update imports
- `.kata/DECISIONS.md` — D173, D174
- `.kata/STATE.md` — progress update
