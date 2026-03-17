# S05: Worktree Enhancements & Tech Debt — Research

**Date:** 2026-03-16

## Summary

S05 covers four requirements: session linkage on `WorktreeMetadata` (R012), orphan detection (R010), collision prevention (R011), and 15 worktree tech debt issues (R013). The worktree module is mature (1061 lines, 18 tests) with clean patterns — the enhancements are additive and low-risk.

Session linkage is the foundation: adding `session_id: Option<String>` to `WorktreeMetadata` enables both orphan detection (worktrees with no active `WorkSession`) and collision prevention (reject `create()` when spec already has an active worktree with in-progress session). The `WorkSession` type already stores `worktree_path` and `spec_name`, so the linkage is bidirectional.

Tech debt is mostly mechanical: missing `deny_unknown_fields` on `WorktreeMetadata`, `to_string_lossy` usage in production code, `eprintln!` instead of `tracing::warn!`, `detect_main_worktree` naming conflation, and missing tests for edge cases.

## Recommendation

Execute in three tasks:
1. **Session linkage + type changes** — Add `session_id` to `WorktreeMetadata`, update schema snapshot, update `write_metadata`/`read_metadata`, update `create()` to accept optional session_id. This is the foundation for R010/R011.
2. **Orphan detection + collision prevention** — `detect_orphans()` cross-references worktree metadata against `WorkSession` records. Collision check in `create()` rejects if spec has active worktree with in-progress session. Both functions live in `assay-core/src/worktree.rs`.
3. **Tech debt** — Batch all 15 issues in a single task since they're individually small and mechanical.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Session persistence lookup | `work_session::list_sessions()` + `load_session()` | Already proven pattern for scanning `.assay/work_sessions/` |
| Atomic metadata writes | `write_metadata()` already in worktree.rs | Consistent pattern for `.assay/worktree.json` writes |
| Path component validation | `validate_path_component()` in work_session.rs | Reuse for any ID-based file lookups |

## Existing Code and Patterns

- `crates/assay-core/src/worktree.rs` (1061 lines) — Full worktree lifecycle. `create()`, `list()`, `status()`, `cleanup()`, `detect_main_worktree()`. 18 tests (unit + integration). Pattern to follow for new functions.
- `crates/assay-types/src/worktree.rs` — `WorktreeMetadata` (2 fields: `base_branch`, `spec_slug`), `WorktreeInfo`, `WorktreeStatus`, `WorktreeConfig`. Schema snapshots exist for `WorktreeMetadata` and `WorktreeStatus`.
- `crates/assay-core/src/work_session.rs` (1335 lines) — `WorkSession` has `spec_name` and `worktree_path` fields already. `list_sessions()` returns sorted session IDs. `load_session()` loads by ID. Use these for orphan cross-referencing.
- `crates/assay-types/src/work_session.rs` — `WorkSession` struct with `phase: SessionPhase`. Active phases are `Created`, `AgentRunning`, `GateEvaluating`.
- `crates/assay-core/src/error.rs` — `AssayError` with worktree variants: `WorktreeGit`, `WorktreeGitFailed`, `WorktreeExists`, `WorktreeNotFound`, `WorktreeDirty`. May need a new variant for collision.
- `crates/assay-mcp/src/server.rs` — `worktree_create`, `worktree_list`, `worktree_status`, `worktree_cleanup` MCP tools. No `--all` cleanup variant exists.

## Constraints

- `WorktreeMetadata` is a persisted type — schema changes require snapshot update and must be backward-compatible (existing worktrees have no `session_id` field, so it must be `Option<String>`)
- `#[serde(deny_unknown_fields)]` is required on all persisted types — `WorktreeMetadata` currently **lacks** it (the other 3 worktree types have it). Adding it is a tech debt fix but must be coordinated with the `session_id` addition.
- Zero new dependencies — all work uses existing crates
- `WorkSession` lookup requires `assay_dir` (`.assay/`) path — orphan detection must accept this as a parameter
- MCP tools are additive only (D005) — if collision prevention changes `create()` signature, the MCP `worktree_create` handler must be updated without breaking existing callers

## Identified Tech Debt (15 Items)

1. **Missing `deny_unknown_fields` on `WorktreeMetadata`** — All other worktree types have it. Add `#[serde(deny_unknown_fields)]`.
2. **`to_string_lossy` in production code** (lines 310, 482) — `worktree_path.to_string_lossy().to_string()` used to pass paths to git CLI. Non-UTF-8 paths silently lose data. Use `OsStr`-aware passing or document the UTF-8 assumption.
3. **`eprintln!` in `cleanup()`** (line 493) — Branch deletion failure uses `eprintln!` instead of `tracing::warn!`. Inconsistent with rest of module.
4. **`detect_main_worktree` naming conflation** — Name suggests "detect the main worktree" but actually answers "is this CWD a linked worktree?" Returns `Some(main_repo_path)` if linked, `None` if main. Should be `detect_linked_worktree` or `resolve_main_repo` for clarity.
5. **`WorktreeConfig.base_dir` is `String` not `PathBuf`** — Every other path in the codebase is `PathBuf`. This requires conversions at every use site. Consider changing to `PathBuf` (breaking schema change — evaluate impact).
6. **Missing schema snapshot for `WorktreeInfo`** — `WorktreeMetadata` and `WorktreeStatus` have snapshots, but `WorktreeInfo` does not. It's a serialized type returned by MCP tools.
7. **Missing schema registry entry for `WorktreeInfo`** — Has `deny_unknown_fields` but no `inventory::submit!` like `WorktreeConfig` and `WorktreeMetadata`.
8. **Missing schema registry entry for `WorktreeStatus`** — Has snapshot test but no `inventory::submit!` registration.
9. **`spec_slug` field duplicated across types** — `WorktreeMetadata`, `WorktreeInfo`, and `WorktreeStatus` all have `spec_slug: String`. Not a bug but noted for awareness.
10. **No test for `read_metadata` with corrupt JSON** — `read_metadata` logs a warning and returns `None` for corrupt data, but no test exercises this path.
11. **No test for `write_metadata` git exclude behavior** — The git exclude write logic is complex but untested (relies on integration tests only).
12. **No test for `list()` prune failure warning** — `list()` captures prune failures as warnings but no test verifies this behavior.
13. **`ASSAY_WORKTREE_DIR` env var undocumented** — Used in `resolve_worktree_dir` but not mentioned in config docs or help text.
14. **MCP `worktree_cleanup` lacks `--all` mode** — No way to clean up all worktrees in one call. Must call per-spec. (Note: this is additive MCP work — evaluate if it fits S05 scope or should be deferred.)
15. **Prune failure in `list()` swallowed silently in MCP** — `WorktreeListResult.warnings` is returned from `list()` but the MCP `worktree_list` handler may not surface them.

## Common Pitfalls

- **Breaking `WorktreeMetadata` deserialization** — Adding `session_id` must use `Option<String>` with `#[serde(default)]` so existing metadata files without the field still parse. Adding `deny_unknown_fields` at the same time is safe because existing files only contain known fields.
- **Cross-crate dependency direction** — `assay-core/src/worktree.rs` should call `work_session` functions for orphan detection, which is fine (both in assay-core). Do NOT make assay-types depend on session logic.
- **`create()` signature change** — Adding `session_id: Option<&str>` to `create()` is a source-breaking change for all callers (MCP handler, tests). Must update all call sites.
- **Collision prevention race condition** — Between checking for active session and creating worktree, a session could be created. This is acceptable for single-agent M001 — document the limitation for M002 multi-agent.

## Open Risks

- **`WorktreeConfig.base_dir` String→PathBuf migration** — This is a schema-breaking change for `config.toml` parsing. May be better to defer or handle with a serde adapter. Evaluate during execution — if risky, keep as `String` and add a `fn as_path(&self) -> &Path` helper instead.
- **MCP `cleanup --all` scope** — Adding new MCP tool parameters violates D005 (no modification to existing signatures). A new `worktree_cleanup_all` tool would be additive but may be out of scope for S05. Evaluate during planning.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | — | Core language, no skill needed |
| Git worktrees | — | Domain-specific, no skill available |

No external skills are relevant — this is pure Rust + git CLI work on an existing codebase.

## Sources

- `crates/assay-core/src/worktree.rs` — full worktree module read (1061 lines)
- `crates/assay-types/src/worktree.rs` — type definitions (89 lines)
- `crates/assay-core/src/error.rs` — error variants (316 lines)
- `crates/assay-types/src/work_session.rs` — WorkSession type definition
- `crates/assay-core/src/work_session.rs` — session persistence API
- `crates/assay-mcp/src/server.rs` — MCP worktree tool handlers
- `crates/assay-types/tests/snapshots/` — existing schema snapshots
