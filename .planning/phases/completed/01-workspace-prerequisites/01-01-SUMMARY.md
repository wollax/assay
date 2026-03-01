---
phase: 01-workspace-prerequisites
plan: 01
subsystem: build-infrastructure
tags: [schemars, rmcp, mcp, workspace, cargo]
dependency-graph:
  requires: []
  provides:
    - schemars 1.x workspace dependency
    - rmcp 0.17 workspace dependency (server + transport-io)
    - assay-mcp library crate scaffold
  affects:
    - 02-mcp-spike (rmcp dependency available, assay-mcp crate ready)
    - 04-schema-generation (schemars 1.x API available)
    - 08-mcp-server-tools (assay-mcp crate ready for implementation)
tech-stack:
  added:
    - schemars 1.2.1 (upgraded from 0.8.22)
    - rmcp 0.17.0 (new)
    - tokio 1.49.0 (already workspace dep, now used by assay-mcp)
  patterns: []
key-files:
  created:
    - crates/assay-mcp/Cargo.toml
    - crates/assay-mcp/src/lib.rs
  modified:
    - Cargo.toml
    - Cargo.lock
decisions:
  - id: schemars-caret-range
    summary: "Used caret range schemars = \"1\" (not exact pin) — matches rmcp's own declaration, picks up semver-compatible patches"
  - id: deny-toml-unchanged
    summary: "deny.toml required no changes — all rmcp transitive deps have licenses already in the allow-list"
  - id: task-merge
    summary: "Merged Task 1 and Task 2 commits because workspace path dependency (assay-mcp) must exist for cargo check to resolve"
metrics:
  duration: "~26 minutes"
  completed: 2026-03-01
---

# Phase 01 Plan 01: Workspace Prerequisites Summary

Upgraded schemars from 0.8 to 1.x and scaffolded assay-mcp library crate with rmcp 0.17 (server + transport-io features), zero source changes to existing crates.

## What Was Done

### Task 1: Upgrade schemars and add rmcp workspace dependency

- Changed `schemars = "0.8"` to `schemars = "1"` in root `Cargo.toml`
- Added `rmcp = { version = "0.17", features = ["server", "transport-io"] }` to workspace dependencies
- Added `assay-mcp = { path = "crates/assay-mcp" }` to workspace dependencies
- Verified all existing `#[derive(JsonSchema)]` in `assay-types` compile without any source modifications
- `cargo-deny` passed without any `deny.toml` changes (the anticipated `ident_case` license issue from research did not materialize)

### Task 2: Scaffold assay-mcp library crate

- Created `crates/assay-mcp/Cargo.toml` with workspace-inherited metadata and dependencies on `assay-core`, `rmcp`, `schemars`, `serde`, `serde_json`, and `tokio`
- Created `crates/assay-mcp/src/lib.rs` with module-level doc comment (intentionally empty scaffold)
- Workspace glob `members = ["crates/*"]` auto-included the new crate
- `just ready` passes: fmt-check, clippy -D warnings, all tests, cargo-deny

## Deviations from Plan

### Task Sequencing Adjustment

**[Rule 3 - Blocking] Merged assay-mcp scaffold into Task 1 commit**

- **Found during:** Task 1
- **Issue:** The plan specified adding `assay-mcp = { path = "crates/assay-mcp" }` to workspace dependencies in Task 1, but creating the actual crate files in Task 2. Cargo cannot resolve the workspace with a path dependency pointing to a non-existent crate, making `cargo check --workspace` impossible without the scaffold files.
- **Fix:** Created the assay-mcp scaffold files alongside the workspace dependency changes in a single atomic commit, keeping the workspace in a compilable state at every commit.
- **Files affected:** `Cargo.toml`, `Cargo.lock`, `deny.toml`, `crates/assay-mcp/Cargo.toml`, `crates/assay-mcp/src/lib.rs`
- **Commit:** `1ace2b2`

## Decisions Made

| ID | Decision | Rationale |
|----|----------|-----------|
| schemars-caret-range | Used `schemars = "1"` (caret range, not exact pin) | Matches rmcp's own Cargo.toml declaration style. Resolved to 1.2.1. Semver 1.x guarantees backward compatibility. |
| deny-toml-unchanged | No deny.toml modifications needed | All rmcp transitive dependencies (async-trait, futures, tokio-util, chrono, etc.) use MIT/Apache-2.0 licenses already in the allow-list. |
| task-merge | Merged Task 1 + Task 2 into single commit | Workspace path dependency requires crate to exist for resolution. Cannot have a compilable intermediate state otherwise. |

## Verification Results

| Check | Result |
|-------|--------|
| `cargo check --workspace` | Pass (zero errors) |
| `cargo check -p assay-mcp` | Pass |
| `just ready` (fmt-check + lint + test + deny) | Pass ("All checks passed") |
| `assay-types/src/lib.rs` unchanged | Confirmed (zero diff) |
| `schemars = "1"` in Cargo.toml | Confirmed |
| `rmcp 0.17` with features in Cargo.toml | Confirmed |

## Observations

- The crossterm 0.28/0.29 duplicate (from assay-tui depending on 0.28 while ratatui 0.30 uses 0.29 via ratatui-crossterm) produces cargo-deny duplicate warnings. This is pre-existing and unrelated to this plan.
- schemars resolved to 1.2.1, the latest 1.x release. The `JsonSchema` derive macro interface is unchanged from 0.8 for the basic struct patterns used in assay-types.
- rmcp 0.17 brings in tokio, futures, chrono, and tracing as transitive dependencies. None caused license or advisory issues.

## Next Phase Readiness

Phase 02 (MCP Spike) is unblocked:
- rmcp 0.17 is available as a workspace dependency
- assay-mcp crate exists and compiles
- schemars 1.x is active (rmcp's requirement satisfied)
