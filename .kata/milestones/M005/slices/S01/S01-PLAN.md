# S01: Milestone & Chunk Type Foundation

**Goal:** Establish the data model and file I/O layer for milestones and chunk metadata, then expose the two milestone query MCP tools and a CLI stub — proving the foundation that S02–S06 depend on.
**Demo:** `assay milestone list` prints milestones from `.assay/milestones/`; existing specs with the added `milestone` and `order` fields still pass all gate runs; `milestone_list` and `milestone_get` MCP tools return structured data; `just ready` is green.

## Must-Haves

- `GatesSpec` gains `milestone: Option<String>` and `order: Option<u32>` fields; existing TOML without those fields still parses; `gates_spec_rejects_unknown_fields` still passes
- `Milestone`, `ChunkRef`, and `MilestoneStatus` types exist in `assay-types` with TOML round-trip tests and schema snapshots accepted by `cargo insta review`
- `milestone_load()`, `milestone_save()`, `milestone_scan()` in `assay-core::milestone` with atomic writes, graceful empty-directory handling, and unit tests
- `milestone_list` and `milestone_get` tools registered in `AssayServer` tool router, returning structured JSON; tool-in-router assertions pass
- `assay milestone list` CLI subcommand runs without error and prints milestones (or "no milestones found" for empty directories)
- All 1271+ existing tests pass; `just ready` green

## Proof Level

- This slice proves: **contract** (TOML round-trip, schema snapshots, file I/O unit tests, MCP tool registration) + **integration** (MCP tool → assay-core → filesystem; CLI → assay-core → output)
- Real runtime required: no (unit and integration tests; no agent spawning)
- Human/UAT required: no

## Verification

- `cargo test -p assay-types` — all existing tests pass; new `gates_spec_milestone_fields_*` and `milestone_*` tests pass; schema snapshots accepted
- `cargo test -p assay-core --test milestone_io` — `milestone_load`, `milestone_save`, `milestone_scan` round-trip and edge-case tests pass
- `cargo test -p assay-mcp` — `milestone_list_tool_in_router`, `milestone_get_tool_in_router`, `milestone_list_returns_empty_for_no_milestones`, `milestone_get_returns_error_for_missing_slug` pass
- `cargo test -p assay-cli` — `milestone_list_subcommand_no_milestones` passes
- `just ready` — fmt + lint + test + deny all green

## Observability / Diagnostics

- Runtime signals: `milestone_load` returns `AssayError::Io` with path on parse failure; `milestone_scan` returns `AssayError::Io` on unreadable directory entry; `milestone_save` returns `AssayError::Io` on atomic write failure
- Inspection surfaces: `assay milestone list` CLI surfaces scan errors to stderr; MCP tools surface errors as `isError: true` with message text
- Failure visibility: all error paths return the offending file path and operation label via `AssayError::Io { operation, path, source }`
- Redaction constraints: none (no secrets in milestone data)

## Integration Closure

- Upstream surfaces consumed: `assay-types::GatesSpec`, `assay-core::history::validate_path_component` (crate-internal), atomic tempfile pattern from `assay-core::work_session`
- New wiring introduced in this slice: `assay-core::milestone` → `assay-mcp::server` (milestone tools), `assay-core::milestone` → `assay-cli::commands::milestone` (CLI stub); `assay-types::milestone` re-exported from crate root
- What remains before the milestone is truly usable end-to-end: S02 (cycle state machine, `cycle_status`/`cycle_advance`), S03 (wizard), S04 (PR workflow), S05/S06 (plugins)

## Tasks

- [x] **T01: Define Milestone types in assay-types and extend GatesSpec** `est:1h`
  - Why: Every downstream task (I/O, MCP, CLI) depends on the type definitions; locking the schema here prevents drift. GatesSpec backward-compat is a high-risk item that must be verified first.
  - Files: `crates/assay-types/src/gates_spec.rs`, `crates/assay-types/src/milestone.rs` (new), `crates/assay-types/src/lib.rs`, `crates/assay-types/tests/schema_snapshots.rs`
  - Do: Add `milestone: Option<String>` and `order: Option<u32>` to `GatesSpec` with `#[serde(default, skip_serializing_if = "Option::is_none")]` after the `depends` field and before `criteria`. Create `milestone.rs` with `MilestoneStatus` enum (`draft`, `in_progress`, `verify`, `complete`; `#[serde(rename_all = "snake_case")]`), `ChunkRef` struct (`slug: String`, `order: u32`), and `Milestone` struct (`slug: String`, `name: String`, `description: Option<String>`, `chunks: Vec<ChunkRef>`, `status: MilestoneStatus`, `depends_on: Vec<String>`, `pr_branch: Option<String>`, `pr_base: Option<String>`, `created_at: DateTime<Utc>`, `updated_at: DateTime<Utc>`). Apply `deny_unknown_fields`, `JsonSchema`, `Serialize`/`Deserialize`, `Debug`, `Clone`, `PartialEq`. Register `SchemaEntry` via `inventory::submit!` for all three new types. Re-export `Milestone`, `ChunkRef`, `MilestoneStatus` from `assay-types/src/lib.rs`. Add schema snapshot tests to `schema_snapshots.rs` for all three types plus the updated `GatesSpec`. Run `cargo test -p assay-types` → new snapshots generated → run `cargo insta review` → run `cargo test -p assay-types` again to confirm green.
  - Verify: `cargo test -p assay-types` all pass including `gates_spec_rejects_unknown_fields` (existing test unchanged); snapshot tests for `milestone-schema`, `chunk-ref-schema`, `milestone-status-schema`, and updated `gates-spec-schema` all pass after `cargo insta review`; `cargo test -p assay-types -- gates_spec_milestone_fields` (new tests) pass
  - Done when: `cargo test -p assay-types` is fully green with no pending snapshots; all existing gate spec tests still pass; three new types are re-exported from `assay-types` root

- [x] **T02: Implement milestone_load, milestone_save, milestone_scan in assay-core** `est:1.5h`
  - Why: The I/O layer is the foundation for MCP tools, CLI, and S02 cycle state machine; must be independently tested before wiring into consumers.
  - Files: `crates/assay-core/src/milestone/mod.rs` (new), `crates/assay-core/src/lib.rs`, `crates/assay-core/tests/milestone_io.rs` (new)
  - Do: Create `crates/assay-core/src/milestone/` directory with `mod.rs`. Implement `milestone_scan(assay_dir: &Path) -> Result<Vec<Milestone>>` — returns empty `Vec` when `.assay/milestones/` doesn't exist (not an error); reads each `*.toml` file; derives `slug` from filename stem; sets `milestone.slug` from stem after load; collects errors via `?`-early-return on unreadable entries. Implement `milestone_load(assay_dir: &Path, slug: &str) -> Result<Milestone>` — validates slug using `crate::history::validate_path_component`; reads and parses the TOML file; returns `AssayError::Io` with path on missing file or parse failure. Implement `milestone_save(assay_dir: &Path, milestone: &Milestone) -> Result<()>` — validates slug; creates `.assay/milestones/` dir with `create_dir_all`; writes via `NamedTempFile` + `sync_all()` + `persist()` (atomic pattern from `work_session.rs`); serializes with `toml::to_string`. Register module in `lib.rs` as `pub mod milestone`. Write integration test file `tests/milestone_io.rs` covering: save-then-load round-trip, scan returns empty for missing dir, scan returns all saved milestones, load returns error for missing slug, save validates slug safety.
  - Verify: `cargo test -p assay-core --test milestone_io` all 5+ tests pass; `cargo test -p assay-core` (full suite) still green
  - Done when: `cargo test -p assay-core` fully green; `milestone_scan` returns empty vec (not error) for a project with no `.assay/milestones/` dir; round-trip TOML fidelity proven by test

- [x] **T03: Register milestone_list and milestone_get MCP tools and add assay milestone CLI stub** `est:1.5h`
  - Why: Closes the slice — the demo requires `assay milestone list` to work and MCP tools to be in the router; S02 and S05/S06 depend on these interfaces being present and tested.
  - Files: `crates/assay-mcp/src/server.rs`, `crates/assay-cli/src/commands/milestone.rs` (new), `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/main.rs`
  - Do: In `server.rs`, add `MilestoneListParams` (empty struct, `Deserialize + JsonSchema`) and `MilestoneGetParams { slug: String }`. Add `milestone_list` tool method to `AssayServer` that calls `assay_core::milestone::milestone_scan(&self.assay_dir())` and returns JSON array of milestone slugs + names + status. Add `milestone_get` tool method that calls `assay_core::milestone::milestone_load(&self.assay_dir(), &params.slug)` and returns full JSON for the milestone or `isError: true` when slug is not found. Both tools annotated with `#[tool(description = "...")]`; router picks them up automatically via `#[tool_router]`. Add tests: `milestone_list_tool_in_router`, `milestone_get_tool_in_router`, `milestone_list_returns_empty_json_array_for_no_milestones`, `milestone_get_returns_error_for_missing_slug`. In CLI, create `commands/milestone.rs` with `MilestoneCommand` enum having `List` variant; `execute_milestone` dispatches to `milestone_list_cmd` which calls `assay_core::milestone::milestone_scan`, prints table (slug | name | status) or "No milestones found." to stdout. Register in `commands/mod.rs` and add `Milestone` variant to `Command` enum in `main.rs`. Add CLI test: `milestone_list_subcommand_no_milestones` runs `assay milestone list` in a temp assay dir and asserts exit code 0 and "No milestones found" in stdout.
  - Verify: `cargo test -p assay-mcp -- milestone` all 4 new tests pass; `cargo test -p assay-cli -- milestone` passes; `just ready` green
  - Done when: `just ready` exits 0; `milestone_list` and `milestone_get` appear in the tool router; `assay milestone list` exits 0 in a project with no milestones

## Files Likely Touched

- `crates/assay-types/src/gates_spec.rs`
- `crates/assay-types/src/milestone.rs` (new)
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-core/src/milestone/mod.rs` (new)
- `crates/assay-core/src/lib.rs`
- `crates/assay-core/tests/milestone_io.rs` (new)
- `crates/assay-mcp/src/server.rs`
- `crates/assay-cli/src/commands/milestone.rs` (new)
- `crates/assay-cli/src/commands/mod.rs`
- `crates/assay-cli/src/main.rs`
