---
estimated_steps: 8
estimated_files: 7
---

# T01: Rename AgentSession → GateEvalContext across the codebase

**Slice:** S01 — Prerequisites — Persistence & Rename
**Milestone:** M001

## Description

Global rename of the `AgentSession` type to `GateEvalContext` across all three crates (assay-types, assay-core, assay-mcp). This is a mechanical rename that touches type definitions, re-exports, imports, doc comments, schema registry entries, schema snapshot tests, and MCP tool descriptions. Field names in MCP params (`session_id`) are preserved per D005 (additive only, no signature changes). Also adds `#[serde(deny_unknown_fields)]` to GateEvalContext to match project convention.

## Steps

1. In `crates/assay-types/src/session.rs`: rename `pub struct AgentSession` → `pub struct GateEvalContext`, add `#[serde(deny_unknown_fields)]` attribute, update doc comment, update schema registry entry from `"agent-session"` to `"gate-eval-context"`, update all test references.
2. In `crates/assay-types/src/lib.rs`: update the `pub use session::AgentSession` re-export to `pub use session::GateEvalContext`.
3. In `crates/assay-types/src/work_session.rs`: update the doc comment reference to `AgentSession` → `GateEvalContext`.
4. In `crates/assay-core/src/gate/session.rs`: update all imports and function signatures/bodies from `AgentSession` to `GateEvalContext`.
5. In `crates/assay-mcp/src/server.rs`: update imports, type annotations (`HashMap<String, AgentSession>` → `HashMap<String, GateEvalContext>`), and description strings. Keep `session_id` field names unchanged (D005).
6. In `crates/assay-types/tests/schema_snapshots.rs`: rename the test function from `agent_session_schema_snapshot` to `gate_eval_context_schema_snapshot`, update the type reference, update the snapshot name.
7. Delete old snapshot file `crates/assay-types/tests/snapshots/schema_snapshots__agent-session-schema.snap`. Run `cargo insta test --accept -p assay-types` to generate the new snapshot.
8. Verify: `rg "AgentSession" --type rust crates/` returns zero matches. Run `just ready`.

## Must-Haves

- [ ] Zero occurrences of `AgentSession` in Rust source under `crates/`
- [ ] `#[serde(deny_unknown_fields)]` on GateEvalContext
- [ ] Schema registry entry name is `"gate-eval-context"`
- [ ] Schema snapshot test passes with new name
- [ ] MCP `session_id` field names preserved (D005 compliance)
- [ ] `just ready` passes

## Verification

- `rg "AgentSession" --type rust crates/` returns zero matches
- `just ready` passes (fmt, lint, test, deny)
- `cargo insta test -p assay-types` — no pending snapshots

## Observability Impact

- Signals added/changed: None (rename only, no runtime behavior change)
- How a future agent inspects this: `rg "GateEvalContext"` to find all usage sites
- Failure state exposed: None

## Inputs

- `crates/assay-types/src/session.rs` — current AgentSession type definition
- `crates/assay-core/src/gate/session.rs` — session lifecycle functions using AgentSession
- `crates/assay-mcp/src/server.rs` — MCP server with in-memory HashMap<String, AgentSession>
- D005 (additive-only MCP tools) and D006 (vocabulary rename) from DECISIONS.md

## Expected Output

- `crates/assay-types/src/session.rs` — GateEvalContext type with deny_unknown_fields
- `crates/assay-types/src/lib.rs` — updated re-export
- `crates/assay-types/src/work_session.rs` — updated doc comment
- `crates/assay-types/tests/schema_snapshots.rs` — renamed test
- `crates/assay-types/tests/snapshots/schema_snapshots__gate-eval-context-schema.snap` — new snapshot file
- `crates/assay-core/src/gate/session.rs` — all references updated
- `crates/assay-mcp/src/server.rs` — all references updated, field names preserved
