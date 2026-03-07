---
phase: "23"
plan: "01"
status: complete
duration: "~12 minutes"
---

# 23-01 Summary: Guard Foundation Types

Established guard daemon foundation: GuardConfig in assay-types, PID file management, threshold evaluation, config validation, and guard-specific error variants.

## Tasks Completed

| # | Task | Status |
|---|------|--------|
| 1 | Add GuardConfig to assay-types and workspace deps | Done |
| 2 | Add guard error variants, PID file module, and threshold evaluator | Done |

## Commits

- `07999c4`: feat(23-01): add GuardConfig to assay-types and workspace deps
- `a429a2a`: feat(23-01): add guard error variants, PID file management, and threshold evaluator
- `cb6a6da`: fix(23-01): update deny.toml for notify dependency

## Deviations

1. **PID safety fix (auto-fix):** `is_process_alive()` needed guards against PID values that wrap to negative i32 (e.g., u32::MAX becomes -1, causing `kill(-1, 0)` to signal all processes on macOS). Added `i32::try_from` check and reject PIDs <= 0.

2. **deny.toml updates (auto-fix):** notify 7.0 brought in CC0-1.0 license (notify itself), bitflags 1.x duplicate (via inotify), windows-sys 0.52 duplicate, and RUSTSEC-2024-0384 advisory (instant crate via notify-types). All resolved via deny.toml allowances/skips.

3. **Existing Config struct consumers (auto-fix):** Added `guard: None` to manual Config constructions in assay-core config tests, assay-mcp server tests, and assay-types schema roundtrip test. Updated Config schema snapshot.

4. **Pre-existing circuit_breaker module:** A `guard::circuit_breaker` module already existed (from a previous session). Preserved it and included it in mod.rs exports.

## Decisions

- PID 0 and PIDs > i32::MAX are treated as "not alive" to prevent unsafe kill() behavior
- CC0-1.0 and Artistic-2.0 licenses added to allowlist for notify ecosystem
- RUSTSEC-2024-0384 (instant crate) ignored — no fix available in notify 7.x

## Test Results

- 36 guard-specific tests passing (config: 8, pid: 7, thresholds: 9, circuit_breaker: 11)
- All existing tests continue to pass
- `just ready` passes clean
