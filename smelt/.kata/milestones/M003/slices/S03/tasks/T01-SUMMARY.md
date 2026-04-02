---
id: T01
parent: S03
milestone: M003
provides:
  - RunState with 5 new serde-default fields: pr_status, ci_status, review_count, forge_repo, forge_token_env
  - PrState and CiStatus enums now derive Serialize/Deserialize (snake_case) in forge.rs
  - Phase 9 in run.rs persists forge_repo and forge_token_env into RunState at PR creation time
  - format_pr_section(state) -> Option<String> in status.rs — pub fn, returns None when pr_url absent
  - print_status() wired to call format_pr_section and print when Some
  - tests/status_pr.rs with 5 passing unit tests covering all display and backward-compat cases
requires: []
affects: [T02]
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-core/src/forge.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/tests/status_pr.rs
key_decisions:
  - "PrState and CiStatus needed Serialize/Deserialize derives to be stored in RunState TOML — added with serde(rename_all = snake_case)"
  - "format_pr_section declared pub (not pub(crate)) so the integration test in tests/ can import it directly from smelt_cli::commands::status"
  - "review_count displays as '0' when None (not 'unknown') to match natural expectation — unknown only used for state and CI"
patterns_established:
  - "serde(default) on individual RunState fields — backward-compat pattern established in S02, extended here"
  - "Observability: cat .smelt/run-state.toml exposes all 5 fields after Phase 9; smelt status renders them"
drill_down_paths:
  - .kata/milestones/M003/slices/S03/tasks/T01-PLAN.md
duration: 20min
verification_result: pass
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Extend RunState with forge context fields and add smelt status PR section

**RunState gains 5 backward-compatible serde-default fields; `smelt status` now renders a `── Pull Request ──` section when a PR exists; 5 unit tests all pass.**

## What Happened

Added `pr_status: Option<PrState>`, `ci_status: Option<CiStatus>`, `review_count: Option<u32>`, `forge_repo: Option<String>`, and `forge_token_env: Option<String>` to `RunState` in `monitor.rs`, each decorated with `#[serde(default)]` to maintain backward compatibility with state files that predate these fields.

`PrState` and `CiStatus` in `forge.rs` needed `Serialize`/`Deserialize` derives to be stored in TOML — added with `#[serde(rename_all = "snake_case")]`.

Phase 9 in `run.rs` now writes `forge_repo` and `forge_token_env` into `monitor.state` alongside the existing `pr_url`/`pr_number` fields before calling `monitor.write()`.

`format_pr_section` in `status.rs` is a `pub fn` returning `Option<String>`: `None` when `pr_url` is absent, `Some(text)` otherwise with URL, State (or "unknown"), CI (or "unknown"), and Reviews (or 0). `print_status()` calls it and prints when `Some`.

`tests/status_pr.rs` covers: absent when no URL, URL shown, all three status fields shown, "unknown" fallback, and TOML backward-compat without the new fields.

## Deviations

None — implemented exactly as planned.

## Files Created/Modified

- `crates/smelt-core/src/forge.rs` — Added `Serialize`/`Deserialize` + `serde(rename_all)` to `PrState` and `CiStatus`
- `crates/smelt-core/src/monitor.rs` — Added 5 new `#[serde(default)]` fields to `RunState`; updated `JobMonitor::new()` init
- `crates/smelt-cli/src/commands/run.rs` — Phase 9 now persists `forge_repo` and `forge_token_env`
- `crates/smelt-cli/src/commands/status.rs` — Added `format_pr_section` (pub); wired into `print_status()`
- `crates/smelt-cli/tests/status_pr.rs` — New: 5 unit tests, all passing
