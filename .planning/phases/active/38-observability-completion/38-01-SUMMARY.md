# Phase 38 Plan 1: Resolved Config Display Summary

**One-liner:** spec_get gains resolve=true parameter that returns timeout 3-tier cascade and working_dir validation inline

## What Was Done

### Task 1: Add resolve parameter and resolved config block to spec_get
- Added `resolve: bool` field to `SpecGetParams` with `#[serde(default)]`
- Updated `spec_get` tool description to mention the resolve parameter
- Built resolved block conditionally before the match arms, then inserted into both Legacy and Directory response JSON objects
- Timeout cascade shape: `{effective, spec (null), config, default}` — always same shape regardless of which tiers have values
- Working dir block: `{path, exists, accessible}` — real filesystem checks via `resolve_working_dir`
- Config tier is null when no `[gates]` section exists in config
- **Commit:** `7e55a67`

### Task 2: Add tests for resolved config in spec_get
- 3 deserialization tests: resolve defaults to false, resolve=true, resolve=false explicit
- 4 handler integration tests: no resolved key when resolve=false, full resolved block shape when resolve=true, config timeout reflected when [gates] section exists, working_dir exists/accessible for valid CWD
- **Commit:** `19f17a0`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed existing tests that construct SpecGetParams directly**
- **Found during:** Task 1
- **Issue:** Two existing handler tests (`spec_get_valid_spec_returns_content`, `spec_get_missing_spec_returns_error`) construct `SpecGetParams` without the new `resolve` field, causing compilation failure
- **Fix:** Added `resolve: false` to both existing test call sites
- **Files modified:** `crates/assay-mcp/src/server.rs`
- **Commit:** `7e55a67` (included in Task 1 commit)

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Use inline `serde_json::json!()` for resolved block | Consistent with existing spec_get pattern — no dedicated response struct |
| Build resolved block before match, insert into each arm | Avoids duplicating config/timeout extraction in both Legacy and Directory branches |
| spec tier always null | Per-criterion timeout varies across criteria; global resolved view shows config-level cascade only |

## Key Files

| File | Change |
|------|--------|
| `crates/assay-mcp/src/server.rs` | Added resolve field to SpecGetParams, conditional resolved block in spec_get handler, 7 new tests |

## Verification

- All 95 tests pass (`cargo test -p assay-mcp`)
- Clippy clean (`cargo clippy -p assay-mcp -- -D warnings`)

## Metrics

- **Duration:** ~3.5 minutes
- **Completed:** 2026-03-13
- **Tasks:** 2/2

---

*Phase: 38-observability-completion*
*Plan: 01*
