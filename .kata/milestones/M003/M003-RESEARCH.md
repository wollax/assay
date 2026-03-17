# M003: Conflict Resolution & Polish — Research

**Date:** 2026-03-17

## Summary

M003 covers three distinct work areas: AI conflict resolution (R026), SessionCore type unification (R025), and OTel instrumentation (R027, deferred to M004+). The conflict resolution work is the most architecturally significant — it requires modifying the merge runner's conflict-handling flow to keep the repo in a conflicted state long enough for an AI agent to resolve it, then commit. The current `merge_execute()` automatically aborts on conflict, which means the resolution handler never sees the working tree in a mergeable state.

The SessionCore unification (R025) is lower-risk but touches persisted types with `deny_unknown_fields` contracts. The two candidate types for factoring (`GateEvalContext` and `WorkSession`) share only `session_id`/`id` + `spec_name` + `created_at` — a thin overlap that may not justify the `#[serde(flatten)]` complexity, especially given the known incompatibility between `flatten` and `deny_unknown_fields` in serde. The project already has a working `flatten` pattern in `context.rs` (without `deny_unknown_fields`), which proves it can work — but `GateEvalContext` currently uses `deny_unknown_fields` and would need to drop it.

**Primary recommendation:** Start with R026 (conflict resolution) as it's the differentiating feature and has the most design risk. Prove the merge-execute flow change first (keeping conflict state alive for the handler), then wire the evaluator agent. Defer R025 until API surface stabilizes through real usage — the cost/benefit ratio is unfavorable now given the thin field overlap and `deny_unknown_fields` friction.

## Recommendation

**Slice ordering:** R026 conflict resolution first (highest risk, highest value), then R025 if the API surface justifies it after usage.

**For R026:** Split `merge_execute()` into a two-phase approach: (1) attempt merge, (2) on conflict, leave the repo in conflict state and call the handler, (3) if handler returns `Resolved`, verify the commit; if `Skip`/`Abort`, run `git merge --abort`. This matches the existing `ConflictAction::Resolved(sha)` contract — the handler is expected to produce a commit SHA.

**For R025:** Consider deferring entirely or implementing as a non-breaking additive change (new `SessionCore` struct that existing types can `From`-convert to, without changing persisted schemas). The `#[serde(flatten)]` approach from the brainstorm is risky with `deny_unknown_fields`.

**For R027 (OTel):** Already deferred to M004+ per D030. Keep it deferred.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Conflict detection | `merge_check()` in `assay-core/src/merge.rs` | Already uses `git merge-tree --write-tree` for zero-side-effect conflict detection; use for pre-flight check before attempting real merge |
| Conflict marker scanning | `scan_files_for_markers()` in `assay-core/src/merge.rs` | Parses `<<<<<<<`/`=======`/`>>>>>>>` markers into structured `ConflictScan`; pass to AI evaluator as context |
| Evaluator subprocess | `run_evaluator()` in `assay-core/src/evaluator.rs` | Spawns Claude with `--json-schema` for structured output; adapt for conflict resolution prompts |
| Merge ordering | `order_sessions()` in `assay-core/src/orchestrate/ordering.rs` | CompletionTime and FileOverlap strategies already minimize conflicts; no need to reinvent |

## Existing Code and Patterns

- `crates/assay-core/src/merge.rs` — `merge_execute()` runs `git merge --no-ff`, auto-aborts on conflict, returns `MergeExecuteResult` with `conflict_details`. **Must be modified** to optionally leave repo in conflict state for the handler.
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `merge_completed_sessions()` loops over sessions, calls `merge_execute()`, invokes `conflict_handler` closure on conflict. The `ConflictAction::Resolved(sha)` path already exists but is untested with real resolution. **Key integration point.**
- `crates/assay-core/src/evaluator.rs` — `run_evaluator()` spawns Claude subprocess with structured JSON output. **Reuse pattern** for conflict resolution: build a conflict-resolution prompt, pass conflicting file contents, get back resolution instructions.
- `crates/assay-types/src/orchestrate.rs` — `ConflictAction` enum (`Resolved(String)`, `Skip`, `Abort`) is the handler's return contract. Already in the type system.
- `crates/assay-types/src/session.rs` — `GateEvalContext` with `deny_unknown_fields`. Uses `session_id`, `spec_name`, `created_at`.
- `crates/assay-types/src/work_session.rs` — `WorkSession` without `deny_unknown_fields` (intentional, documented). Uses `id`, `spec_name`, `created_at`.
- `crates/assay-types/src/context.rs` — `EntryMetadata` with `#[serde(flatten)]` pattern, no `deny_unknown_fields`. **Proven pattern** for composition.
- `crates/assay-harness/src/lib.rs` — Three adapters (claude, codex, opencode) with identical module structure. No new adapters needed for M003.

## Constraints

- **`merge_execute()` auto-aborts on conflict** — The handler in `merge_completed_sessions()` receives conflict info but the working tree is already clean. To enable AI resolution, the merge must be left in conflict state until the handler resolves or aborts.
- **Sync core convention (D007)** — `run_evaluator()` is async (uses `tokio::process::Command`). Conflict resolution in the merge runner is sync (called from `std::thread::scope` workers). Need `spawn_blocking` or a dedicated tokio runtime bridge.
- **Zero-trait convention (D001)** — Conflict handler is already a closure. AI resolution must be a closure that captures an evaluator config, not a trait impl.
- **`deny_unknown_fields` + `#[serde(flatten)]` incompatibility** — serde's `deny_unknown_fields` does not work correctly with `flatten`. If SessionCore uses `flatten`, any parent struct must drop `deny_unknown_fields`. `GateEvalContext` currently has it; `WorkSession` does not.
- **Schema snapshots are locked** — Any type change to persisted types must update insta snapshots. SessionCore would change the JSON shape of serialized types.
- **Evaluator uses Claude Code exclusively** — `spawn_and_collect()` hardcodes `claude` binary. Conflict resolution would also use Claude unless a separate adapter is built.

## Common Pitfalls

- **AI resolution produces subtly wrong merges** — The conflict may look resolved but introduce semantic errors (e.g., duplicate function definitions, broken imports). Mitigate: run `cargo check` or project-specific validation after AI resolution, before accepting the commit. Consider making post-resolution validation an optional step.
- **`#[serde(flatten)]` changes JSON shape** — Extracting fields into `SessionCore` and flattening changes nothing in the JSON wire format *if done correctly*, but adding/removing `deny_unknown_fields` can break existing persisted data. Mitigate: extensive round-trip tests with fixtures from current production data.
- **Conflict handler called after abort** — Current flow: `merge_execute()` detects conflict → aborts → returns `MergeExecuteResult { was_conflict: true }` → handler is called. But the handler can't resolve because the merge is already aborted. This is the **primary bug** that M003 must fix.
- **Async evaluator in sync merge loop** — The merge runner runs in `std::thread::scope`. Spawning a tokio runtime per conflict resolution is wasteful. Better: accept a pre-built `tokio::runtime::Handle` or use `std::process::Command` (sync) for the evaluator subprocess in the conflict path.
- **Partial resolution leaves repo dirty** — If AI resolution crashes mid-way, the repo has a partial merge. Must `git merge --abort` on any handler error, not just `Skip`/`Abort`.

## Open Risks

- **AI conflict resolution quality is hard to test deterministically** — Real conflicts require agent-produced code that overlaps. Integration tests can create synthetic conflicts, but testing AI resolution quality requires real model calls or careful mocking.
- **`merge_execute()` refactor may break existing tests** — 7 existing merge runner tests assume auto-abort behavior. Changing to two-phase (leave conflict / handler resolves) requires updating all test fixtures.
- **SessionCore may not be worth the churn** — Only 3 fields overlap (`id`/`session_id`, `spec_name`, `created_at`) across `GateEvalContext` and `WorkSession`. The other session-like types (`SessionStatus`, `ManifestSession`, `SessionInfo`) have different field sets entirely. The unification may create more confusion than it resolves.
- **OTel (R027) explicitly deferred** — D030 defers to M004+. No work needed in M003.

## Requirement Analysis

### R025 (SessionCore) — Likely overbuilt for current state

The brainstorm recommended `SessionCore` with `#[serde(flatten)]`, but the actual field overlap across session types is minimal:
- `GateEvalContext`: `session_id`, `spec_name`, `created_at` + 7 domain-specific fields
- `WorkSession`: `id`, `spec_name`, `created_at` + 7 domain-specific fields
- `SessionStatus`: `name`, `spec`, `state` + 4 status fields (different names, different semantics)
- `ManifestSession`: `spec`, `name` + override fields (authoring surface, not runtime)

The naming inconsistency (`session_id` vs `id` vs `name`) and semantic differences suggest these aren't actually the same "session" concept — they're different domain types that happen to track specs. **Candidate recommendation: downgrade R025 to advisory/deferred.** If pursued, prefer a non-breaking `impl From<WorkSession> for SessionSummary` adapter over `#[serde(flatten)]` composition.

### R026 (AI Conflict Resolution) — Table stakes for M003's value proposition

This is the primary differentiator. Without it, M003 is a polish milestone with no user-visible capability gain. The merge runner already has the `ConflictAction::Resolved` path — M003 needs to:
1. Fix `merge_execute()` to optionally not abort on conflict
2. Build a conflict resolution prompt (reuse evaluator patterns)
3. Wire a concrete AI conflict handler into the merge runner

### R027 (OTel) — Correctly deferred

D030 rationale is sound. Cross-cutting instrumentation is better as a dedicated pass after surfaces stabilize.

### Candidate New Requirements

- **R028 (candidate): Post-resolution validation** — After AI resolves a conflict, run a configurable validation command (e.g., `cargo check`, `npm test`) before accepting the merge commit. Without this, AI resolution is a trust-me black box.
- **R029 (candidate): Conflict resolution audit trail** — Record what the AI changed, the original conflict markers, and the resolution rationale in the `MergeReport`. Critical for debugging when AI resolutions introduce subtle bugs.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | `modu-ai/moai-adk@moai-lang-rust` (56 installs) | available — general Rust skill, not M003-specific |
| Git merge | `personamanagmentlayer/pcl@git-expert` (71 installs) | available — general git skill, tangential |

No skills are directly relevant to the core M003 work (AI conflict resolution in a Rust merge pipeline). The existing codebase patterns are sufficient.

## Sources

- Conflict handler contract and `ConflictAction::Resolved` path (source: `crates/assay-core/src/orchestrate/merge_runner.rs`)
- `merge_execute()` auto-abort behavior (source: `crates/assay-core/src/merge.rs`, lines ~420-470)
- `#[serde(flatten)]` + `deny_unknown_fields` incompatibility (source: [serde issue #1358](https://github.com/serde-rs/serde/issues/1358))
- Evaluator subprocess pattern (source: `crates/assay-core/src/evaluator.rs`)
- Existing `#[serde(flatten)]` usage without `deny_unknown_fields` (source: `crates/assay-types/src/context.rs`)
- Session type field comparison (source: `crates/assay-types/src/{session,work_session,orchestrate,manifest}.rs`)
