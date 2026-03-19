# S01: Prerequisites — Persistence & Rename

**Goal:** GateEvalContext (renamed from AgentSession) persists to disk via write-through, and all "AgentSession" references are renamed across the codebase.
**Demo:** After this slice: `just ready` passes, a GateEvalContext round-trips through save/load/list, and zero occurrences of "AgentSession" remain in source (excluding git history).

## Must-Haves

- AgentSession renamed to GateEvalContext in assay-types, assay-core, and assay-mcp
- Schema registry entry renamed from "agent-session" to "gate-eval-context"
- Schema snapshot file renamed and content updated
- `deny_unknown_fields` added to GateEvalContext
- MCP tool descriptions updated (field names like `session_id` preserved per D005)
- `save_context()`, `load_context()`, `list_contexts()` functions in `assay-core/src/gate/session.rs`
- Persistence uses atomic tempfile-then-rename pattern (matching `work_session.rs`)
- Storage under `.assay/gate_sessions/<run_id>.json`
- MCP server write-through: gate_run saves, gate_report saves, gate_finalize loads fallback
- `just ready` passes (fmt + lint + test + deny)

## Proof Level

- This slice proves: contract (persistence round-trip, rename completeness, schema correctness)
- Real runtime required: no (unit/integration tests with tempdir are sufficient)
- Human/UAT required: no

## Verification

- `just ready` — full check suite passes (fmt, lint, test, deny)
- `rg "AgentSession" --type rust crates/` returns zero matches
- Persistence round-trip test: save a GateEvalContext, load it by ID, assert equality
- List test: save multiple contexts, list returns all IDs sorted
- Schema snapshot test passes with renamed snapshot file
- MCP server compiles with write-through changes

## Observability / Diagnostics

- Runtime signals: persistence errors surface through `AssayError` variants with path context
- Inspection surfaces: `.assay/gate_sessions/*.json` files are human-readable pretty-printed JSON; `list_contexts()` enumerates all persisted sessions
- Failure visibility: save/load errors include the file path, session ID, and IO/JSON error cause
- Redaction constraints: none (no secrets in gate eval contexts)

## Integration Closure

- Upstream surfaces consumed: none (first slice)
- New wiring introduced in this slice: `save_context`/`load_context`/`list_contexts` in gate/session.rs; MCP server write-through calls in `gate_run`/`gate_report`/`gate_finalize`
- What remains before the milestone is truly usable end-to-end: S02 (harness crate + profile type), S03 (prompt/settings/hooks), S04 (Claude adapter), S05 (worktree enhancements), S06 (manifest), S07 (pipeline)

## Tasks

- [x] **T01: Rename AgentSession → GateEvalContext across the codebase** `est:45m`
  - Why: R002 requires vocabulary cleanup before adding new types. Renaming first avoids writing persistence code that immediately needs renaming.
  - Files: `crates/assay-types/src/session.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-types/src/work_session.rs`, `crates/assay-types/tests/schema_snapshots.rs`, `crates/assay-types/tests/snapshots/schema_snapshots__agent-session-schema.snap`, `crates/assay-core/src/gate/session.rs`, `crates/assay-mcp/src/server.rs`
  - Do: (1) Rename struct `AgentSession` → `GateEvalContext` in session.rs, add `#[serde(deny_unknown_fields)]`. (2) Update schema registry entry name to "gate-eval-context". (3) Update re-export in lib.rs. (4) Update all imports in gate/session.rs. (5) Update all imports/usages in server.rs (type refs and description strings only — keep `session_id` field names per D005). (6) Update doc comment in work_session.rs. (7) Rename schema snapshot test function and file, run `cargo insta test --accept` to regenerate. (8) Verify `rg "AgentSession" --type rust crates/` returns zero matches.
  - Verify: `just ready` passes; `rg "AgentSession" --type rust crates/` returns zero matches
  - Done when: zero "AgentSession" occurrences in Rust source; all tests pass including schema snapshot

- [x] **T02: Add GateEvalContext persistence functions and tests** `est:45m`
  - Why: R001 requires GateEvalContext to persist to disk, surviving MCP restarts. The `work_session.rs` pattern provides a proven template.
  - Files: `crates/assay-core/src/gate/session.rs`, `crates/assay-core/src/error.rs`
  - Do: (1) Add `GateEvalContextNotFound` error variant to AssayError (mirroring `WorkSessionNotFound`). (2) Implement `save_context(assay_dir, context) -> Result<PathBuf>` — creates `.assay/gate_sessions/` dir, validates session_id via `history::validate_path_component`, atomic tempfile-then-rename write. (3) Implement `load_context(assay_dir, session_id) -> Result<GateEvalContext>` — validates ID, reads JSON, returns typed error on not-found. (4) Implement `list_contexts(assay_dir) -> Result<Vec<String>>` — lists `.json` files in `gate_sessions/`, returns sorted IDs. (5) Add tests: round-trip save/load, list empty dir, list returns sorted, path traversal rejection, not-found error, atomic write creates directory.
  - Verify: `cargo test -p assay-core -- gate::session` passes with all new tests green
  - Done when: save/load/list functions exist with full test coverage; `just ready` passes

- [x] **T03: Wire MCP server write-through persistence** `est:30m`
  - Why: The MCP server's in-memory HashMap must write through to disk so sessions survive restarts. This completes R001's persistence requirement at the integration level.
  - Files: `crates/assay-mcp/src/server.rs`
  - Do: (1) In `gate_run`: after inserting GateEvalContext into HashMap, call `save_context()` inside the lock. (2) In `gate_report`: after updating the context with `report_evaluation`, call `save_context()` inside the lock. (3) In `gate_finalize`: before removing from HashMap, add disk-fallback — if not in HashMap, try `load_context()` from disk. After finalization, delete the on-disk file (best-effort, log warning on failure). (4) Ensure `assay_dir` path is available in the server context (check how it's already threaded). (5) Keep existing error handling — persistence failures should log warnings but not block the MCP response for gate_run/gate_report (the in-memory path still works).
  - Verify: `cargo test -p assay-mcp` passes; `cargo build -p assay-mcp` compiles clean
  - Done when: MCP server compiles with write-through; gate_run/gate_report save to disk; gate_finalize falls back to disk load; `just ready` passes

## Files Likely Touched

- `crates/assay-types/src/session.rs`
- `crates/assay-types/src/lib.rs`
- `crates/assay-types/src/work_session.rs`
- `crates/assay-types/tests/schema_snapshots.rs`
- `crates/assay-types/tests/snapshots/schema_snapshots__agent-session-schema.snap`
- `crates/assay-core/src/gate/session.rs`
- `crates/assay-core/src/error.rs`
- `crates/assay-mcp/src/server.rs`
