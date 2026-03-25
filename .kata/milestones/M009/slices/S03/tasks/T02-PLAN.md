---
estimated_steps: 6
estimated_files: 6
---

# T02: Decompose ssh.rs into directory module with client, operations, and mock

**Slice:** S03 — Large file decomposition
**Milestone:** M009

## Description

Convert `serve/ssh.rs` (976 lines) from a flat file into a `serve/ssh/` directory module. The trait and types stay in `mod.rs`, the subprocess client implementation goes to `client.rs`, the free functions (deliver, sync, run_remote) go to `operations.rs`, and the mock + all tests go to `mock.rs` (cfg(test) gated). All existing import paths (`crate::serve::ssh::*`) continue working via re-exports.

## Steps

1. Create `crates/smelt-cli/src/serve/ssh/` directory. Move `ssh.rs` to `ssh/mod.rs`.
2. Extract `SubprocessSshClient` struct + its `impl SubprocessSshClient` (build_ssh_args, build_scp_args, ssh_binary, scp_binary) + `impl SshClient for SubprocessSshClient` to `ssh/client.rs`. Re-export from `mod.rs`.
3. Extract `deliver_manifest()`, `sync_state_back()`, `run_remote_job()` free functions to `ssh/operations.rs`. Re-export from `mod.rs`.
4. Extract the entire `pub(crate) mod tests` block (MockSshClient, all test functions, test helpers) to `ssh/mock.rs`. Keep `#[cfg(test)]` gating. Re-export `MockSshClient` via `pub(crate) mod mock` in `mod.rs`.
5. Ensure doc comments on all `pub` items in new files. Adjust `use` / `super` imports in each file so they compile.
6. Verify: `cargo test --workspace`, `cargo doc --workspace --no-deps`, and `wc -l ssh/mod.rs` < 400.

## Must-Haves

- [ ] `ssh/mod.rs` exists and is < 400 lines — contains `SshOutput`, `SshClient` trait, re-exports
- [ ] `ssh/client.rs` exists with `SubprocessSshClient` + impl
- [ ] `ssh/operations.rs` exists with `deliver_manifest`, `sync_state_back`, `run_remote_job`
- [ ] `ssh/mock.rs` exists with `MockSshClient` and all SSH-related unit tests
- [ ] All existing SSH tests pass (build_ssh_args, build_scp_args, mock tests, localhost tests)
- [ ] `cargo build --workspace` compiles with no new warnings
- [ ] `cargo doc --workspace --no-deps` exits 0 with zero warnings

## Verification

- `cargo test -p smelt-cli -- ssh` — all SSH tests pass
- `cargo test --workspace` — 286+ pass, 0 failures
- `wc -l crates/smelt-cli/src/serve/ssh/mod.rs` — under 400
- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` — 0

## Observability Impact

- Signals added/changed: None — pure refactoring
- How a future agent inspects this: `cargo test`, `cargo build`, `wc -l`
- Failure state exposed: Compiler errors on broken imports/visibility

## Inputs

- `crates/smelt-cli/src/serve/ssh.rs` — the 976-line file to decompose
- `crates/smelt-cli/src/serve/mod.rs` — declares `pub mod ssh`
- `crates/smelt-cli/src/serve/tests.rs` — imports from `ssh::tests::MockSshClient` — import paths must remain valid
- `crates/smelt-cli/src/serve/dispatch.rs` — imports from `ssh` module — paths must remain valid
- S01 summary — `deny(missing_docs)` is enforced on smelt-cli

## Expected Output

- `crates/smelt-cli/src/serve/ssh/mod.rs` — trait + types + re-exports (< 400 lines)
- `crates/smelt-cli/src/serve/ssh/client.rs` — SubprocessSshClient implementation
- `crates/smelt-cli/src/serve/ssh/operations.rs` — deliver, sync, run_remote free functions
- `crates/smelt-cli/src/serve/ssh/mock.rs` — MockSshClient + all tests
