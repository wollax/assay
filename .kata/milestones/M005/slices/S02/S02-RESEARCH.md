# S02: Development Cycle State Machine — Research

**Date:** 2026-03-19

## Summary

S02 builds the development cycle state machine on top of S01's foundation. The core deliverables are: a `cycle.rs` module in `assay-core::milestone` with `cycle_status()`, `cycle_advance()`, and `milestone_phase_transition()`; three new MCP tools (`cycle_status`, `cycle_advance`, `chunk_status`); and two new CLI subcommands (`assay milestone status`, `assay milestone advance`). 

The key design challenge is: how do we track which chunks within a milestone are "complete"? The answer is to add a `completed_chunks: Vec<String>` field to the `Milestone` type (serde default = empty vec, skip_serializing_if empty). The "active chunk" is derived at runtime as the first entry in `milestone.chunks` sorted by `order` that is not in `completed_chunks`. This is a backward-compatible schema extension but will require updating the schema snapshot.

`cycle_advance` must check gates live (via `evaluate_all_gates` wrapped in `spawn_blocking`) before marking a chunk complete. Checking history would allow stale data to advance the cycle — live evaluation is the correct contract. `chunk_status` reads history (last run) to report current pass/fail without running gates.

**Critical prerequisite:** S02 was branched from `main` before S01 merged. Execution must begin by merging `kata/root/M005/S01` into the S02 branch (or rebasing S02 onto S01) to get the `Milestone` type, `milestone_load/save/scan`, and `milestone_list`/`milestone_get` MCP tools.

## Recommendation

Add `completed_chunks: Vec<String>` to `Milestone` in `assay-types`. Build `cycle.rs` as pure sync functions (consistent with D001/D007). Call `evaluate_all_gates` live in `cycle_advance` via `spawn_blocking` from MCP — same pattern as `gate_run`. The `chunk_status` MCP tool uses `history::list` + `history::load` to return the latest run. CLI subcommands follow the `milestone.rs` pattern from S01.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Running gates and checking pass/fail | `assay_core::gate::evaluate_all_gates(&GatesSpec, &Path, cli_timeout, config_timeout) -> GateRunSummary` | Already handles enforcement levels, Required vs Advisory. Check `summary.enforcement.required_failed == 0` for "all required pass". |
| Loading a spec (chunk) to evaluate | `assay_core::spec::load_spec_entry_with_diagnostics(slug, specs_dir) -> Result<SpecEntry>` | Handles both directory and legacy specs. For directory specs: `SpecEntry::Directory { gates, .. }` gives the `GatesSpec`. |
| Latest gate run for a chunk | `assay_core::history::list(assay_dir, spec_name) -> Vec<String>` + `history::load(assay_dir, spec_name, run_id)` | IDs sorted oldest-first; `.last()` for newest. Already used in `gate_history` MCP tool. |
| Atomic milestone persistence | `assay_core::milestone::milestone_save(assay_dir, &milestone)` | NamedTempFile + sync_all + persist — established S01 pattern. |
| MCP tool registration | `#[tool(description = "...")]` + `tool_router` macro auto-discovery | No manual registration. Pattern established for `milestone_list`/`milestone_get` in S01. |
| MCP async → sync gate evaluation | `tokio::task::spawn_blocking(move || evaluate_all_gates(...))` | Same pattern used in `gate_run` MCP tool. |
| Spec path resolution | `cwd.join(".assay").join(&config.specs_dir)` | Config contains `specs_dir` (default `"specs/"`). Used by `load_spec_entry_mcp()`. |
| Slug path traversal guard | `assay_core::history::validate_path_component(slug, "label")` | Already used in `milestone_load`/`milestone_save`. Use for chunk slug validation. |

## Existing Code and Patterns

- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types; S02 adds `completed_chunks: Vec<String>` + `pr_number`/`pr_url` (preview for S04). Add `completed_chunks` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. Update schema snapshot.
- `crates/assay-core/src/milestone/mod.rs` (on S01 branch) — `milestone_scan`, `milestone_load`, `milestone_save`; S02 adds `pub mod cycle;` here.
- `crates/assay-core/src/gate/mod.rs:171` — `pub fn evaluate_all_gates(gates: &GatesSpec, working_dir: &Path, cli_timeout: Option<u64>, config_timeout: Option<u64>) -> GateRunSummary`. Non-async; wrap in `spawn_blocking`.
- `crates/assay-mcp/src/server.rs` (gate_run handler, lines 1231–1290) — pattern for loading a spec entry and evaluating gates inside `spawn_blocking`. Reuse `load_spec_entry_mcp()` and `resolve_working_dir()` helpers.
- `crates/assay-mcp/src/server.rs:2066` (`gate_history` handler) — pattern for reading latest history: `list()` → `.rev()` → `load()`. For `chunk_status`, use `.last()` on the sorted ID list.
- `crates/assay-core/src/history/mod.rs:220` — `pub fn list(assay_dir, spec_name) -> Result<Vec<String>>`: returns run IDs sorted oldest-first. IDs include timestamp prefix so `.iter().rev().next()` = most recent.
- `crates/assay-cli/src/commands/milestone.rs` (S01) — CLI pattern: `MilestoneCommand` enum, `handle()` dispatch, one function per variant. S02 adds `Status` and `Advance` variants.
- `crates/assay-core/src/orchestrate/executor.rs` — not directly used, but `persist_state()` there demonstrates the same atomic write pattern.

## Constraints

- **Zero traits (D001):** All cycle functions must be plain functions, not methods on a struct or trait objects. `cycle_status`, `cycle_advance`, `milestone_phase_transition` are free functions in `cycle.rs`.
- **Sync core (D007):** `cycle.rs` is synchronous. MCP handlers wrap with `spawn_blocking`. No async in assay-core.
- **Additive MCP tools (D005):** Never modify existing tool signatures. `cycle_status`, `cycle_advance`, `chunk_status` are new tools with new `Params` structs.
- **S01 prerequisite:** Current S02 branch is cut from `main` (before S01 landed). Must merge `kata/root/M005/S01` into S02 branch before writing any code. The `Milestone` type, `milestone_load/save/scan`, MCP tools (24 total), and CLI stub are all on S01.
- **Schema snapshot required:** Adding `completed_chunks` to `Milestone` changes the JSON schema. Run `INSTA_UPDATE=always cargo test -p assay-types` to update `schema_snapshots__milestone-schema.snap`.
- **Feature gate bug workaround:** `cargo test -p assay-core --test cycle_*` requires `--features assay-types/orchestrate` due to pre-existing `manifest.rs` bug. Use workspace-level tests as the reliable surface: `cargo test --workspace`.
- **Spec as chunk**: Chunks are directory-based specs in `.assay/specs/<chunk-slug>/gates.toml`. `load_spec_entry` → `SpecEntry::Directory { gates, .. }` gives the `GatesSpec`. Must gracefully handle legacy specs (flat `.toml`) but directory format is expected for milestone chunks.

## Common Pitfalls

- **Forgetting to update schema snapshot** — Adding `completed_chunks` to `Milestone` will cause `schema_snapshots__milestone-schema.snap` to fail. Run `INSTA_UPDATE=always cargo test -p assay-types --test schema_snapshots` immediately after the type change.
- **Not sorting chunks before finding active** — `milestone.chunks` may not be stored in order; sort by `.order` before finding the first not in `completed_chunks`. The `ChunkRef.order` field is `u32`, sort ascending.
- **Evaluating gates in `cycle_advance` from the MCP async context without `spawn_blocking`** — `evaluate_all_gates` is sync and blocks on subprocess execution. Must use `spawn_blocking`. See `gate_run` handler pattern (lines ~1260–1285 of server.rs).
- **Not checking `required_failed` specifically** — `GateRunSummary.failed` includes advisory failures. Use `summary.enforcement.required_failed == 0` to determine if all *required* gates pass, consistent with the cycle contract (R043).
- **Active milestone ambiguity when multiple InProgress** — If multiple milestones are `InProgress`, `cycle_status` should return the first alphabetically by slug (since `milestone_scan` returns sorted). Document this in the tool description as "returns the first active milestone in alphabetical order by slug."
- **Treating an empty `completed_chunks` as progress** — An `InProgress` milestone with no completed chunks means chunk[0] (lowest order) is the active chunk. `completed_chunks` being empty is a valid state: work just started.
- **Tool count test assertions** — S01 summary says "tool count: 24". S02 adds 3 tools → 27. If any test checks `tools.len() == 24`, it will break. Check server.rs tests for count assertions. (The S01 tests use `tool_names.contains(&"...")` not length checks — but verify before proceeding.)
- **`milestone_phase_transition` and invalid transitions** — Returning an `AssayError` for invalid transitions is the right pattern. Add new `InvalidMilestoneTransition` or `MilestonePreconditionFailed` variants to `AssayError`, OR return `Err(AssayError::Io { ... })` with a descriptive message (simpler, consistent with S01's TOML error mapping). The latter is simpler.

## Open Risks

- **Working directory for gate evaluation**: `cycle_advance` evaluates gates for a chunk. The working dir should be the project root (same as `gate_run`). This requires resolving config for `gates.working_dir`. In `cycle.rs` (pure logic), pass `working_dir: &Path` as a parameter — don't resolve it inside. The MCP handler resolves it using `resolve_working_dir(&cwd, &config)` as in `gate_run`.
- **Milestone with no chunks**: `cycle_status` on a `Draft` milestone with no chunks is valid. `cycle_advance` on a milestone with no chunks should return a precondition error (cannot advance: no chunks defined).
- **Chunk slug doesn't match any spec**: `cycle_advance` loads the chunk as a spec. If `ChunkRef.slug` doesn't exist in `.assay/specs/`, `load_spec_entry_with_diagnostics` returns a descriptive error — propagate it as a domain error.
- **S04 milestone fields**: S04 will add `pr_number: Option<u64>` and `pr_url: Option<String>` to `Milestone`. No action needed in S02, but the schema snapshot will need updating again in S04. Design the S02 schema addition (`completed_chunks`) to avoid conflicts.

## Implementation Plan (for Planning)

### Type changes (assay-types)
1. Add `completed_chunks: Vec<String>` to `Milestone` with `serde(default, skip_serializing_if = "Vec::is_empty")`. Update the struct literal in the test at the bottom of `milestone.rs`. Update schema snapshot.

### cycle.rs (assay-core::milestone::cycle)

```rust
// assay-core/src/milestone/cycle.rs
//
// Pure sync functions — no async, no traits.

pub struct CycleStatus {
    pub milestone_slug: String,
    pub milestone_name: String,
    pub phase: MilestoneStatus,          // draft|in_progress|verify|complete
    pub active_chunk_slug: Option<String>,
    pub completed_count: usize,
    pub total_count: usize,
}

/// Return the active chunk for a milestone: lowest-order chunk not in completed_chunks.
/// Returns None when all chunks complete (or no chunks defined).
pub fn active_chunk(milestone: &Milestone) -> Option<&ChunkRef>

/// Find the first InProgress milestone. Returns slug of that milestone.
/// Scans `assay_dir/milestones/` via milestone_scan.
pub fn cycle_status(assay_dir: &Path) -> Result<Option<CycleStatus>>

/// Transition a milestone's status. Returns Err for invalid transitions.
/// draft→in_progress requires chunks.len() >= 1
/// in_progress→verify requires active_chunk(m).is_none() (all done)
/// verify→complete: always valid
/// all others: error
pub fn milestone_phase_transition(milestone: &mut Milestone, next: MilestoneStatus) -> Result<()>

/// Check all required gates pass for the active chunk, mark it complete, advance.
/// Loads the chunk spec from specs_dir, evaluates gates, checks required_failed == 0.
/// Saves updated milestone on success.
/// Returns updated CycleStatus on success. Returns Err if gates fail or preconditions unmet.
pub fn cycle_advance(
    assay_dir: &Path,
    specs_dir: &Path,
    working_dir: &Path,
    config_timeout: Option<u64>,
) -> Result<CycleStatus>
```

### New MCP tools (3)

**`cycle_status`**: No params. Calls `cycle_status(assay_dir)`. Returns JSON `CycleStatus` or null if no active milestone.

**`cycle_advance`**: Optional `milestone_slug: Option<String>` (if None, targets the first InProgress milestone). Calls `cycle_advance(...)` via `spawn_blocking`. Returns updated `CycleStatus` or domain error if gates fail.

**`chunk_status`**: Params: `chunk_slug: String`. Loads last gate run from history. Returns `{chunk_slug, has_history, latest_run_id?, passed?, failed?, required_failed?, results?}`.

### New CLI subcommands

**`assay milestone status`**: Prints progress table for InProgress milestones.
```
MILESTONE: my-feature (in_progress)
  [ ] chunk-one     (active — 2/3 criteria pass)
  [x] chunk-zero    (complete)
```

**`assay milestone advance`**: Calls `cycle_advance`, prints result or error.

### Test plan
- `crates/assay-core/tests/cycle.rs` — integration tests (uses real temp files):
  - `test_cycle_status_no_milestones` — returns None
  - `test_cycle_advance_transitions_to_next_chunk` — two chunks, advance once
  - `test_cycle_advance_all_chunks_moves_to_verify` — last chunk done → verify phase
  - `test_cycle_advance_gates_fail_returns_error` — spec with failing criterion
  - `test_milestone_phase_transition_invalid` — verify → draft returns error
- MCP tests in server.rs — `cycle_status_tool_in_router`, `cycle_advance_tool_in_router`, `chunk_status_tool_in_router` (presence tests, not count tests)
- CLI tests in commands/milestone.rs — `milestone_status_no_milestones`, `milestone_advance_no_active`

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (general) | — | none needed — standard Rust patterns |
| rmcp (#[tool]) | — | Pattern already established in codebase |

## Sources

- `crates/assay-core/src/gate/mod.rs:171` — `evaluate_all_gates` signature and return type (codebase)
- `crates/assay-mcp/src/server.rs:1231` — `gate_run` handler as spawn_blocking + gate eval pattern (codebase)
- `crates/assay-mcp/src/server.rs:2066` — `gate_history` handler as history pattern (codebase)
- `crates/assay-core/src/history/mod.rs:63,220` — `save_run`, `list`, `load` API (codebase)
- `.kata/milestones/M005/slices/S01/S01-SUMMARY.md` — S01 foundation: types, I/O, MCP tools, patterns established (project artifact)
- `git show kata/root/M005/S01:crates/assay-types/src/milestone.rs` — actual Milestone type and test patterns (codebase)
- `git show kata/root/M005/S01:crates/assay-core/src/milestone/mod.rs` — actual I/O functions (codebase)
- `crates/assay-types/src/enforcement.rs` — `EnforcementSummary.required_failed` is the correct gate-pass check field (codebase)
