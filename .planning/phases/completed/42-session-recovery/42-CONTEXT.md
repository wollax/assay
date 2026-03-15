# Phase 42: Session Recovery & Internal API - Context

**Gathered:** 2026-03-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Startup recovery for stale `agent_running` sessions, and an internal Rust API surface that `gate_evaluate` (Phase 43) will consume instead of MCP round-trips. Recovery handles corrupt files gracefully.

Success criteria (from ROADMAP.md):
1. On startup, `.assay/sessions/` is scanned for stale `agent_running` sessions — each is marked `abandoned` with a recovery note and timestamp
2. `gate_evaluate` calls session management through direct Rust function calls, never MCP round-trips
3. Recovery scan handles corrupt session files gracefully — logs warning, skips file, continues scan

</domain>

<decisions>
## Implementation Decisions

### Recovery trigger and scope
- Recovery runs **automatically on MCP server init** (`AssayServer` construction/startup)
- Only `agent_running` sessions are candidates for recovery
- `Created` is a valid resting state (session waiting to be picked up) — not swept
- `GateEvaluated` is a valid parking state (agent reviewing results) — not swept

### Staleness detection
- **Phase + age threshold** model: a session must be in `agent_running` AND older than the threshold
- Age is measured from the **transition timestamp** into `agent_running` (not `created_at`)
- **Default threshold: 1 hour**, configurable per agent/session
- Threshold is configurable in `assay.toml` — research should investigate the config shape
- Recovery scan **capped at 100 sessions** with oldest-eviction (matches `timed_out_sessions` precedent)

### Recovery notes
- Abandoned sessions include **machine context**: hostname, PID (of the recovering process), and timestamp
- Recovery note format: structured enough to debug why the session was abandoned
- Example: `"Recovered on startup: stale for 3h12m (threshold: 1h). Host: hostname, PID: 12345"`

### Recovery observability
- `tracing::warn!` per recovered session
- `tracing::info!` summary count (e.g., "Recovered 3 stale sessions")
- No MCP response warnings (recovery runs before any tool call)

### Internal API boundary
- Phase 42 **reshapes the existing `assay_core::work_session` API** to be a clean internal surface for Phase 43 — not building new MCP tools
- Introduce **convenience functions** for common Phase 43 workflows (e.g., `start_session` that creates + transitions to `agent_running` + saves atomically)
- Internal API returns **domain types directly** (`WorkSession`, `Result<()>`) — warnings and response shaping are MCP presentation concerns
- Extract the **load→mutate→save pattern** into a reusable helper (e.g., `with_session(id, |session| { ... })`) used by both MCP handlers and `gate_evaluate`
- Exact convenience functions and `with_session` shape need **verification during research** against Phase 43's actual flow requirements

### Claude's Discretion
- Recovery note exact format and fields beyond hostname/PID/timestamp
- `with_session` helper signature and error handling strategy
- Whether convenience functions are separate or composed (e.g., `start_session` vs. builder pattern)
- Config key naming for staleness threshold in `assay.toml`
- Logging levels for edge cases (e.g., session already abandoned, corrupt file)

</decisions>

<specifics>
## Specific Ideas

- Staleness threshold config should be settable per agent/session, not just globally — some agents run long evaluations
- The `with_session` pattern should align with the existing atomic tempfile-then-rename write pattern in `save_session`
- Recovery should be idempotent — running it twice produces the same result (already-abandoned sessions are skipped)

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 42-session-recovery*
*Context gathered: 2026-03-15*
