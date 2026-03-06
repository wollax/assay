---
phase: 18
plan: "01"
subsystem: cli
tags: [anyhow, error-handling, refactor, cli]
requires: []
provides:
  - "run() -> anyhow::Result<i32> CLI pattern"
  - "ASSAY_DIR_NAME constant and assay_dir() helper"
  - "anyhow workspace dependency"
affects:
  - "18-02 (enforcement surface builds on this foundation)"
tech-stack:
  added: [anyhow]
  patterns: [catch-at-top error handling, Result-based exit codes]
key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/assay-cli/Cargo.toml
    - crates/assay-cli/src/main.rs
    - crates/assay-mcp/src/server.rs
decisions:
  - "assay_mcp::serve() error mapped via anyhow::anyhow! (Box<dyn Error> lacks Send+Sync)"
  - "Bare invocation outside project returns Ok(1) not Err (user-facing message, not error)"
  - "Gate business logic failures return Ok(1) (exit code) not Err (error propagation)"
metrics:
  duration: "~8m"
  completed: "2026-03-06"
---

# Phase 18 Plan 01: CLI Error Propagation & Constant Extraction Summary

**One-liner:** Refactored CLI to `run() -> anyhow::Result<i32>` with single `process::exit()`, extracted `.assay` to constant, fixed Gate help string.

## What Was Done

### Task 1: Add anyhow dependency and extract ASSAY_DIR_NAME constant + fix help string

- Added `anyhow = "1"` to workspace dependencies and assay-cli
- Created `ASSAY_DIR_NAME` constant (`".assay"`) and `assay_dir()` helper function
- Replaced all 12 `.join(".assay")` literals with `assay_dir()` calls
- Changed Gate parent command doc from "Run quality gates for a spec" to "Manage quality gates"

### Task 2: Migrate main() to run() pattern with anyhow::Result<i32>

- Created `async fn run() -> anyhow::Result<i32>` containing all dispatch logic
- Rewrote `main()` to catch-at-top pattern: match on `run().await`, `process::exit(code)`
- Converted `project_root()` from exit-on-error to `anyhow::Result<PathBuf>`
- Converted `load_gate_context()` to return `anyhow::Result<(...)>`
- Converted all 7 handler functions to return `anyhow::Result<i32>`
- Converted `show_status()` from `Result<(), String>` to `anyhow::Result<()>`
- Replaced all `process::exit(1)` calls with `bail!`, `?`, or `return Ok(1)`
- Used `Cli::try_parse().unwrap_or_else(|e| e.exit())` to preserve clap error formatting
- Net reduction: 120 insertions, 244 deletions (124 lines removed)

### Task 3: Verify full build and tests pass

- Fixed cargo fmt issues (import ordering, line wrapping)
- Fixed clippy `needless_borrow` in `show_status()` (root already `&Path`)
- Fixed pre-existing cargo fmt issue in assay-mcp test
- `just ready` passes: fmt-check, lint, test (142+ tests), cargo-deny

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Box<dyn Error> incompatible with anyhow::Error**

- **Found during:** Task 2
- **Issue:** `assay_mcp::serve()` returns `Box<dyn std::error::Error>` (without `Send + Sync`), which cannot convert to `anyhow::Error` via `?`
- **Fix:** Used `.map_err(|e| anyhow::anyhow!("{e}"))` to convert the error
- **Files modified:** `crates/assay-cli/src/main.rs`
- **Commit:** 7db868b

**2. [Rule 3 - Blocking] Pre-existing formatting issue in assay-mcp test**

- **Found during:** Task 3
- **Issue:** `cargo fmt --check` caught a pre-existing formatting issue in `crates/assay-mcp/src/server.rs`
- **Fix:** Applied `cargo fmt`
- **Files modified:** `crates/assay-mcp/src/server.rs`
- **Commit:** d1e021c

## Verification Results

| Check | Result |
|-------|--------|
| `cargo check -p assay-cli` | Pass |
| `grep -c "process::exit"` | 1 (in `main()` only) |
| `grep '.join(".assay")'` | 0 matches |
| `just ready` | Pass (all checks) |

## Decisions Made

1. **assay_mcp::serve() error conversion:** Used `anyhow::anyhow!("{e}")` to wrap `Box<dyn Error>` since it lacks `Send + Sync` bounds required by anyhow. This loses the error chain but preserves the display message. A proper fix would be to change the MCP server return type to use `anyhow::Error` or `Box<dyn Error + Send + Sync>`, but that's out of scope for this plan.

2. **Bare invocation exit code:** Returns `Ok(1)` (not `Err(...)`) when outside a project, because this is an expected user-facing condition, not an error. The hint message is printed to stderr.

3. **Gate failure exit codes:** Business logic failures (required criterion failed) return `Ok(1)`, not `Err(...)`. Only unexpected errors (IO, config parse, serialization) propagate as `Err`.

## Next Phase Readiness

Plan 18-02 can proceed immediately. The `run() -> anyhow::Result<i32>` pattern is established and all handlers return the correct types. The enforcement surface changes in 18-02 will build on this foundation.
