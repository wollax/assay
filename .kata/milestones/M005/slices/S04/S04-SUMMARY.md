---
id: S04
parent: M005
milestone: M005
provides:
  - "`Milestone` type extended with `pr_number: Option<u64>` and `pr_url: Option<String>` (serde default + skip_serializing_if); milestone schema snapshot updated"
  - "`assay-core::pr` module — `ChunkGateFailure`, `PrCreateResult`, `pr_check_milestone_gates`, `pr_create_if_gates_pass` with pre-flight `gh` check"
  - "`pr_check_milestone_gates` evaluates all milestone chunks in order-ascending order; returns empty vec on full pass, structured `ChunkGateFailure` list on partial fail"
  - "`pr_create_if_gates_pass` guards creation with idempotency check, `gh` pre-flight, gate evaluation, `gh pr create --json number,url`, milestone TOML mutation, and Verify→Complete auto-transition"
  - "`assay pr create <milestone>` CLI subcommand with `--title` and `--body` options; `feat: <milestone>` title default"
  - "`pr_create` MCP tool in `assay-mcp` router following `spawn_blocking` pattern; 11 total tests (8 integration + 2 CLI + 1 MCP presence)"
requires:
  - slice: S01
    provides: "Milestone, ChunkRef, MilestoneStatus types; milestone_load, milestone_save, milestone_scan functions"
  - slice: S02
    provides: "milestone_phase_transition, cycle_advance; gate evaluation via evaluate_all_gates"
affects: [S05, S06]
key_files:
  - crates/assay-types/src/milestone.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap
  - crates/assay-core/src/pr.rs
  - crates/assay-core/src/lib.rs
  - crates/assay-core/tests/pr.rs
  - crates/assay-core/tests/milestone_io.rs
  - crates/assay-core/tests/cycle.rs
  - crates/assay-cli/src/commands/pr.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "D077: pr_create_if_gates_pass uses `gh --json number,url` for stable machine-readable output — plain-text stdout is not stable across gh versions"
  - "D078: ChunkGateFailure and PrCreateResult live in assay-core::pr, not assay-types — response-only types stay local per D046/D051/D073 pattern"
  - "D079: test-first discipline — tests/pr.rs written in T01 (red state) before assay-core::pr exists in T02"
  - "Pre-flight gh check (`gh --version`) runs before gate evaluation — PATH restriction that breaks `sh -c 'true'` would otherwise mask the actionable 'gh CLI not found' error"
  - "CLI constructs effective_title = title.unwrap_or_else(|| format!(\"feat: {milestone}\")) before passing to core — core accepts title as a param, does not derive from milestone.name"
patterns_established:
  - "gh CLI integration pattern: pre-flight availability check → gate eval → build args → spawn → check exit → parse JSON → mutate+save milestone"
  - "Milestone struct cascade: every new optional field requires searching all Milestone { ... } literals workspace-wide before running tests (wizard.rs and wizard test also had literals)"
  - "New CLI subcommand group: create commands/<name>.rs with #[derive(Subcommand)] enum + handle fn; add pub mod to mod.rs; add variant+dispatch to main.rs"
observability_surfaces:
  - "`assay pr create <slug>` exit 0 = PR created (stdout: 'PR created: #N — <url>'); exit 1 = failure (stderr: 'Error: <msg>')"
  - "`cat .assay/milestones/<slug>.toml` shows `pr_number` and `pr_url` after successful PR creation"
  - "`AssayError::Io { operation: 'pr_create_if_gates_pass', path: milestone_slug }` on all failure paths"
  - "`ChunkGateFailure { chunk_slug, required_failed }` list in error message naming blocking chunks"
  - "MCP `pr_create` response: `{pr_number, pr_url}` on success; `isError: true` + domain message on failure"
drill_down_paths:
  - .kata/milestones/M005/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M005/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M005/slices/S04/tasks/T03-SUMMARY.md
duration: ~95min (T01: 25m, T02: 45m, T03: 25m)
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
---

# S04: Gate-Gated PR Workflow

**`assay pr create <milestone>` and `pr_create` MCP tool deliver gate-gated GitHub PR creation with idempotency, structured failure reporting, and Verify→Complete auto-transition — all 11 tests green, `just ready` clean (1331 total workspace tests).**

## What Happened

S04 completed in three tasks following strict test-first discipline.

**T01** extended `Milestone` with `pr_number: Option<u64>` and `pr_url: Option<String>` fields, updated every `Milestone { ... }` struct literal across the workspace (wider than planned — `wizard.rs` source and test files also had literals), regenerated the schema snapshot, and wrote all 8 integration tests in `tests/pr.rs` against the not-yet-existing `assay_core::pr` module (deliberate red state).

**T02** implemented `assay-core::pr` with the two public functions and two result types. A key architectural deviation from the plan: the `gh` pre-flight availability check was moved to before gate evaluation rather than at spawn time. The reason: `test_pr_create_gh_not_found` empties `PATH` before calling the function, which also breaks `sh -c "true"` during gate evaluation — without pre-flight, gate failures would mask the actionable "gh CLI not found" error. Moving pre-flight first ensures the correct error always surfaces. All 8 tests went green.

**T03** wired the CLI `assay pr create <milestone>` subcommand and the `pr_create` MCP tool. A required deviation: `pr_create_if_gates_pass`'s core signature was extended from 4 to 6 params (`title: &str`, `body: Option<&str>`) because the task plan required the CLI to pass an effective title rather than letting core derive it from `milestone.name`. All call sites in `tests/pr.rs` were updated. The full PR lifecycle is now reachable via both CLI and MCP.

## Verification

- `cargo test -p assay-core --features assay-types/orchestrate --test pr` → 8 passed
- `cargo test -p assay-cli -- pr` → 2 passed (unit tests for exit-1 paths)
- `cargo test -p assay-mcp -- pr_create` → 1 passed (presence test)
- `cargo test --workspace` → 1331 passed, 0 failed
- `just ready` → "All checks passed."
- `assay pr create --help` → shows `<MILESTONE>`, `--title`, `--body`
- `cargo clippy --workspace -- -D warnings` → clean

## Requirements Advanced

- R045 (Gate-gated PR creation) — moved from active to validated: `pr_check_milestone_gates` + `pr_create_if_gates_pass` proven by 8 integration tests; CLI and MCP wiring proven by 3 additional tests; PR creation with mock `gh` binary confirmed; milestone TOML mutation confirmed via `cat` after test
- R046 (Branch-per-chunk naming) — the `pr_create_if_gates_pass` accepts `base_branch` from `milestone.pr_base`; PR is opened from the current branch (convention enforced by caller); no regression introduced

## Requirements Validated

- R045 — Gate-gated PR creation: all three layers (core logic, CLI, MCP) proven by tests; mock-gh integration test confirms end-to-end TOML mutation and Verify→Complete transition; real GitHub PR creation is UAT-only
- R046 — Branch-per-chunk naming: convention respected; `pr_create_if_gates_pass` uses `milestone.pr_base` (defaulting to `"main"`) as the base branch; branch naming enforced by caller workflow

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

1. **`gh` pre-flight before gate evaluation** (T02): Plan placed the `gh` spawn error check inside the spawn step. Pre-flight moved before gate evaluation to ensure the correct "gh CLI not found" error surfaces when `PATH` is restricted. Semantics strictly better.
2. **`pr_create_if_gates_pass` signature extended** (T03): T02 derived title from `milestone.name`; T03 plan required the CLI to pass `effective_title`. Core signature extended to `(assay_dir, specs_dir, working_dir, milestone_slug, title, body)`. All 8 `tests/pr.rs` call sites updated.
3. **Milestone struct cascade wider than planned** (T01): `crates/assay-core/src/wizard.rs` and `crates/assay-core/tests/wizard.rs` both had `Milestone { ... }` literals not listed in the plan. Fixed during compilation feedback.

## Known Limitations

- `pr_create_if_gates_pass` reads milestone from disk, checks gates, then creates the PR — there is no transactional lock between these steps. A concurrent write between check and PR creation could produce unexpected state. Acceptable for the current single-user CLI use case.
- Wizard-generated specs (from S03) have description-only criteria without `cmd` fields (D076). Gates cannot pass until `cmd` is added manually. This is a known S03 limitation, not introduced by S04.
- `gh` must be pre-authenticated (`gh auth login`) before PR creation will succeed. No Assay-managed auth flow.

## Follow-ups

- S05 (Claude Code plugin) consumes the `pr_create` MCP tool via the `/assay:plan` → `/assay:status` → `/assay:next-chunk` skill chain
- S06 (Codex plugin) consumes `milestone_list`, `cycle_status`, and `pr_create` MCP tools via skills
- R058 (Advanced PR workflow: labels, reviewers, body templates) deferred to M008/S02

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — `pr_number`, `pr_url` fields added; 2 test literals updated
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — regenerated with new fields
- `crates/assay-core/src/pr.rs` — new: `ChunkGateFailure`, `PrCreateResult`, `pr_check_milestone_gates`, `pr_create_if_gates_pass`
- `crates/assay-core/src/lib.rs` — `pub mod pr;` added
- `crates/assay-core/tests/pr.rs` — new: 8 integration tests
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper updated
- `crates/assay-core/tests/cycle.rs` — `make_milestone_with_status` + 2 inline literals updated
- `crates/assay-core/tests/wizard.rs` — `make_milestone` helper updated
- `crates/assay-core/src/wizard.rs` — 2 `Milestone { ... }` construction sites updated
- `crates/assay-cli/src/commands/pr.rs` — new: `PrCommand`, `handle`, `pr_create_cmd`, 2 unit tests
- `crates/assay-cli/src/commands/mod.rs` — `pub mod pr;` added
- `crates/assay-cli/src/main.rs` — `Command::Pr` variant + dispatch arm added
- `crates/assay-mcp/src/server.rs` — `PrCreateParams`, `pr_create` method, `pr_create_tool_in_router` test, doc comment updated

## Forward Intelligence

### What the next slice should know
- The `pr_create` MCP tool is fully registered and tested — S05/S06 skills can call it with `{ milestone_slug, title, body? }` and receive `{ pr_number, pr_url }` or an `isError` response
- `ChunkGateFailure` and `PrCreateResult` are exported from `assay_core::pr::*` — import them directly for any downstream wiring
- The milestone TOML gains `pr_number` and `pr_url` fields after successful PR creation — `milestone_load` will return them on subsequent reads

### What's fragile
- `with_mock_gh_path` helper in `tests/pr.rs` uses `unsafe { std::env::set_var }` + `#[serial]` — any future PATH-mutating tests in this file must also use `#[serial]` or they will race
- Pre-flight `gh --version` spawn runs unconditionally even when gates fail first in non-test paths — the pre-flight is cheap but adds a subprocess per `pr_create` call

### Authoritative diagnostics
- `cat .assay/milestones/<slug>.toml` — confirms `pr_number` and `pr_url` were written after a successful real `gh pr create` call
- `assay pr create <slug>` exit code + stderr — exit 0 = success; exit 1 + stderr message is the complete diagnostic surface

### What assumptions changed
- Title derivation: plan assumed core would derive `"feat: <milestone-name>"` from `milestone.name`. Actual: core accepts `title: &str`; CLI constructs the default. Better separation of concerns.
