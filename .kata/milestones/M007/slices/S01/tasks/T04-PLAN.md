---
estimated_steps: 4
estimated_files: 3
---

# T04: Final cleanup and `just ready` green

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

This task runs the full quality gate (`just ready`: fmt, clippy, test, deny) and fixes any issues surfaced. This is the slice's exit gate. The implementation is done; this task is about polish and proof.

Common issues to expect:
- `clippy::missing_safety_doc` or `dead_code` on new items
- Formatting inconsistencies from rapid editing
- Unused import warnings (e.g., `use crossterm::event::{self, Event}` removed from `main.rs` but compiler may flag old uses)
- `cargo deny` may flag the new `assay-harness` dep path (workspace dep should pass)

## Steps

1. **Run `cargo fmt --all` and commit any formatting changes**
   - `cargo fmt --all` — fix all formatting; confirm diff is only whitespace/style

2. **Run `cargo clippy --workspace --all-targets` and fix all warnings**
   - Common fixes: add `#[allow(dead_code)]` for fields/methods not yet used by `run()` in tests, OR mark them `pub` so clippy sees them as part of the library API; fix any `unused_imports` in `main.rs`; fix any `must_use` lint on `JoinHandle` from `agent_thread`

3. **Run `cargo test --workspace` and confirm all tests pass**
   - Expected: all 35 `assay-tui` tests + all other workspace tests
   - If any test is flaky (e.g., `launch_agent_streaming` echo test relies on `/bin/echo` path), fix the subprocess path to be portable (`sh -c 'echo line1; echo line2'` works on both macOS and Linux)

4. **Run `just ready` and confirm it exits 0**
   - `just ready` runs: `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`, `cargo deny check`
   - If `cargo deny check` surfaces new advisories from `assay-harness` dep, review and add to `deny.toml` allow-list if appropriate

## Must-Haves

- [ ] `just ready` exits 0 (no fmt, clippy, test, or deny failures)
- [ ] `cargo build -p assay-tui` produces `target/debug/assay-tui`
- [ ] All 35 `assay-tui` tests pass (27 pre-existing + 8 agent_run)
- [ ] No new clippy warnings introduced by S01 changes
- [ ] `cargo deny check` passes (no new advisories blocked)

## Verification

```bash
just ready
cargo build -p assay-tui
ls -la target/debug/assay-tui
```

## Observability Impact

- Signals added/changed: None — this is a cleanup task.
- How a future agent inspects this: `just ready` output is the canonical pass/fail signal for the slice.
- Failure state exposed: Any remaining lint or test failure is surfaced by `just ready`.

## Inputs

- All outputs from T01–T03
- `justfile` — `just ready` recipe (already exists)
- `deny.toml` — cargo-deny configuration (already exists)

## Expected Output

- Clean workspace: `just ready` exits 0
- `target/debug/assay-tui` binary present
- Slice S01 complete: all must-haves from S01-PLAN.md are satisfied
