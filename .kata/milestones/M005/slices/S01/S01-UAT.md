# S01: Milestone & Chunk Type Foundation — UAT

**Milestone:** M005
**Written:** 2026-03-19

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is a pure data model + I/O + MCP/CLI wiring slice with no interactive wizard, no browser flow, and no external service calls. All observable behavior is verifiable through `cargo test`, `just ready`, and the `assay milestone list` CLI command. No human-experience validation is needed at this tier.

## Preconditions

- Rust toolchain installed, `just` available
- Working directory: project root (`/Users/wollax/Git/personal/assay`)
- No `.assay/` directory required — test cases cover both present and absent states

## Smoke Test

```
cargo test --workspace 2>&1 | grep "test result"
```

Expected: all lines show `0 failed`. Total passed ≥ 1293.

## Test Cases

### 1. Backward-compatible GatesSpec parsing

1. Run: `cargo test --workspace -- gates_spec_rejects_unknown_fields`
2. Run: `cargo test --workspace -- gates_spec_milestone_fields`
3. **Expected:** Both commands report `ok`. Existing gates.toml files without `milestone`/`order` fields parse without error. Files with unknown fields are still rejected.

### 2. Milestone type round-trip

1. Run: `cargo test --workspace -- milestone`
2. **Expected:** All 10+ tests pass. `Milestone`, `ChunkRef`, `MilestoneStatus` serialize to TOML and deserialize back with identical values. Unknown TOML fields produce a parse error. Minimal form (only required fields) parses correctly.

### 3. Schema snapshots locked

1. Run: `cargo test --workspace -- milestone_schema`
2. Run: `ls crates/assay-types/tests/snapshots/*.snap.new 2>/dev/null || echo "no pending snapshots"`
3. **Expected:** Schema tests pass. No `.snap.new` files exist — snapshots are accepted and stable.

### 4. Milestone I/O integration

1. Run: `cargo test -p assay-core --features assay-types/orchestrate --test milestone_io`
2. **Expected:** 5 tests pass:
   - `test_milestone_save_and_load_roundtrip` — written milestone reads back with all fields identical
   - `test_milestone_scan_empty_for_missing_dir` — scanning a dir with no `.assay/milestones/` returns empty vec, not error
   - `test_milestone_scan_returns_all_milestones` — all saved milestones appear in scan result
   - `test_milestone_load_error_for_missing_slug` — loading a nonexistent slug returns `Err` (not panic)
   - `test_milestone_slug_validation_rejects_traversal` — path traversal slug (`../evil`) returns `Err`

### 5. MCP tools registered and functional

1. Run: `cargo test -p assay-mcp -- milestone`
2. **Expected:** 4 tests pass:
   - `milestone_list_tool_in_router` — tool exists in router at server init
   - `milestone_get_tool_in_router` — tool exists in router at server init
   - `milestone_list_returns_empty_json_array_for_no_milestones` — returns `[]` when no milestones exist
   - `milestone_get_returns_error_for_missing_slug` — returns `isError: true` for unknown slug

### 6. CLI subcommand

1. Run: `cargo test -p assay-cli -- milestone_list_subcommand_no_milestones`
2. **Expected:** Test passes. Exit code 0. Output contains "No milestones found."

### 7. `just ready` green

1. Run: `just ready`
2. **Expected:** Exits 0. All four checks pass: `cargo fmt --check`, `cargo clippy`, `cargo test --workspace`, `cargo deny check`. No warnings treated as errors, no lint failures.

## Edge Cases

### Milestone with unknown TOML field

1. Create a temp TOML file with an extra field not in the `Milestone` schema.
2. Attempt to parse via `milestone_load`.
3. **Expected:** Returns `AssayError::Io` with "parsing milestone TOML" in the operation and the file path included — not a panic or silent data loss.

### Empty milestones directory

1. Create `.assay/milestones/` but place no `.toml` files inside it.
2. Call `milestone_scan` or run `assay milestone list`.
3. **Expected:** Returns `Ok(vec![])` / prints "No milestones found." — empty dir is not an error.

### Slug with path traversal characters

1. Attempt to load or save a milestone with slug `../../etc/passwd` or similar.
2. **Expected:** `validate_path_component` returns `Err` before any filesystem operation occurs.

## Failure Signals

- Any `FAILED` in `cargo test --workspace` output — indicates regression or new type incompatibility
- `.snap.new` files in `crates/assay-types/tests/snapshots/` — indicates a type field changed without updating the locked schema contract
- `just ready` exits non-zero — fmt, lint, test, or deny failure
- `isError: false` from `milestone_get` with a nonexistent slug — MCP error handling broken
- `assay milestone list` exits non-zero on an empty project — scan error handling broken

## Requirements Proved By This UAT

- R039 (Milestone concept) — `Milestone`, `ChunkRef`, `MilestoneStatus` types exist with correct fields, TOML contract locked by schema snapshots, milestone directory convention established
- R040 (Chunk-as-spec) — `GatesSpec` extended with backward-compatible optional fields; `gates_spec_rejects_unknown_fields` still passes; existing specs unaffected
- R041 (Milestone file I/O) — atomic write + read + scan proven by 5 integration tests; empty dir returns empty collection; error paths return structured `AssayError::Io` with path and operation

## Not Proven By This UAT

- Live MCP protocol interaction with a running server — tests exercise the tool logic directly, not via JSON-RPC wire protocol
- `assay milestone list` with real milestone files in a real project directory (tested with temp dirs only)
- State machine transitions (draft → in_progress → verify → complete) — guarded transitions belong to S02
- Any `assay plan` wizard functionality — belongs to S03
- PR creation or branch naming — belongs to S04
- Plugin skill integration (Claude Code, Codex) — belongs to S05/S06

## Notes for Tester

The `just ready` check is the definitive verification surface — it runs fmt, clippy, all workspace tests, and cargo deny in sequence. If `just ready` exits 0, S01 is complete.

Note: `cargo test -p assay-core --test milestone_io` (without `--features assay-types/orchestrate`) will fail with a compile error due to a pre-existing bug in `assay-types/src/manifest.rs` — this is NOT caused by S01 and does not affect `just ready` or workspace-level tests.
