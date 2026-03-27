---
id: S04
milestone: M011
status: ready
---

# S04: SshSyncBackend and CLI/MCP factory wiring — Context

## Goal

Implement `SshSyncBackend` (all 7 `StateBackend` methods via `scp`, `CapabilitySet::all()`) and wire `backend_from_config()` into all 6 `OrchestratorConfig` construction sites in `assay-cli` and `assay-mcp` so any manifest with a `state_backend` field uses the correct backend.

## Why this Slice

S02 and S03 delivered the HTTP and gh-CLI-based remote backends; S04 completes the trifecta with the "smelt native" SSH transport and ties the whole factory wiring together. Without S04, no real-world run ever uses a remote backend — the factory function exists but is never invoked at the actual construction sites.

## Scope

### In Scope

- `SshSyncBackend` struct in `assay-backends::ssh` behind `cfg(feature = "ssh")`, implementing all 7 `StateBackend` methods
- `SshSyncBackend::new(host, remote_assay_dir, user, port)` constructor
- `scp_push` / `scp_pull` helpers using `Command::arg()` chaining (never shell string)
- `SshSyncBackend` returns `CapabilitySet::all()` — mirrors `LocalFsBackend`
- Write-local-then-scp semantics: `push_session_event` writes a local `.assay/` file first, then syncs it to remote via scp; same pattern as `LocalFsBackend` extended with a sync step
- scp failure propagates as `Err` — hard fail (not silent degrade)
- `backend_from_config()` fully resolved: `Ssh` variant → `Arc::new(SshSyncBackend::new(...))`; `Linear` and `GitHub` arms wired in S02/S03 already
- All 6 `OrchestratorConfig` construction sites wired: `run.rs` (execute_orchestrated, execute_mesh, execute_gossip) and `server.rs` (3 orchestrate sites)
- `None` state_backend in RunManifest → `LocalFs` (current behavior preserved, fully additive migration)
- Contract test: scp arg construction for a path containing spaces does not produce shell-injection risk
- `just ready` green with 1497+ tests

### Out of Scope

- SshSyncBackend `send_message` / `poll_inbox` (messaging capability is `false` — no SSH inbox/outbox semantics)
- Live SSH connection in automated tests — scp arg construction is verified only; real SSH is UAT
- SshSyncBackend checkpoint persistence semantics beyond what LocalFsBackend already does
- Multi-machine smelt integration testing (automated) — UAT only

## Constraints

- D007 (sync core): all `SshSyncBackend` methods are sync; `std::process::Command` (blocking) for scp invocations
- D163: all scp arguments passed via individual `Command::arg()` calls — never shell string interpolation; prevents injection from user-supplied host/path values
- The `assay-core` dep in `assay-backends/Cargo.toml` requires the `orchestrate` feature to access `LocalFsBackend` and `StateBackend` — the `ssh` feature flag must be additive on top of this
- Factory wiring in run.rs / server.rs: `manifest.state_backend.as_ref().map_or_else(|| backend_from_config(&StateBackendConfig::LocalFs, assay_dir.clone()), |c| backend_from_config(c, assay_dir.clone()))` — or equivalent pattern that preserves `None → LocalFs`
- No new hardcoded `LocalFsBackend::new(...)` at manifest-dispatch call sites after S04

## Integration Points

### Consumes

- `assay-backends::factory::backend_from_config` (from S01) — the Ssh arm currently dispatches to NoopBackend; S04 replaces it with `SshSyncBackend::new(...)`
- `StateBackendConfig::Ssh { host, remote_assay_dir, user, port }` (from S01) — field shapes are locked
- `assay-core::LocalFsBackend` — write-local semantics reused; SshSyncBackend delegates local writes to LocalFsBackend internals or replicates the pattern, then adds scp sync
- `assay-core::StateBackend` trait — all 7 methods to implement
- `assay-cli::commands::run` — 3 hardcoded `LocalFsBackend::new()` sites to replace
- `assay-mcp::server` — 3 hardcoded `LocalFsBackend::new()` sites to replace

### Produces

- `crates/assay-backends/src/ssh.rs` — `SshSyncBackend` struct behind `cfg(feature = "ssh")`
- `crates/assay-backends/src/factory.rs` — `Ssh` arm fully wired; factory fn now resolves all 4 real variants
- Updated `assay-cli/src/commands/run.rs` — all 3 OrchestratorConfig construction sites use `backend_from_config()`
- Updated `assay-mcp/src/server.rs` — all 3 OrchestratorConfig construction sites use `backend_from_config()`
- Contract test in `crates/assay-backends/tests/` or inline — proves arg construction for a path with spaces is injection-safe

## Open Questions

- **`CapabilitySet::all()` vs LocalFsBackend subset** — SshSyncBackend mirrors LocalFsBackend capabilities. If LocalFsBackend ever drops a capability, should SshSyncBackend track it or stay at `all()`? Current thinking: match LocalFsBackend exactly; revisit if they diverge.
- **scp remote path construction** — For `push_session_event`, the remote path is assembled from `remote_assay_dir` + the relative path used locally. The exact joining convention (trailing slash handling, subdirectory creation on remote via `ssh mkdir -p` before scp) is an implementation detail for the planning phase to nail down.
