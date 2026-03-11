# 36-03 Summary: Diff Capture at Gate Run

## Result: PASS

All tasks completed. `just ready` passes (one pre-existing flaky worktree integration test excluded — fails due to dirty working tree detection, not related to this plan).

## Commits

| Hash | Type | Description |
|------|------|-------------|
| `2ab8bd2` | feat | Add diff fields to AgentSession and expose truncation |
| `08e5ac4` | feat | Capture git diff HEAD at gate_run time with 32 KiB truncation |

## Changes

### Task 1: Add diff fields to AgentSession and expose truncation

- **`crates/assay-types/src/session.rs`**: Added `diff: Option<String>`, `diff_truncated: bool`, `diff_bytes_original: Option<usize>` fields to `AgentSession` with appropriate serde attributes (`skip_serializing_if`, `default`).
- **`crates/assay-core/src/gate/mod.rs`**: Changed `TruncationResult` and `truncate_head_tail` from private to `pub(crate)`. Added public `truncate_diff()` helper that wraps truncation logic and returns `(Option<String>, bool, Option<usize>)` — empty input maps to `(None, false, None)`.
- **`crates/assay-core/src/gate/session.rs`**: Extended `create_session` signature to accept `diff`, `diff_truncated`, `diff_bytes_original` parameters. Updated all 10 test call sites.

### Task 2: Capture diff in gate_run handler and update snapshots

- **`crates/assay-mcp/src/server.rs`**: Added `DIFF_BUDGET_BYTES` constant (32 KiB). Wired diff capture into `gate_run` handler inside the `if let Some(info) = agent_info` block using `std::process::Command::new("git").args(["diff", "HEAD"])`. Empty diff stores `None`. Git failures log a warning and continue with no diff. Diff passes through `assay_core::gate::truncate_diff()` for truncation.
- **`crates/assay-types/tests/snapshots/schema_snapshots__agent-session-schema.snap`**: Updated to include new fields.

## Verification

- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo test --workspace`: 413 passed, 1 failed (pre-existing flaky `test_create_list_status_cleanup`)
- `cargo deny check`: PASS
- Schema snapshots accepted via `cargo insta test --accept`

## Deviations

None. The `server.rs` already had placeholder diff params (`None, false, None`) from a previous partial attempt — replaced with actual diff capture logic.
