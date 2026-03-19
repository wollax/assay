---
estimated_steps: 5
estimated_files: 6
---

# T01: Session linkage on WorktreeMetadata and create() signature

**Slice:** S05 — Worktree Enhancements & Tech Debt
**Milestone:** M001

## Description

Add `session_id: Option<String>` to `WorktreeMetadata` to link worktrees to work sessions. This is the foundation for orphan detection (T02) and collision prevention (T02). Update `create()` to accept and persist the session_id. Update all callers. Regenerate schema snapshot.

## Steps

1. Add `session_id: Option<String>` to `WorktreeMetadata` in `assay-types/src/worktree.rs` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Add `#[serde(deny_unknown_fields)]` to the struct (tech debt item #1).
2. Run `cargo insta test -p assay-types --accept` to regenerate the `worktree-metadata` schema snapshot. Verify the new field appears as optional string.
3. Update `create()` signature in `assay-core/src/worktree.rs` to add `session_id: Option<&str>` parameter. Thread it into the `WorktreeMetadata` struct written by `write_metadata`. Also populate `session_id` on the returned `WorktreeInfo` if we add it there (evaluate — may not be needed on `WorktreeInfo`).
4. Update all callers of `create()`: MCP `worktree_create` handler in `assay-mcp/src/server.rs` (pass `None` — session linkage from MCP is future work), all existing tests in `worktree.rs`.
5. Add unit test `test_metadata_session_id_round_trip` verifying: (a) metadata with session_id serializes and deserializes correctly, (b) metadata without session_id (legacy format) deserializes with `session_id: None`, (c) `create()` with session_id persists it to disk.

## Must-Haves

- [ ] `WorktreeMetadata` has `session_id: Option<String>` with `#[serde(default)]`
- [ ] `WorktreeMetadata` has `#[serde(deny_unknown_fields)]`
- [ ] Schema snapshot updated and accepted
- [ ] `create()` accepts `session_id: Option<&str>` and persists it
- [ ] All callers updated (MCP handler passes `None`, tests compile)
- [ ] Round-trip test for session_id (present and absent)

## Verification

- `cargo test -p assay-core -- worktree` — all existing + new tests pass
- `cargo insta test -p assay-types` — no pending snapshots
- `cargo test -p assay-mcp` — MCP tests pass with updated call site
- `cargo build --workspace` — clean compilation

## Observability Impact

- Signals added/changed: `session_id` field now visible in `.assay/worktree.json` metadata files
- How a future agent inspects this: read `.assay/worktree.json` in any worktree dir, check `session_id` field
- Failure state exposed: None (this task adds the data field; T02 adds the logic that uses it)

## Inputs

- `crates/assay-types/src/worktree.rs` — current `WorktreeMetadata` type (2 fields, no `deny_unknown_fields`)
- `crates/assay-core/src/worktree.rs` — current `create()` with 5 params
- `crates/assay-mcp/src/server.rs` — `worktree_create` handler calling `create()`
- S01 summary — persistence patterns established (atomic writes, `#[serde(deny_unknown_fields)]` convention)

## Expected Output

- `crates/assay-types/src/worktree.rs` — `WorktreeMetadata` with `session_id` field and `deny_unknown_fields`
- `crates/assay-core/src/worktree.rs` — `create()` with `session_id` param, new round-trip test
- `crates/assay-mcp/src/server.rs` — `worktree_create` passes `None` for session_id
- `crates/assay-types/tests/snapshots/schema_snapshots__worktree-metadata-schema.snap` — updated snapshot
