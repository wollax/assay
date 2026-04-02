---
id: T02
parent: S02
milestone: M010
provides:
  - build_common_ssh_args() private helper consolidating SSH/SCP flag-building logic
  - build_ssh_args() thin 1-line delegation wrapper
  - build_scp_args() thin 1-line delegation wrapper
  - ~40 fewer lines of duplicated SSH arg-building code
key_files:
  - crates/smelt-cli/src/serve/ssh/client.rs
key_decisions:
  - "build_common_ssh_args is private (fn not pub fn) — only the two public wrappers are API surface"
  - "tool_name parameter preserves SSH vs SCP distinction in tracing messages — no behavioral change"
patterns_established:
  - "build_common_ssh_args(worker, timeout_secs, port_flag, tool_name, extra_args) pattern for SSH/SCP arg building"
observability_surfaces:
  - none — pure refactoring, tracing messages unchanged
duration: 8min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: Extract build_common_ssh_args and deduplicate SSH arg builders

**Extracted `build_common_ssh_args()` private helper parameterized by port flag and tool name; `build_ssh_args`/`build_scp_args` are now single-line delegations**

## What Happened

The duplicated flag-building logic in `build_ssh_args()` and `build_scp_args()` was extracted into a private `build_common_ssh_args(worker, timeout_secs, port_flag, tool_name, extra_args)` helper. The two differences between the originals — port flag (`-p` vs `-P`) and tracing tool name (`"SSH"` vs `"SCP"`) — became parameters. Both public methods are now single-line delegations.

## Verification

| Check | Status | Evidence |
| --- | --- | --- |
| `build_common_ssh_args()` exists with doc comment | ✓ PASS | Present in client.rs with full rustdoc |
| `build_ssh_args()` body ≤5 lines | ✓ PASS | 1 line — `Self::build_common_ssh_args(...)` |
| `build_scp_args()` body ≤5 lines | ✓ PASS | 1 line — `Self::build_common_ssh_args(...)` |
| SSH arg tests pass without modification | ✓ PASS | 4/4 tests: test_ssh_args_build, test_ssh_args_build_custom_port, test_scp_args_build, test_scp_args_custom_port |
| `cargo test --workspace` | ✓ PASS | 155 tests + 3 doc-tests, 0 failures |
| `cargo clippy --workspace` | ✓ PASS | Clean |
| `cargo doc --workspace --no-deps` | ✓ PASS | Zero warnings |

## Diagnostics

None — pure refactoring. Tracing messages are identical at runtime (tool_name parameter preserves "SSH" vs "SCP" distinction).

## Deviations

Doc links in `build_ssh_args`/`build_scp_args` initially used `[`Self::build_common_ssh_args`]` which generated `cargo doc` warnings (linking to private item). Changed to plain backtick references.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh/client.rs` — Extracted `build_common_ssh_args()` helper; rewrote `build_ssh_args`/`build_scp_args` as delegations
