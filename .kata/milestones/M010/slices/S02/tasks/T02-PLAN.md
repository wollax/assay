---
estimated_steps: 3
estimated_files: 1
---

# T02: Extract build_common_ssh_args and deduplicate SSH arg builders

**Slice:** S02 — Teardown error handling + SSH DRY cleanup
**Milestone:** M010

## Description

Extract the shared flag-building logic from `build_ssh_args()` and `build_scp_args()` into a `build_common_ssh_args()` helper, parameterized by port flag (`-p` vs `-P`) and tool name (`"SSH"` vs `"SCP"` for tracing). The two public methods become thin wrappers delegating to the common helper.

## Steps

1. Read `client.rs` to confirm the two functions and their differences (port flag `-p` vs `-P`, log strings `"SSH"` vs `"SCP"`)
2. Extract `fn build_common_ssh_args(worker, timeout_secs, port_flag, tool_name, extra_args) -> Vec<String>` containing: common `-o` flags, port handling with the parameterized flag, key_env resolution with tracing using `tool_name`, extra_args append
3. Rewrite `build_ssh_args` and `build_scp_args` as ≤5-line delegations to `build_common_ssh_args`
4. Run full workspace tests and final verification (clippy, doc)

## Must-Haves

- [ ] `build_common_ssh_args()` exists with doc comment
- [ ] `build_ssh_args()` body is pure delegation (≤5 lines)
- [ ] `build_scp_args()` body is pure delegation (≤5 lines)
- [ ] All existing SSH arg tests in `mock.rs` pass without modification
- [ ] `cargo test --workspace` passes with 0 failures
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo doc --workspace --no-deps` zero warnings

## Verification

- `cargo test --workspace` — all 286+ tests pass, 0 failures
- `cargo clippy --workspace` — clean
- `cargo doc --workspace --no-deps` — zero warnings
- All SSH arg tests in `mock.rs` pass (specifically: `test_build_ssh_args_*`, `test_build_scp_args_*`)

## Observability Impact

- Signals added/changed: None — tracing messages unchanged (tool_name parameter preserves "SSH" vs "SCP" distinction)
- How a future agent inspects this: same as before — `tracing::debug` on identity file, `tracing::warn` on missing key_env
- Failure state exposed: None — no behavioral change, pure refactoring

## Inputs

- `crates/smelt-cli/src/serve/ssh/client.rs` — the two duplicated `build_ssh_args`/`build_scp_args` methods

## Expected Output

- `crates/smelt-cli/src/serve/ssh/client.rs` — `build_common_ssh_args` helper; two thin wrappers; ~80 fewer lines
