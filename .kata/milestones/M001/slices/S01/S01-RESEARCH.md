# S01: Prerequisites — Persistence & Rename — Research

**Date:** 2026-03-16

## Summary

S01 covers two requirements: R001 (GateEvalContext persistence to disk) and R002 (AgentSession → GateEvalContext rename). Both are well-scoped mechanical work with clear patterns to follow in the existing codebase.

The persistence work mirrors the existing `work_session.rs` pattern exactly — JSON-per-record under `.assay/gate_sessions/`, atomic tempfile-then-rename writes, `save/load/list` functions. The rename is a find-and-replace across 6 files with cascading updates to schema snapshots, re-exports, and MCP tool descriptions. The MCP server's in-memory `HashMap<String, AgentSession>` stays in-memory (it's an ephemeral evaluation cache) but gains a write-through path to disk so sessions survive restarts.

Risk is medium because the rename touches the MCP server (6060 lines) and schema snapshots, and the persistence changes affect the MCP server's session lifecycle. But both are mechanical and fully testable.

## Recommendation

**Approach:** Do the rename first, then add persistence.

Renaming first avoids writing new persistence code that immediately needs renaming. The rename is a global find-replace of `AgentSession` → `GateEvalContext` across types, core, and MCP crates, plus updating the schema registry entry name, snapshot file, re-exports, and doc comments. Then persistence follows the `work_session.rs` pattern — a new `gate_sessions/` directory under `.assay/`, with `save_context/load_context/list_contexts` functions in `assay-core/src/gate/session.rs`.

The MCP server's `sessions` HashMap becomes a write-through cache: `gate_run` creates + saves, `gate_report` updates + saves, `gate_finalize` loads from HashMap or falls back to disk. On startup, the MCP server can optionally reload in-progress sessions from disk (or leave that for S05 if scope is tight).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic file writes | `tempfile` crate + rename pattern | Already used in `history/mod.rs` and `work_session.rs` — proven crash-safe |
| Run ID generation | `history::generate_run_id()` | Already used by `gate/session.rs::create_session()` — consistent format |
| Path validation | `history::validate_path_component()` | Prevents directory traversal in session IDs — already proven |

## Existing Code and Patterns

- `crates/assay-core/src/work_session.rs` — **Primary pattern to follow.** JSON-per-record under `.assay/sessions/`, atomic writes, `save_session/load_session/list_sessions`. The gate session persistence should mirror this exactly.
- `crates/assay-core/src/history/mod.rs` — Atomic write pattern origin, `validate_path_component()` and `generate_run_id()` utilities. Gate session module already imports `history::generate_run_id`.
- `crates/assay-types/src/session.rs` — `AgentSession` struct (rename target). Has 10 fields, schema registry entry named `"agent-session"`, and comprehensive tests.
- `crates/assay-core/src/gate/session.rs` — Session lifecycle: `create_session`, `report_evaluation`, `build_finalized_record`, `finalize_session`, `finalize_as_timed_out`. All functions take `&AgentSession` or `&mut AgentSession`. This is where persistence functions should be added.
- `crates/assay-mcp/src/server.rs` — `sessions: Arc<Mutex<HashMap<String, AgentSession>>>` at line 800. This is the in-memory store that needs write-through. Functions `gate_run` (creates), `gate_report` (updates), `gate_finalize` (reads + removes) touch it.
- `crates/assay-types/tests/schema_snapshots.rs` — Snapshot test at line 136 for `AgentSession` schema. Must update test name and snapshot file.
- `crates/assay-types/tests/snapshots/schema_snapshots__agent-session-schema.snap` — Schema snapshot file that will need renaming.

## Constraints

- **`deny_unknown_fields` missing on `AgentSession`.** The struct lacks this attribute, violating the project convention. Add it during the rename to `GateEvalContext`. This is a breaking change for any persisted JSON with extra fields — acceptable since no persistence exists yet.
- **Schema registry entry name `"agent-session"` must change to `"gate-eval-context"`.** The inventory-based registry is used for schema generation — consumers may reference by name.
- **MCP tool descriptions reference "AgentSession" in user-facing text** (e.g., `GateReportParams` description at line 114-117). These must update to "GateEvalContext" for vocabulary consistency, but the field name `session_id` in MCP params should stay — it's the wire format (D005: additive only, no signature changes).
- **The `pub use session::AgentSession` re-export in `lib.rs`** means all downstream crates import from `assay_types::AgentSession`. The rename cascades through every import site.
- **`work_session.rs` doc comment references `AgentSession`** (line 118) — must update.
- **History module's `generate_run_id` is `pub(crate)`** — gate/session.rs already imports it. Persistence functions in gate/session.rs can use it directly.
- **Existing `finalize_session` already calls `history::save`** for the final GateRunRecord. The new persistence is for the *in-progress* GateEvalContext, not the finalized record.

## Common Pitfalls

- **Schema snapshot drift** — After renaming, the schema snapshot test and `.snap` file must both be updated. If only the test is updated, `cargo insta review` will show a diff but tests will fail until reviewed. Run `cargo insta review` or `--accept` after the rename.
- **MCP field name confusion** — `GateReportParams.session_id` is a *wire format* field name that MCP tool consumers use. Don't rename this field even though the underlying type is renamed — it would break the MCP API (D005 violation). Only update the description text.
- **Write-through race in MCP server** — The `sessions` HashMap is behind `Arc<Mutex<>>`. Disk writes should happen inside the lock to prevent stale reads. But `serde_json::to_string_pretty` + file I/O inside a mutex lock is fine for single-digit concurrent sessions — don't over-engineer async I/O here.
- **Restart recovery scope** — Full restart recovery (reload all in-progress sessions from disk on MCP server startup) may be more than S01 needs. The minimum for R001 is: sessions persist to disk and can be loaded by ID. Recovery sweep can be deferred if scope is tight.

## Open Risks

- **Schema snapshot acceptance workflow** — The rename will invalidate the `agent-session-schema` snapshot. If `insta` isn't configured to auto-accept, CI will fail until snapshots are reviewed. This is a known workflow step, not a real risk — just don't forget it.
- **MCP server test coverage** — The MCP server has integration tests that create sessions. All of these will need the type rename but should otherwise pass unchanged. If any test hard-codes JSON with `"AgentSession"` as a key (unlikely — the type name isn't serialized), it would break.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | — | No specific skill needed — standard Rust, serde, file I/O |
| schemars | — | Already in use, no skill needed |

No external skills are relevant. This is pure Rust codebase-internal work with no external service dependencies.

## Sources

- `crates/assay-core/src/work_session.rs` — persistence pattern reference (source: codebase)
- `crates/assay-types/src/session.rs` — AgentSession type definition (source: codebase)
- `crates/assay-mcp/src/server.rs` — in-memory session storage (source: codebase)
- `crates/assay-core/src/gate/session.rs` — session lifecycle functions (source: codebase)
- Decision D006, D005, D009 — vocabulary, MCP additive-only, JSON persistence (source: DECISIONS.md)
