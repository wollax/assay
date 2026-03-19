# S01: Milestone & Chunk Type Foundation — Research

**Researched:** 2026-03-19
**Domain:** Rust type design, TOML persistence, MCP tool registration
**Confidence:** HIGH

## Summary

S01 establishes the foundational data model for the milestone/chunk hierarchy. Three areas of work are involved: (1) adding backward-compatible `milestone` and `order` fields to `GatesSpec` in `assay-types`, (2) creating new `Milestone`, `ChunkRef`, and `MilestoneStatus` types in `assay-types`, and (3) adding `milestone_load()`, `milestone_save()`, and `milestone_scan()` functions in `assay-core`, plus two MCP tools (`milestone_list`, `milestone_get`) in `assay-mcp`.

The codebase has strong established patterns for all three areas. The biggest risk is the `deny_unknown_fields` attribute on `GatesSpec` — adding new fields requires removing that attribute or switching to `#[serde(default)]` on the new fields while keeping the struct attribute. The existing behavior is that `deny_unknown_fields` rejects any unknown TOML key, so new optional fields with `serde(default)` are fine as long as they are defined on the struct — they are not "unknown". This is confirmed safe: serde's `deny_unknown_fields` only rejects fields present in the input but absent from the struct. Adding fields to the struct that have `serde(default)` allows existing TOML files (which omit those fields) to continue parsing correctly.

The milestone TOML format follows D062 (`.assay/milestones/<slug>.toml`). The file I/O pattern is the atomic tempfile-rename from `assay-core::history` and `assay-core::work_session`. The MCP tool registration uses `#[tool_router]` + `#[tool]` macros from the `rmcp` crate, with all tools on the `AssayServer` impl block.

## Recommendation

Follow existing patterns exactly: `GatesSpec` extension with `serde(default, skip_serializing_if = "Option::is_none")` for the two new fields. New `Milestone` type with `deny_unknown_fields`, TOML round-trip, inventory registration, and schema snapshot. File I/O in `assay-core::milestone` module mirroring `assay-core::history` (atomic tempfile-rename). MCP tools added to `AssayServer` as additional `#[tool]` methods.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic TOML/JSON file write | `tempfile::NamedTempFile` + `persist()` pattern in `history/mod.rs` and `work_session.rs` | Crash-safe; proven across all persistence in the codebase |
| MCP tool registration | `#[tool_router]` + `#[tool]` macros from `rmcp` crate | All existing tools use this; schema auto-generation included |
| Schema snapshot locking | `inventory::submit!` + `SchemaEntry` in `schema_registry.rs` | All types use this; required for CI schema checks |
| Path component safety | `history::validate_path_component()` | Already handles traversal prevention; reuse for milestone slugs |

## Existing Code and Patterns

- `crates/assay-types/src/gates_spec.rs` — **Target for extension**: add `milestone: Option<String>` and `order: Option<u32>` fields. Has `deny_unknown_fields` — new fields on the struct are NOT unknown, so backward compat is preserved. Existing tests must all pass unchanged.
- `crates/assay-core/src/history/mod.rs` — **Primary pattern for milestone I/O**: atomic `NamedTempFile` + `persist()`, `validate_path_component()`, `generate_run_id()`. Reuse for `milestone_save()`.
- `crates/assay-core/src/work_session.rs` — **Secondary pattern**: `save_session()` shows exact tempfile-rename sequence with `sync_all()` before persist. Copy this pattern for `milestone_save()`.
- `crates/assay-core/src/spec/mod.rs` — **Pattern for scan + load**: `scan()`, `load_spec_entry()`, `load_gates()`. Milestone scan follows same flat-directory approach: one file per slug.
- `crates/assay-mcp/src/server.rs` — **MCP registration**: `#[tool_router]` on impl block, `#[tool(description = "...")]` on each method. `AssayServer` struct holds `tool_router: ToolRouter<Self>`. New milestone tools add as additional methods. Tool count: currently 22 tools registered.
- `crates/assay-types/src/schema_registry.rs` — **Schema registry**: `inventory::submit! { SchemaEntry { name: "...", generate: || schemars::schema_for!(Type) } }`. Every new type needs this.
- `crates/assay-types/src/lib.rs` — **Public re-exports**: new milestone types must be re-exported from the crate root like all other types.
- `crates/assay-types/src/orchestrate.rs` — **Feature gate pattern**: if milestone types need feature-gating, this is the model. However, D064 says milestone module is in `assay-core` without feature gating — types go in `assay-types` (no feature gate needed for the core types).

## Constraints

- `GatesSpec` has `#[serde(deny_unknown_fields)]`. The new `milestone` and `order` fields MUST be added to the struct (not just a wrapper), and MUST have `#[serde(default, skip_serializing_if = "Option::is_none")]`. This preserves backward compat: existing TOML files omitting these fields parse fine; the struct's `deny_unknown_fields` only blocks fields in the file that aren't on the struct.
- All 1271+ existing tests must pass unchanged after the `GatesSpec` extension. The existing test `gates_spec_rejects_unknown_fields` specifically tests that unknown fields in TOML are rejected — this test remains valid and must pass.
- Milestone slugs come from file names (like spec slugs). The `validate_path_component()` helper in `history/mod.rs` is `pub(crate)` — it will need to be made more accessible or duplicated in the milestone module.
- `toml` crate is a workspace dependency available to `assay-core` and `assay-types`. `assay-types` lists `toml` only in `dev-dependencies`. The milestone types themselves don't need TOML in their own crate — serialization/deserialization happens in `assay-core` which already has `toml.workspace = true` as a regular dep.
- MCP tools must be additive (D005). No modification to existing tool signatures.
- The `assay-core::milestone` module does not need an `orchestrate` feature gate — it's always-on core functionality.

## Common Pitfalls

- **`deny_unknown_fields` + `serde(default)` confusion** — `deny_unknown_fields` rejects fields present in the TOML that aren't on the Rust struct. It does NOT prevent adding fields to the Rust struct with `serde(default)`. Adding `milestone` and `order` to the struct with `serde(default)` is safe. The test `gates_spec_rejects_unknown_fields` remains correct because it tests the rejection of truly unknown TOML keys (e.g., `unknown = "oops"`), not the new legitimate fields.
- **Schema snapshot drift** — Every new type with `inventory::submit! { SchemaEntry { ... } }` requires running `cargo insta review` to accept the new snapshots. Plan for this as an explicit task step.
- **TOML serialization order** — `toml::to_string()` serializes fields in struct declaration order. Put `milestone` and `order` after the existing `depends` field and before `criteria` for a natural reading order in generated files.
- **`validate_path_component` visibility** — Currently `pub(crate)` in `assay-core::history`. For `milestone_save()` in `assay-core::milestone`, either move it to `assay-core` module root as `pub(crate)` or re-export it. Do NOT duplicate.
- **MilestoneStatus state machine** — The `MilestoneStatus` enum must be serialized case-sensitively to match what `cycle.rs` in S02 will use. Use `#[serde(rename_all = "snake_case")]` for TOML friendliness (matching existing enum patterns in orchestrate.rs).

## Open Risks

- **Schema snapshot count**: Adding ~3 new types means ~3 new schema snapshots. CI will fail until `cargo insta review` is run. This is expected and not a blocker, but the plan must include the snapshot acceptance step.
- **`assay-cli` milestone subcommand stub**: S01 boundary map says `assay milestone list` is an output. The CLI command scaffolding needs to exist in S01 even if the full implementation belongs to S02. Confirm scope: S01 boundary map shows the CLI command is implied by "assay milestone list shows milestones" in the slice demo — may need a minimal `milestone list` command in `assay-cli` even if only S02 fills it out fully.
- **Milestone scan directory creation**: `.assay/milestones/` may not exist in existing projects. `milestone_scan()` must handle missing directory gracefully (return empty list, not error) since existing projects have no milestones yet.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (general) | n/a | None needed — codebase patterns are sufficient |

## Sources

- `crates/assay-types/src/gates_spec.rs` — Existing GatesSpec structure and tests (codebase direct read)
- `crates/assay-core/src/history/mod.rs` — Atomic write pattern and path validation (codebase direct read)
- `crates/assay-core/src/work_session.rs` — `save_session()` tempfile pattern (codebase direct read)
- `crates/assay-core/src/spec/mod.rs` — `scan()` and `load_spec_entry()` patterns (codebase direct read)
- `crates/assay-mcp/src/server.rs` — `#[tool_router]`, `#[tool]`, `AssayServer` structure (codebase direct read)
- `crates/assay-types/src/lib.rs` — Type export and feature gate patterns (codebase direct read)
- `.kata/DECISIONS.md` — D062 (milestone TOML format), D063 (chunk=spec), D064 (milestone module location), D065 (gh CLI for PR), D066 (dialoguer), D067 (MCP tool naming) — preloaded context
