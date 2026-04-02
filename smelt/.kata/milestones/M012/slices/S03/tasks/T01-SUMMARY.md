---
id: T01
parent: S03
milestone: M012
provides:
  - GhClient trait with 4 async RPITIT methods (list_issues, edit_labels, create_label, auth_status)
  - SubprocessGhClient shelling out to gh CLI via tokio::process::Command
  - MockGhClient VecDeque-based test double for all 4 methods
  - GhIssue struct with serde Deserialize for gh --json output
  - github module registered in serve/mod.rs
key_files:
  - crates/smelt-cli/src/serve/github/mod.rs
  - crates/smelt-cli/src/serve/github/client.rs
  - crates/smelt-cli/src/serve/github/mock.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "Used SmeltError::Tracker for all GhClient errors (matching task plan), not anyhow::Result (matching SSH pattern)"
  - "MockGhClient uses SmeltError instead of anyhow — aligns with GhClient trait signature"
patterns_established:
  - "GhClient trait with RPITIT async methods in serve/github/mod.rs — mirrors SshClient pattern"
  - "MockGhClient VecDeque pattern in serve/github/mock.rs — mirrors MockSshClient"
observability_surfaces:
  - "tracing::debug! on every gh subprocess invocation with full command + args"
  - "tracing::warn! on non-zero exit codes with stderr"
  - "SmeltError::Tracker { operation: 'gh_binary' } for missing gh binary"
  - "SmeltError::Tracker { operation: 'auth_status' } for auth failures"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T01: GhClient trait, SubprocessGhClient, and MockGhClient

**GhClient trait with 4 async RPITIT methods, SubprocessGhClient shelling out to `gh` CLI, and MockGhClient VecDeque test double — all returning SmeltError::Tracker on failure**

## What Happened

Created the `serve/github/` module hierarchy mirroring the existing `serve/ssh/` pattern. The `GhClient` trait uses RPITIT async methods (per D019) returning `Result<T, SmeltError>` instead of `anyhow::Result` — this is a deliberate difference from `SshClient` since the GitHub tracker layer needs structured error types. `SubprocessGhClient` discovers `gh` via `which::which` and shells out via `tokio::process::Command` for all 4 operations. `MockGhClient` uses the same `Arc<Mutex<VecDeque<Result>>>` pattern as `MockSshClient`.

## Verification

- `cargo test -p smelt-cli --lib -- serve::github` — 8 tests passed (mock queue, serde, trait compile, binary discovery, exhausted queue)
- `cargo clippy --workspace -- -D warnings` — zero warnings
- `cargo doc --workspace --no-deps` — zero warnings

## Diagnostics

- `SmeltError::Tracker { operation: "gh_binary", message }` when `gh` is missing from PATH
- `SmeltError::Tracker { operation: "auth_status", message }` with stderr on auth failure
- `tracing::debug!` on every gh subprocess call includes full command + args
- `tracing::warn!` on non-zero exit codes includes exit code + stderr

## Deviations

- Removed the `pub(crate) mod tests` compatibility shim from `mod.rs` — it was unused and triggered a clippy warning. The SSH module has this shim because `dispatch.rs` imports from it; GitHub has no such consumer yet.
- GhClient returns `SmeltError` directly instead of `anyhow::Result` — this diverges from SshClient but matches what TrackerSource will need (structured errors for operation-specific handling).

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/github/mod.rs` — GhClient trait, GhIssue struct, module re-exports
- `crates/smelt-cli/src/serve/github/client.rs` — SubprocessGhClient with gh CLI wrappers
- `crates/smelt-cli/src/serve/github/mock.rs` — MockGhClient test double + 8 unit tests
- `crates/smelt-cli/src/serve/mod.rs` — added `pub mod github`
