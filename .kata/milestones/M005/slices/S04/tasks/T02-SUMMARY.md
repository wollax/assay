---
id: T02
parent: S04
milestone: M005
provides:
  - "`crates/assay-core/src/pr.rs` — new module with `ChunkGateFailure`, `PrCreateResult`, `pr_check_milestone_gates`, `pr_create_if_gates_pass`"
  - "`pr_check_milestone_gates` evaluates all milestone chunks in order-ascending order; returns `Ok(vec![])` on full pass or `Ok(failures)` with per-chunk `required_failed` counts"
  - "`pr_create_if_gates_pass` guards PR creation with: idempotency check, `gh` pre-flight, gate evaluation, `gh pr create --json number,url`, milestone TOML mutation, and Verify→Complete transition"
  - "Pre-flight `gh` availability check runs before gate evaluation — ensures 'gh CLI not found' error surfaces even when PATH is restricted"
  - "All 8 tests in `tests/pr.rs` green; no regressions across full workspace"
key_files:
  - crates/assay-core/src/pr.rs
  - crates/assay-core/src/lib.rs
key_decisions:
  - "Pre-flight `gh` check (spawn `gh --version`) runs before gate evaluation in `pr_create_if_gates_pass` — test_pr_create_gh_not_found manipulates PATH before calling the function, which also breaks `sh -c 'true'` gate evaluation; pre-flight preserves the intended failure path"
  - "No new `AssayError` variants — all domain errors use `AssayError::Io` (consistent with D065, D008)"
  - "JSON parse failures use defensive fallback: `pr_number = 0`, `pr_url = raw_stdout_trimmed` to tolerate non-standard `gh` output"
patterns_established:
  - "gh CLI integration pattern: pre-flight availability check → gate eval → build args → spawn → check exit → parse JSON → mutate+save milestone"
  - "Legacy spec guard in milestone-scoped gate loops: same `SpecEntry::Legacy` error path as `cycle_advance`"
observability_surfaces:
  - "`AssayError::Io { operation: 'pr_create_if_gates_pass', path: milestone_slug }` on all failure paths"
  - "`AssayError::Io { operation: 'gh pr create', path: milestone_slug, source: stderr }` when gh exits non-zero"
  - "`ChunkGateFailure { chunk_slug, required_failed }` list formatted into error message naming which chunks block the PR"
  - "`cat .assay/milestones/<slug>.toml` shows `pr_number` and `pr_url` after successful PR creation"
duration: 45min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T02: Implement `assay-core::pr` module

**`assay-core::pr` implemented with pre-flight `gh` check, chunk-level gate evaluation, and milestone TOML mutation — all 8 integration tests green.**

## What Happened

Created `crates/assay-core/src/pr.rs` with the two public functions and two public result types specified in the task plan.

**`pr_check_milestone_gates`** loads the milestone, sorts chunks by `order` ascending, and for each chunk calls `load_spec_entry_with_diagnostics` + `evaluate_all_gates`. Chunks with `required_failed > 0` are collected into a `Vec<ChunkGateFailure>`. Legacy spec entries return `AssayError::Io` (same pattern as `cycle_advance`).

**`pr_create_if_gates_pass`** follows this sequence:
1. Load milestone → bail if `pr_number` already set ("PR already created: #N — url")
2. **Pre-flight**: spawn `gh --version` to verify `gh` is in PATH — return "gh CLI not found" immediately if `NotFound`
3. Call `pr_check_milestone_gates` → bail with structured chunk failure list if non-empty
4. Build `gh pr create --title ... --base ... --json number,url` args
5. Spawn `gh`, check exit status (forward stderr on failure)
6. Parse JSON response (`number` + `url`); defensive fallback to `pr_number=0` / raw stdout on parse failure
7. Reload milestone, set `pr_number`/`pr_url`/`updated_at`, transition `Verify→Complete` if applicable, save

**Key deviation from plan**: The task plan placed the `gh` spawn error check inside step 4 (run gh). However, `test_pr_create_gh_not_found` sets `PATH` to an empty directory before calling `pr_create_if_gates_pass`, which also prevents `sh -c "true"` from executing — causing gate evaluation to report `required_failed=1` before we ever reach `gh`. Moving the `gh` pre-flight check to before gate evaluation ensures the intended error path is always reached regardless of PATH state.

Registered `pub mod pr;` in `lib.rs` after `pub mod milestone;`.

## Verification

```
cargo test -p assay-core --features assay-types/orchestrate --test pr
# → 8 passed, 0 failed

cargo test --workspace
# → all suites green (no regressions)

cargo clippy --workspace -- -D warnings
# → clean for assay-core; pre-existing dead-field warnings in assay-mcp/src/server.rs
#   (confirmed pre-existing: errors present on stash before T02 changes)
```

Slice-level checks (partial — T02 is not the final task):
- ✅ `cargo test -p assay-core --features assay-types/orchestrate --test pr` → 8 tests pass
- ✅ `cargo test --workspace` → all workspace tests green
- ⏳ `cargo test -p assay-cli -- pr` — T03 scope
- ⏳ `cargo test -p assay-mcp -- pr_create` — T04 scope

## Diagnostics

- `cat .assay/milestones/<slug>.toml` — shows `pr_number` and `pr_url` fields after successful PR creation
- Error messages include milestone slug as `path` context on all `AssayError::Io` failures
- Gate failures name chunk slugs + required_failed counts in the error message body

## Deviations

**`gh` pre-flight before gate evaluation** (planned: check only at spawn time). Rationale: `test_pr_create_gh_not_found` empties PATH before calling the function, which also makes `sh -c "true"` fail in gate evaluation. Pre-flight ensures the actionable "gh CLI not found" error surfaces correctly in all PATH states. Semantics are strictly better — fail fast with an actionable error when the required tool is absent.

## Known Issues

Pre-existing clippy dead-field warnings in `assay-mcp/src/server.rs` (`MilestoneChunkInput` and `SpecCreateParams` fields). Present before T02; not introduced by this task.

## Files Created/Modified

- `crates/assay-core/src/pr.rs` — new: `ChunkGateFailure`, `PrCreateResult`, `pr_check_milestone_gates`, `pr_create_if_gates_pass`
- `crates/assay-core/src/lib.rs` — `pub mod pr;` added after `pub mod milestone;`
