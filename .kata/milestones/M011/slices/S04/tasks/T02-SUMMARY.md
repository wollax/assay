---
id: T02
parent: S04
milestone: M011
provides:
  - Complete SshSyncBackend implementation with all 7 StateBackend methods
  - ScpRunner low-level scp/ssh command wrapper using Command::arg() chaining
  - shell_quote helper for safe remote command string composition
  - Factory wiring with cfg(feature = "ssh") gating
key_files:
  - crates/assay-backends/src/ssh.rs
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-backends/Cargo.toml
key_decisions:
  - Added tempfile as optional dependency gated behind ssh feature (not available as regular dep in assay-backends)
  - Used timestamp-based filenames for checkpoint summaries (avoids uuid dependency)
  - shell_quote uses single-quote wrapping with '\'' escape pattern for all remote paths in ssh commands
patterns_established:
  - ScpRunner::scp_push/scp_pull pattern — build_scp_base() + .arg(local).arg(remote_spec) / .arg(remote_spec).arg(local)
  - ensure_remote_dir() helper consolidates ssh mkdir -p calls with shell_quote
  - to_remote_path() strips local_assay_dir prefix to compute remote relative paths
observability_surfaces:
  - tracing::debug! at start of scp_push, scp_pull, ssh_run with operation name
  - AssayError::io("scp push/pull failed: <stderr>", path, err) carries operation label + captured stderr
  - tracing::warn! in factory when ssh feature is disabled at build time
  - tracing::warn! in poll_inbox when remote file removal fails (non-fatal)
duration: 12 minutes
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T02: Implement SshSyncBackend and wire into factory

**Implemented complete SshSyncBackend with ScpRunner wrapper, all 7 StateBackend methods, shell_quote injection protection, and factory dispatch behind `#[cfg(feature = "ssh")]`.**

## What Happened

Created `crates/assay-backends/src/ssh.rs` (~200 lines) with:
- `shell_quote()` helper for safe single-quote wrapping of paths in remote commands
- `ScpRunner` struct with `host_spec()`, `remote_spec()`, `build_scp_base()`, `scp_push()`, `scp_pull()`, and `ssh_run()` — all using `Command::arg()` chaining (D163 compliance)
- `SshSyncBackend` struct implementing all 7 `StateBackend` methods with `CapabilitySet::all()`
- Port flags correctly differentiated: scp uses uppercase `-P`, ssh uses lowercase `-p`
- `read_run_state` returns `Ok(None)` on scp pull failure (file not found)
- `poll_inbox` returns `Ok(vec![])` when remote inbox doesn't exist

Registered `pub mod ssh` behind `#[cfg(feature = "ssh")]` in `lib.rs`. Replaced the noop stub in `factory.rs` with real SshSyncBackend dispatch (ssh feature) and a fallback NoopBackend arm (not-ssh feature). Updated the factory test to assert `CapabilitySet::all()` when the ssh feature is enabled.

Added `tempfile` as an optional dependency gated behind the `ssh` feature in Cargo.toml.

## Verification

- `cargo test -p assay-backends --features ssh --test ssh_backend` — **all 9 tests pass**
- `cargo test -p assay-backends` (without ssh feature) — **5 factory tests pass**, no compile errors
- `cargo clippy -p assay-backends --features ssh -- -D warnings` — **0 warnings**
- `grep -n "sh -c\|shell" crates/assay-backends/src/ssh.rs` — no shell string interpolation for user-supplied paths (only doc comments and shell_quote usage)

### Slice-level verification (partial — T02 is intermediate):
- ✅ `cargo test -p assay-backends --features ssh` — all ssh_backend contract tests pass
- ⬜ `cargo test -p assay-cli --features orchestrate` — CLI wiring not yet done (T03)
- ⬜ `cargo test -p assay-mcp` — MCP wiring not yet done (T03)
- ⬜ `just ready` — deferred to final task

## Diagnostics

- `cargo test -p assay-backends --features ssh -- --nocapture` shows tracing debug output for scp/ssh operations
- Mock scp/ssh scripts in tests write marker files and arg logs to temp dirs for inspection
- `AssayError::io("scp push failed: <stderr>", path, err)` carries operation label + captured stderr for all failures
- Factory emits `tracing::warn!(backend = "ssh", ...)` when ssh feature is disabled

## Deviations

- Fixed T01 mock scp script's direction detection: the original `for arg in $ARGS` (unquoted) caused word-splitting on paths with spaces, breaking the injection safety test. Replaced with a direct iteration over `"$@"` that properly preserves arg boundaries. This was a bug in the mock's shell script, not a contract change.
- Updated factory test `factory_ssh_returns_noop` → `factory_ssh_capabilities` with `#[cfg(feature)]` conditional assertions (now checks `CapabilitySet::all()` with ssh feature, `none()` without).

## Known Issues

None.

## Files Created/Modified

- `crates/assay-backends/src/ssh.rs` — New: complete SshSyncBackend implementation (~200 lines)
- `crates/assay-backends/src/lib.rs` — Added `pub mod ssh` behind `#[cfg(feature = "ssh")]`
- `crates/assay-backends/src/factory.rs` — Replaced noop Ssh stub with real dispatch + updated factory test
- `crates/assay-backends/Cargo.toml` — Added `tempfile` optional dep, updated ssh feature
- `crates/assay-backends/tests/ssh_backend.rs` — Fixed mock scp direction detection for paths with spaces
