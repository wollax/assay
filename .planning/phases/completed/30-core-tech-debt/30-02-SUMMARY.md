# 30-02 Summary: save_run() API and generate_run_id encapsulation

**Phase:** 30-core-tech-debt
**Plan:** 02
**Status:** Complete
**Duration:** ~4 minutes
**Requirement:** CORE-06

## Changes

### Task 1: Add save_run() convenience API and tighten generate_run_id

- Added `history::save_run()` public function that encapsulates run ID generation, timestamp capture, and record construction
- Changed `generate_run_id` visibility from `pub` to `pub(crate)`
- Internal caller `gate/session.rs` unaffected (still within crate)
- Added 2 tests: `test_save_run_creates_record` and `test_save_run_respects_max_history`

**Commit:** `11536cf` — `refactor(30-02): add save_run() API and tighten generate_run_id visibility`

### Task 2: Migrate external callers to save_run()

- **assay-cli** (`gate.rs`): Replaced `generate_run_id` + manual `GateRunRecord` construction with `save_run()` call
- **assay-mcp** (`server.rs`): Replaced command-only gate run save path with `save_run()` call
- Zero external references to `generate_run_id` remain

**Commit:** `55944da` — `refactor(30-02): migrate external callers to history::save_run()`

## Files Modified

- `crates/assay-core/src/history/mod.rs` — new `save_run()` function, `pub(crate)` visibility, 2 new tests
- `crates/assay-cli/src/commands/gate.rs` — simplified `save_run_record()` body
- `crates/assay-mcp/src/server.rs` — simplified command-only gate history save

## Verification

- `just fmt-check` — pass
- `just lint` — pass
- `just test` — pass (all 358 core + 53 MCP tests)
- `grep generate_run_id crates/assay-cli/ crates/assay-mcp/` — zero results

## Must-Have Truths Verified

- [x] `generate_run_id` is `pub(crate)` not `pub`
- [x] External callers use `history::save_run()` instead of `generate_run_id` + manual record construction
- [x] `history::list()` warning is present (unchanged, already existed)
