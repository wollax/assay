# S03: Large file decomposition

**Goal:** Decompose `run.rs` (791L), `ssh.rs` (976L), and serve `tests.rs` (1370L) into focused modules along natural seams while preserving all 286+ tests and maintaining `deny(missing_docs)` compliance.
**Demo:** All three files are under their size thresholds (`run.rs` < 300L, `ssh.rs` < 400L, `tests.rs` < 500L), remainder lives in focused child modules, and `cargo test --workspace` passes with zero regressions.

## Must-Haves

- `run.rs` < 300 lines — phases, dry-run, helpers extracted to child modules
- `ssh.rs` < 400 lines — trait+types in mod.rs, SubprocessSshClient in client.rs, free functions in operations.rs, mock+tests in test modules
- serve `tests.rs` < 500 lines — tests split by feature area into child modules
- `cargo test --workspace` passes with 286+ tests, 0 failures
- `cargo doc --workspace --no-deps` exits 0 with zero warnings
- `cargo build --workspace` compiles clean (no new warnings)
- All existing public API signatures preserved — no breaking changes to `pub` items

## Proof Level

- This slice proves: contract (refactoring safety — same behavior, same tests, same API)
- Real runtime required: no (compile + test suite is sufficient)
- Human/UAT required: no

## Verification

- `cargo test --workspace 2>&1 | grep 'test result:'` — all lines show 0 failures, total ≥ 286 passing
- `cargo doc --workspace --no-deps 2>&1 | grep -c warning` — 0
- `wc -l crates/smelt-cli/src/commands/run.rs` — under 300 (or `run/mod.rs` if converted to directory module)
- `wc -l crates/smelt-cli/src/serve/ssh.rs` — under 400 (or `ssh/mod.rs`)
- `wc -l crates/smelt-cli/src/serve/tests.rs` — under 500 (or `tests/mod.rs`)

## Observability / Diagnostics

- Runtime signals: none (refactoring only, no behavior changes)
- Inspection surfaces: `cargo test --workspace`, `cargo doc --workspace --no-deps`, `wc -l` on target files
- Failure visibility: compiler errors and test failures surface immediately
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: S01's `deny(missing_docs)` baseline — all new modules must have doc comments on public items
- New wiring introduced in this slice: `mod` declarations replacing flat files with directory modules; re-exports via `pub use` to preserve existing import paths
- What remains before the milestone is truly usable end-to-end: nothing — S03 is the final slice in M009

## Tasks

- [x] **T01: Decompose run.rs into directory module with phases, dry-run, and helpers** `est:30m`
  - Why: `run.rs` is 791 lines with 9 execution phases, dry-run logic, helper functions, and tests all in one file. The roadmap requires < 300 lines in the main module.
  - Files: `crates/smelt-cli/src/commands/run.rs` → `run/mod.rs`, `run/phases.rs`, `run/dry_run.rs`, `run/helpers.rs`
  - Do: Convert `run.rs` to `run/` directory module. Extract `run_with_cancellation` + all Phase logic to `phases.rs`. Extract `execute_dry_run` + `print_execution_plan` + `truncate_spec` to `dry_run.rs`. Extract `ensure_gitignore_assay` + `should_create_pr` to `helpers.rs`. Keep `RunArgs`, `execute()`, `AnyProvider` enum, and `ExecOutcome` in `mod.rs`. Re-export public items via `pub use`. Move unit tests to their respective files. Ensure `deny(missing_docs)` compliance on any new `pub` items.
  - Verify: `cargo test --workspace` all green; `wc -l crates/smelt-cli/src/commands/run/mod.rs` < 300; `cargo doc --workspace --no-deps` zero warnings
  - Done when: `run/mod.rs` < 300 lines, all tests pass, no new warnings

- [x] **T02: Decompose ssh.rs into directory module with client, operations, and mock** `est:30m`
  - Why: `ssh.rs` is 976 lines with a trait, a full subprocess client impl, free functions, a mock, and ~500 lines of tests. The roadmap requires < 400 lines in the main module.
  - Files: `crates/smelt-cli/src/serve/ssh.rs` → `ssh/mod.rs`, `ssh/client.rs`, `ssh/operations.rs`, `ssh/mock.rs`
  - Do: Convert `ssh.rs` to `ssh/` directory module. Keep `SshOutput`, `SshClient` trait in `mod.rs`. Extract `SubprocessSshClient` + its `impl SshClient` to `client.rs`. Extract `deliver_manifest`, `sync_state_back`, `run_remote_job` free functions to `operations.rs`. Extract `MockSshClient` + all test code to `mock.rs` (gated with `#[cfg(test)]`). Re-export public items. Ensure doc comments on all new `pub` items.
  - Verify: `cargo test --workspace` all green; `wc -l crates/smelt-cli/src/serve/ssh/mod.rs` < 400; `cargo doc --workspace --no-deps` zero warnings
  - Done when: `ssh/mod.rs` < 400 lines, all tests pass, no new warnings

- [x] **T03: Decompose serve tests.rs into directory module by feature area** `est:30m`
  - Why: `tests.rs` is 1370 lines covering queue, dispatch, HTTP, SSH, config, TUI, and worker tests. The roadmap requires < 500 lines in the main module.
  - Files: `crates/smelt-cli/src/serve/tests.rs` → `tests/mod.rs`, `tests/queue.rs`, `tests/dispatch.rs`, `tests/http.rs`, `tests/ssh_dispatch.rs`, `tests/config.rs`
  - Do: Convert `tests.rs` to `tests/` directory module. Keep shared helpers (`VALID_MANIFEST_TOML`, `manifest()`, `start_test_server()`) in `mod.rs`. Extract queue tests (FIFO, max_concurrent, cancel, retry) to `queue.rs`. Extract dispatch/cancellation tests to `dispatch.rs`. Extract HTTP API tests (post, get, delete) to `http.rs`. Extract SSH dispatch tests (round-robin, failover, offline, state-sync, manifest delivery) to `ssh_dispatch.rs`. Extract config tests (worker, server config parsing/validation) to `config.rs`. Keep TUI test in `mod.rs` (small). Fix all `use`/`super` imports. Run full test suite.
  - Verify: `cargo test --workspace` all green; `wc -l crates/smelt-cli/src/serve/tests/mod.rs` < 500; `cargo doc --workspace --no-deps` zero warnings
  - Done when: `tests/mod.rs` < 500 lines, all 31 serve tests pass, no new warnings

- [x] **T04: Final verification and clippy cleanup** `est:15m`
  - Why: Belt-and-suspenders verification that all size thresholds are met, all 286+ tests pass, and the workspace is clean. Also address the 16 pre-existing clippy warnings in smelt-core (S01 follow-up) since we're already touching the codebase.
  - Files: all `run/`, `ssh/`, `tests/` modules; `crates/smelt-core/src/compose.rs`, `crates/smelt-core/src/k8s.rs`
  - Do: Run full verification suite. Fix the 16 pre-existing `collapsible-if` clippy warnings in compose.rs and k8s.rs (noted in S01 summary as follow-up for S03). Verify all line-count thresholds. Run `cargo clippy --workspace`.
  - Verify: `cargo test --workspace` ≥ 286 pass, 0 fail; `cargo clippy --workspace` clean or reduced warnings; all three target files under size thresholds
  - Done when: All verification checks pass, clippy warnings addressed, workspace is in final clean state

## Files Likely Touched

- `crates/smelt-cli/src/commands/run.rs` → `run/mod.rs`, `run/phases.rs`, `run/dry_run.rs`, `run/helpers.rs`
- `crates/smelt-cli/src/serve/ssh.rs` → `ssh/mod.rs`, `ssh/client.rs`, `ssh/operations.rs`, `ssh/mock.rs`
- `crates/smelt-cli/src/serve/tests.rs` → `tests/mod.rs`, `tests/queue.rs`, `tests/dispatch.rs`, `tests/http.rs`, `tests/ssh_dispatch.rs`, `tests/config.rs`
- `crates/smelt-cli/src/commands/mod.rs` — update `mod run` if path changes
- `crates/smelt-cli/src/serve/mod.rs` — update `mod ssh`, `mod tests` if paths change
- `crates/smelt-core/src/compose.rs` — fix collapsible-if clippy warnings
- `crates/smelt-core/src/k8s.rs` — fix collapsible-if clippy warnings
