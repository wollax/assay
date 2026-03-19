# S02: Development Cycle State Machine — UAT

**Milestone:** M005
**Written:** 2026-03-19

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All cycle state machine behavior is exercised by 10 integration tests with real file I/O and real gate subprocess evaluation. The MCP tools are verified by presence tests confirming router registration. CLI subcommands are verified by tests asserting exact output strings and exit codes. No human interaction is required to validate the state machine logic — behavior is fully deterministic and observable by inspection of TOML files and command output.

## Preconditions

- Rust toolchain installed (`cargo`)
- A project with `.assay/milestones/` directory
- At least one spec in `.assay/specs/<slug>/gates.toml` for `cycle_advance` testing
- For MCP tool testing: `assay-mcp` server running or MCP client connected

## Smoke Test

Run `cargo test -p assay-core --features assay-types/orchestrate --test cycle` from the project root. All 10 tests should pass in under 1 second. This confirms the full state machine (status, advance, transitions) is working end-to-end with real file I/O.

## Test Cases

### 1. cycle_status returns None with no milestones

1. Ensure `.assay/milestones/` is empty (or does not exist)
2. Call MCP tool `cycle_status` with `{}` params
3. **Expected:** response is `"null"` (no active milestone)

### 2. cycle_status returns progress for InProgress milestone

1. Create a milestone TOML in `.assay/milestones/my-feature.toml` with `status = "in_progress"` and 2 chunks, `completed_chunks = []`
2. Call `cycle_status` via MCP
3. **Expected:** JSON with `milestone_slug: "my-feature"`, `phase: "InProgress"`, `completed_count: 0`, `total_count: 2`, `active_chunk_slug` set to the lowest-order chunk slug

### 3. assay milestone status prints progress table

1. Create an InProgress milestone with 2 chunks; one in `completed_chunks`
2. Run `assay milestone status`
3. **Expected:** output contains `MILESTONE: <slug> (InProgress)`, `[x] <completed-chunk-slug>  (complete)`, `[ ] <active-chunk-slug>  (active)`

### 4. cycle_advance marks chunk complete and saves atomically

1. Create an InProgress milestone with 2 chunks
2. Create a spec for the active chunk with a passing gate criterion (`cat /dev/null` or similar)
3. Call `cycle_advance` via MCP (or `assay milestone advance`)
4. **Expected:** response contains `completed_count: 1`; `cat .assay/milestones/<slug>.toml` shows `completed_chunks = ["<chunk-slug>"]`

### 5. cycle_advance transitions to Verify when last chunk completes

1. Create an InProgress milestone with 1 chunk, `completed_chunks = []`
2. Create a passing spec for that chunk
3. Call `cycle_advance` (MCP or CLI)
4. **Expected:** response `phase: "Verify"`; `.assay/milestones/<slug>.toml` shows `status = "verify"` and `completed_chunks = ["<chunk-slug>"]`

### 6. cycle_advance rejects when required gates fail

1. Create an InProgress milestone with 1 chunk
2. Create a spec for that chunk with a failing criterion (`exit 1`)
3. Call `cycle_advance`
4. **Expected:** error response (MCP `isError: true` or CLI exit code 1 with stderr message containing "gates failed")
5. **Expected:** `cat .assay/milestones/<slug>.toml` shows `completed_chunks = []` — milestone unchanged

### 7. chunk_status returns gate run summary

1. Run `assay gate run <chunk-slug>` to produce a history entry
2. Call `chunk_status` MCP tool with `{ "chunk_slug": "<chunk-slug>" }`
3. **Expected:** `{ "has_history": true, "latest_run_id": "...", "passed": N, "failed": M, "required_failed": K }`

### 8. chunk_status returns has_history: false for new chunk

1. Call `chunk_status` for a chunk slug with no gate history
2. **Expected:** `{ "has_history": false }`

## Edge Cases

### Invalid milestone phase transition

1. Create a milestone with `status = "verify"` 
2. Try to call `cycle_advance` (no active chunk)
3. **Expected:** error containing "invalid milestone phase transition" or "already in Verify"

### assay milestone advance — no active milestone

1. Ensure no milestones exist in `.assay/milestones/`
2. Run `assay milestone advance`
3. **Expected:** exit code 1; stderr contains "Error:" with descriptive message about no active milestone

### assay milestone status — no InProgress milestones

1. Ensure all milestones are `draft` or `complete`
2. Run `assay milestone status`
3. **Expected:** output "No active milestones." and exit code 0

## Failure Signals

- `cycle_advance` returns success but `completed_chunks` not updated in TOML — atomic save failed or wrong milestone targeted
- `assay milestone status` shows all chunks as `[ ]` even after advancement — `completed_chunks` not persisted
- `chunk_status` returns `has_history: true` but `passed`/`failed` are null — history record format changed
- `cycle_status` returns a non-null result for a Draft milestone — status filtering in `cycle_status` is broken

## Requirements Proved By This UAT

- R043 (Development cycle state machine) — guarded phase transitions (Draft→InProgress→Verify→Complete), `cycle_advance` gate-pass precondition, milestone state persistence proven by integration tests and TOML inspection
- R044 (Cycle MCP tools) — `cycle_status`, `cycle_advance`, `chunk_status` all registered in router and returning correct structured JSON; failure paths return structured domain errors

## Not Proven By This UAT

- R042 (Guided authoring wizard) — wizard creates milestones in S03; this slice only consumes already-created milestones
- R045 (Gate-gated PR creation) — `pr_create` MCP tool and `assay pr create` CLI are S04 deliverables
- Real agent workflow using all three MCP tools together — the skills that chain `cycle_status` + `chunk_status` + `cycle_advance` are S05/S06 plugin deliverables
- Multi-milestone disambiguation when multiple milestones are simultaneously InProgress — `cycle_status` and `cycle_advance` select the first alphabetically; this behavior is not tested with competing milestones

## Notes for Tester

- `cycle_advance` requires real spec files (with `[[criteria]]` entries and `cmd` fields) in `.assay/specs/<chunk-slug>/gates.toml` to run gates. A minimal passing spec has `name = "<slug>"` at root and `[[criteria]]` with `description = "ok"` and `cmd = "true"`.
- The `completed_chunks` field is not written to TOML when empty (serde `skip_serializing_if = "Vec::is_empty"`). After advancement, the field appears in the TOML. This is expected behavior.
- `chunk_status` reads history from `.assay/history/<slug>/`. History records are only created by `gate_run` — you must run gates first before `chunk_status` returns `has_history: true`.
