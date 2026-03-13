# Phase 38 Plan 2: Growth Rate Metrics Summary

**One-liner:** Token growth rate estimation from non-sidechain assistant turn history with configurable threshold

## What Was Done

### Task 1: Add GrowthRate type and update TokenEstimate
- Added `GrowthRate` struct to `assay-types` with `avg_tokens_per_turn`, `estimated_turns_remaining`, `turn_count`
- Added `growth_rate: Option<GrowthRate>` field to `TokenEstimate` with `skip_serializing_if = "Option::is_none"`
- Updated all construction sites (core, types tests) with `growth_rate: None`
- Commit: `9950669`

### Task 2: Implement growth rate computation and integration
- Added `collect_turn_tokens` function: full session parse, filters non-sidechain assistant entries with usage data
- Added `compute_growth_rate` function: returns `None` below 5-turn threshold, computes avg/remaining from cumulative token snapshots
- Integrated growth rate computation into `estimate_tokens` (now does a full parse in addition to tail-read for usage)
- Updated MCP tool description to document growth rate metrics, removed "Fast: tail-read only" claim
- Added 6 unit tests covering threshold boundary, correct calculation, saturation, zero avg, and sidechain filtering
- Commit: `85ecb19`

## Decisions Made

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Growth rate uses last cumulative token count divided by turn count for average | Simple, stable metric that doesn't require per-turn deltas |
| 2 | `estimate_tokens` now does both tail-read (for usage) and full parse (for growth rate) | Growth rate requires all turns; tail-read still provides the latest usage efficiently |

## Deviations from Plan

None - plan executed exactly as written.

## Verification

- `cargo test --workspace`: 678 passed, 3 ignored
- `cargo clippy --workspace -- -D warnings`: clean
- All 6 new growth rate tests pass

## Key Files

### Created
(none)

### Modified
- `crates/assay-types/src/context.rs` — GrowthRate struct, TokenEstimate.growth_rate field
- `crates/assay-core/src/context/tokens.rs` — collect_turn_tokens, compute_growth_rate, estimate_tokens integration, tests
- `crates/assay-mcp/src/server.rs` — updated estimate_tokens tool description
- `crates/assay-types/tests/context_types.rs` — updated TokenEstimate construction sites

## Duration

~26 minutes

## Next Phase Readiness

Phase 38 is now complete (2/2 plans done). No blockers for subsequent phases.
