---
estimated_steps: 5
estimated_files: 4
---

# T03: `just ready` Final Pass and Write S04-SUMMARY.md

**Slice:** S04 — Integration + Observability
**Milestone:** M004

## Description

With T01 (CLI rewrites) and T02 (integration tests) complete, this task confirms the workspace is fully green and closes out the slice with a summary and STATE.md update. The milestone is complete after this task commits.

## Steps

1. **Run `just ready`** and fix any issues in order:
   - `cargo fmt --all` — fix formatting
   - `cargo clippy --workspace --all-targets --features orchestrate -- -D warnings` — fix any warnings (likely none, but may catch unused imports or variables from T01/T02)
   - `cargo test --workspace --features orchestrate` — verify all tests pass; note total count
   - `cargo deny check` — verify dependency policy
   If any clippy warnings appear in `run.rs` from the T01 rewrite (e.g. unused variables in the outcome arms if json flag skips eprintln), address them with `#[allow(unused_variables)]` or restructuring.

2. **Verify test count ≥ 1270** — the baseline from S03 was 1264. T01 adds 2 CLI tests, T02 adds 2 MCP tests + 3 core integration tests = 7 new tests minimum. Confirm with `cargo test --workspace --features orchestrate 2>&1 | grep "test result"`.

3. **Append D061 to `.kata/DECISIONS.md`**:
   ```
   | D061 | M004/S04 | convention | execute_mesh/execute_gossip use HarnessWriter pattern without merge phase | CLI stubs for mesh/gossip reuse the same `Box<HarnessWriter>` closure construction as `execute_orchestrated()` (D035) but skip Phase 2 (checkout) and Phase 3 (merge) — Mesh/Gossip produce parallel outcomes only, no branch merging. `OrchestrationResponse.merge_report` is an empty zero-filled struct for now (D005 additive — no new response types needed). | Yes — if mesh/gossip need post-run merge in a future milestone |
   ```

4. **Write `S04-SUMMARY.md`** in `.kata/milestones/M004/slices/S04/` using the standard frontmatter-plus-prose format:
   - `id: S04`, `milestone: M004`
   - `provides`: executed CLI mesh/gossip with real runners; MCP mesh/gossip status test coverage; all-modes integration_modes.rs
   - `requires`: S02 (run_mesh full impl), S03 (run_gossip full impl)
   - `key_files`: run.rs, mcp_handlers.rs, integration_modes.rs
   - `key_decisions`: D061
   - `verification_result: passed`
   - `completed_at: <today>`
   - Prose sections: What Happened, Verification, Requirements Validated, Files Created/Modified, Forward Intelligence

5. **Update `.kata/STATE.md`**: mark S04 complete, mark M004 milestone complete, update total test count to the verified number from step 2.

6. **Commit all changes**: `feat(S04): wire execute_mesh/execute_gossip real runners, all-modes tests, close M004`

## Must-Haves

- [ ] `just ready` exits 0 with 0 warnings
- [ ] Test count ≥ 1270 reported by cargo test
- [ ] D061 appended to DECISIONS.md
- [ ] S04-SUMMARY.md written and committed
- [ ] STATE.md updated: S04 ✓, M004 ✓, test count updated
- [ ] Milestone M004 Definition of Done satisfied: S01 ✓ S02 ✓ S03 ✓ S04 ✓, all schema snapshots locked (verified in prior slices), no existing MCP tool signatures changed

## Verification

- `just ready` — exits 0
- `cargo test --workspace --features orchestrate 2>&1 | tail -5` — shows test count ≥ 1270, 0 failed
- `cat .kata/milestones/M004/slices/S04/S04-SUMMARY.md | head -5` — shows `id: S04` frontmatter
- `git log --oneline -1` — shows the feat(S04) commit

## Observability Impact

- Signals added/changed: none at runtime
- How a future agent inspects this: `cat .kata/STATE.md` — M004 shows complete; `git log --oneline | grep S04` — commit history confirms slice closed
- Failure state exposed: if `just ready` fails, the cargo output shows which check failed (fmt/lint/test/deny) with precise file:line

## Inputs

- T01 output: `execute_mesh()` / `execute_gossip()` rewritten in `run.rs`
- T02 output: `mcp_handlers.rs` with 2 new tests; `integration_modes.rs` with 3 new tests
- `.kata/milestones/M004/slices/S04/S04-PLAN.md` — must-haves to verify against
- `.kata/milestones/M004/slices/S02/S02-SUMMARY.md` and S03-SUMMARY.md — reference for Forward Intelligence and Requirements Validated sections
- `.kata/DECISIONS.md` — append D061

## Expected Output

- `just ready` — exits 0 with all checks passing
- `.kata/milestones/M004/slices/S04/S04-SUMMARY.md` — new file: complete slice summary
- `.kata/STATE.md` — updated: S04 and M004 marked complete
- `.kata/DECISIONS.md` — D061 appended
- Git commit `feat(S04): wire execute_mesh/execute_gossip real runners, all-modes tests, close M004`
