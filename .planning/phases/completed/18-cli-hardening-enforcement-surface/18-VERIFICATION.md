# Phase 18 Verification: CLI Hardening & Enforcement Surface

## Status: PASSED

## Must-Haves Verification

### CLI-01: main() returns Result for proper error propagation
- **Status:** PASS
- `run() -> anyhow::Result<i32>` pattern with catch-at-top in `main()`
- Single `process::exit()` in entire file (verified: `grep -c` returns 1)

### CLI-02: Bare invocation exits with non-zero code
- **Status:** PASS
- Outside project: prints help + hint, returns `Ok(1)`
- Inside project: shows status, returns `Ok(0)`

### CLI-03: .assay path extracted to named constant
- **Status:** PASS
- `const ASSAY_DIR_NAME: &str = ".assay"` and `fn assay_dir()` helper
- Zero `.join(".assay")` literals remain (verified: `grep -c` returns 0)

### CLI-04: Gate command help duplication resolved
- **Status:** PASS
- Gate parent: "Manage quality gates" (line 89)

### ENFC-03: CLI exit code reflects only required failures
- **Status:** PASS
- `counters.failed` incremented only for required failures
- `counters.warned` incremented only for advisory failures
- Exit code: `counters.failed > 0` (streaming) / `enforcement.required_failed > 0` (JSON)
- Advisory failures display yellow WARN with `[advisory]` prefix
- Summary: "N passed, M failed, K warned, J skipped (of T total)"

## Build Verification

- `just ready` passes (fmt, clippy, 271 tests, cargo-deny)
- No clippy warnings
- All existing tests pass

## Key Metrics

| Metric | Value |
|--------|-------|
| Plans | 2/2 complete |
| Tests passing | 271 |
| process::exit calls | 1 (main only) |
| .assay literals | 0 |
