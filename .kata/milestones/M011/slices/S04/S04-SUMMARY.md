---
id: S04
parent: M011
milestone: M011
provides:
  - SshSyncBackend implementing all 7 StateBackend methods via scp/ssh Command::arg() chaining
  - ScpRunner low-level subprocess wrapper with shell_quote() for remote command safety
  - CapabilitySet::all() returned — full capability surface via remote filesystem mirroring
  - backend_from_config() fully resolved: all 4 StateBackendConfig variants dispatch to real backends
  - All 6 CLI/MCP construction sites use backend_from_config() — no hardcoded LocalFsBackend::new()
requires:
  - slice: S01
    provides: StateBackendConfig::Ssh variant and backend_from_config() stub in factory.rs
  - slice: S02
    provides: factory pattern (LinearBackend dispatch) and reqwest::blocking precedent
  - slice: S03
    provides: Command subprocess testing pattern (mock PATH override + #[serial])
affects: []
key_files:
  - crates/assay-backends/tests/ssh_backend.rs
  - crates/assay-backends/src/ssh.rs
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-backends/Cargo.toml
  - crates/assay-cli/Cargo.toml
  - crates/assay-mcp/Cargo.toml
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - D163: SshSyncBackend uses Command::arg() chaining for scp — never shell string interpolation
  - D173: ssh_run() uses shell_quote() for remote directory operations (remote shell layer)
  - D174: read_run_state returns Ok(None) on scp pull failure (matches LocalFsBackend first-access semantics)
patterns_established:
  - write_mock_scp(dir, on_push, on_pull) — direction detection via positional arg inspection (colon in remote spec)
  - write_mock_ssh(dir, cmd_handlers) — prefix-match dispatch on remote command string
  - with_mock_path(dir, f) — PATH override helper wrapping closure (same as with_mock_gh_path)
  - manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs) — consistent unwrap pattern at all 6 CLI/MCP construction sites
observability_surfaces:
  - tracing::debug! at start of scp_push, scp_pull, ssh_run with operation name
  - tracing::warn! in factory when ssh feature disabled at build time
  - tracing::warn! in poll_inbox when remote file removal fails (non-fatal)
  - AssayError::io("scp push/pull failed: <stderr>", path, err) carries operation label + captured stderr
  - Mock scp/ssh scripts write marker files and scp_args.log for test inspection
drill_down_paths:
  - .kata/milestones/M011/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M011/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M011/slices/S04/tasks/T03-SUMMARY.md
duration: ~27m (T01: 5m, T02: 12m, T03: 10m)
verification_result: passed
completed_at: 2026-03-27
---

# S04: SshSyncBackend and CLI/MCP factory wiring

**`SshSyncBackend` implements all 7 `StateBackend` methods via `scp`/`ssh` subprocesses using `Command::arg()` chaining (injection-safe); `backend_from_config()` now dispatches all 4 config variants to real backends; all 6 CLI/MCP construction sites use the factory; `just ready` green with 1499 tests.**

## What Happened

Three tasks executed sequentially in test-first order:

**T01** established the contract via 9 `#[serial]` tests in `ssh_backend.rs` before any implementation existed. The test file references `assay_backends::ssh::SshSyncBackend` to fail at compile time intentionally, fixing the contract first. Helper functions `write_mock_scp`, `write_mock_ssh`, and `with_mock_path` follow the `github_backend.rs` pattern exactly. The injection safety test uses `remote_assay_dir = "/remote/assay dir with spaces"` and asserts scp receives the path as a single unbroken argument token.

**T02** implemented `SshSyncBackend` (~200 lines in `src/ssh.rs`). `shell_quote()` safely wraps remote paths for ssh remote commands (single-quote + `'\''` escape for embedded single quotes). `ScpRunner` builds scp/ssh commands with `.arg()` chaining throughout — uppercase `-P` for scp port, lowercase `-p` for ssh port. `SshSyncBackend` implements all 7 `StateBackend` methods: `capabilities()` returns `CapabilitySet::all()`; `push_session_event` serializes to temp file → ssh mkdir -p → scp push; `read_run_state` scp pulls to temp → deserialize, returning `Ok(None)` on failure (D174); `send_message` writes to temp → ssh mkdir -p inbox → scp push; `poll_inbox` ssh ls → for each file: scp pull + ssh rm; `annotate_run` writes manifest path to temp → scp push; `save_checkpoint_summary` serializes checkpoint → scp push. Factory updated: `Ssh` arm replaced stub with `SshSyncBackend::new(...)` behind `#[cfg(feature = "ssh")]`, non-ssh arm falls to `NoopBackend` with `tracing::warn!`. One deviation: fixed mock scp's shell script direction-detection (unquoted `$ARGS` caused word-splitting on paths with spaces — replaced with `"$@"` iteration).

**T03** wired the factory into CLI and MCP. Added `assay-backends = { workspace = true }` to both `assay-cli/Cargo.toml` and `assay-mcp/Cargo.toml`. In `run.rs`: removed `LocalFsBackend` and dead `Arc` imports, added `use assay_backends::factory::backend_from_config` and `use assay_types::StateBackendConfig`, replaced 3 hardcoded `Arc::new(LocalFsBackend::new(...))` callsites in `execute_orchestrated()`, `execute_mesh()`, and `execute_gossip()`. In `server.rs`: same import swap, replaced 3 callsites (DAG, Mesh, Gossip arms). `cargo fmt` applied, `just ready` passed all checks.

## Verification

- `cargo test -p assay-backends --features ssh -- ssh_backend` — all 9 contract tests pass including injection safety
- `cargo test -p assay-backends` (no ssh feature) — 5 factory tests pass, no compile errors
- `cargo test -p assay-cli` — 52 tests pass
- `cargo test -p assay-mcp` — 31 tests pass
- `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` — no matches
- `just ready` — green, 1499 tests run: 1499 passed, 0 skipped

## Requirements Advanced

- R078 — SshSyncBackend fully implemented with all 7 StateBackend methods and CapabilitySet::all()

## Requirements Validated

- R078 — 9 contract tests with mock scp/ssh binaries prove all methods and injection safety; factory dispatches `Ssh` → `SshSyncBackend`; `just ready` green
- R079 — CLI/MCP construction sites now use `backend_from_config()`; `grep -r "LocalFsBackend::new" crates/assay-cli crates/assay-mcp` returns no matches (T03 completes the S04 dependency noted in R079's Notes)

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- **Mock scp shell script word-splitting fix**: T01's initial `write_mock_scp` used `for arg in $ARGS` (unquoted), which caused word-splitting on paths with spaces during the injection safety test. T02 fixed the mock to use `"$@"` iteration. This was a mock script bug, not a contract change — the injection safety test continued to exercise the real `Command::arg()` behavior correctly.
- **Factory test updated**: `factory_ssh_returns_noop` renamed to `factory_ssh_capabilities` with `#[cfg(feature)]` conditional assertions — checks `CapabilitySet::all()` with ssh feature, `none()` without.
- **Removed unused `Arc` import** from `run.rs` — became dead after `backend_from_config()` returns `Arc<dyn StateBackend>` directly.

## Known Limitations

- Real multi-machine SCP validation is UAT only — no test exercises a live SSH server
- `read_run_state` returns `Ok(None)` on any scp pull failure, including transient network errors. This matches LocalFsBackend semantics but may mask connectivity issues silently; callers may retry indefinitely.
- `poll_inbox` non-fatal `tracing::warn!` on ssh rm failure means inbox messages could be delivered twice on retry

## Follow-ups

- UAT against a live SSH server with actual scp push/pull across machines
- M012: checkpoint persistence on remote backends (SshSyncBackend is ready; orchestrator wiring deferred)
- LinearBackend and GitHubBackend end-to-end UAT with real API key and gh CLI

## Files Created/Modified

- `crates/assay-backends/tests/ssh_backend.rs` — 9 contract tests for SshSyncBackend (created T01, fixed T02)
- `crates/assay-backends/src/ssh.rs` — New: complete SshSyncBackend + ScpRunner implementation (~200 lines)
- `crates/assay-backends/src/lib.rs` — Added `pub mod ssh` behind `#[cfg(feature = "ssh")]`
- `crates/assay-backends/src/factory.rs` — Replaced noop Ssh stub with real dispatch; updated factory test
- `crates/assay-backends/Cargo.toml` — Added `tempfile` optional dep gated behind ssh feature
- `crates/assay-cli/Cargo.toml` — Added `assay-backends = { workspace = true }`
- `crates/assay-mcp/Cargo.toml` — Added `assay-backends = { workspace = true }`
- `crates/assay-cli/src/commands/run.rs` — Replaced 3 LocalFsBackend::new() callsites with backend_from_config(); removed dead imports
- `crates/assay-mcp/src/server.rs` — Replaced 3 LocalFsBackend::new() callsites with backend_from_config(); swapped imports

## Forward Intelligence

### What the next slice should know
- `backend_from_config()` is now live at all CLI/MCP callsites — manifests with `state_backend = { type = "ssh", ... }` route to `SshSyncBackend` when compiled with `--features ssh`
- `SshSyncBackend` is built, tested, and wired; UAT is the only remaining gap before R078 is fully proven at runtime
- All M011 requirements are now validated or proven; M011 milestone is complete pending D160–D165 decisions (already documented in prior slices) and D173–D174 (already documented in T02)

### What's fragile
- `poll_inbox` ssh ls parsing: output is newline-split filenames — any filename with embedded newlines would corrupt the parse. No such filenames are expected in practice, but no guard exists.
- `shell_quote()` uses single-quote wrapping and `'\''` for embedded single-quotes — standard, but test coverage only exercises spaces, not single-quote characters in paths.

### Authoritative diagnostics
- `cargo test -p assay-backends --features ssh -- --nocapture` shows tracing debug output for every scp/ssh operation
- Factory `tracing::warn!(backend = "ssh", ...)` fires when a manifest specifies SSH but the binary wasn't compiled with `--features ssh` — look for this with `RUST_LOG=warn`
- Mock scripts in tests write `scp_args.log` to temp dirs — examine with `--nocapture` to verify exact argv passed to scp

### What assumptions changed
- Original T01 plan assumed mock scp direction detection could use unquoted arg iteration — actual implementation required `"$@"` to preserve argument boundaries for paths with spaces.
