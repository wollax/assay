---
phase: 27
plan: 1
status: complete
commit: 21e18da
---

# Plan 27-1 Summary: Workspace clippy lint and Eq derives

## What was done

1. **Workspace clippy lint**: Added `derive_partial_eq_without_eq = "deny"` to `[workspace.lints.clippy]` in root `Cargo.toml`.

2. **Crate lint inheritance**: Added `[lints] workspace = true` to all 5 crate `Cargo.toml` files (assay-cli, assay-core, assay-mcp, assay-tui, assay-types).

3. **Eq derives**: Added `Eq` to all types without f64 fields across 10 source files. Also added `PartialEq` where missing (Gate, Review, Workflow, GatesConfig, AgentState, TaskState, UsageData, SessionInfo, PruneSummary, PruneSample, PruneReport).

4. **Default on GateSection**: Added `Default` derive to `GateSection` in enforcement.rs (works because `Enforcement` already derives Default with `#[default] Required`).

5. **Float-type allowances**: Added `#[allow(clippy::derive_partial_eq_without_eq)]` with `PartialEq` to 8 types containing f64 fields (directly or transitively): GuardConfig, Config, ContextHealthSnapshot, TeamCheckpoint, BloatEntry, BloatBreakdown, DiagnosticsReport, TokenEstimate.

6. **Formatting**: Fixed extra blank lines left by prior Display impl additions.

## Verification

- `cargo clippy --workspace -- -D warnings` passes with zero warnings
- `cargo test --workspace` passes (all 329+ tests)
- `cargo fmt --all -- --check` passes
- `cargo deny check` passes
- Pre-existing plugin version mismatch (unrelated) is the only `just ready` failure

## Files changed (16)

- `Cargo.toml` — workspace lint config
- `crates/*/Cargo.toml` (5 files) — lint inheritance
- `crates/assay-types/src/*.rs` (10 files) — Eq/PartialEq/Default derives and #[allow] annotations
