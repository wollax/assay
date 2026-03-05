# Phase 14: Run History Core - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Persist gate run results to disk as JSON files with atomic writes, providing the audit trail infrastructure for all downstream surfaces. This phase covers saving, loading, and listing run records. Retention policy enforcement is Phase 15. Agent-submitted evaluations are Phase 16.

</domain>

<decisions>
## Implementation Decisions

### Record format & content
- Full detail: GateRunRecord includes complete GateRunSummary with all CriterionResults, evidence, enforcement levels, and timing
- Strict serde: GateRunRecord uses `deny_unknown_fields` — records are versioned artifacts, mismatches should be caught immediately
- assay_version included for schema migration support

### File naming & directory layout
- ISO-ish compact timestamp filenames: `20260304T223015Z.json` (UTC, sortable, no special chars)
- Auto-create `.assay/results/<spec>/` on first save — zero setup friction, `mkdir -p` the spec subdirectory
- Spec subdirectory names are sanitized/slugified — handle edge cases in spec names

### Concurrency & failure modes
- Just append new files — don't validate or inspect existing directory contents
- Atomic write via tempfile-then-rename pattern

### Integration surface
- Save/load/list logic lives in `assay-core` (new `history` module or similar)
- Expose write + read/list API in this phase — Phase 15 (CLI history) and Phase 17 (MCP history) need read access, building it now avoids rework
- One file per gate evaluation call

### Claude's Discretion
- Environment metadata beyond assay_version (working dir, hostname, git ref) — include what seems useful without over-engineering
- Whether spec name is embedded in the record or implied by directory — decide based on downstream consumer ergonomics
- Whether GateRunRecord wraps GateRunSummary or is a standalone type — decide based on type ergonomics and serde
- Whether evaluate() auto-saves or callers save explicitly — decide based on testability and coupling
- Save failure behavior (warn vs fail the run) — decide based on which consumers depend on persistence guarantees
- Concurrent write uniqueness strategy (timestamp-only vs timestamp+random suffix) — decide based on existing run ID format decisions
- Temp file location for atomic writes — decide based on atomicity guarantees
- File granularity (one file per run vs per gate) — decide based on current spec/gate model

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The existing brainstorm decisions provide guidance:
- Timestamp + 6-char random hex suffix for run IDs (no new crate)
- Per-spec subdirectory layout: `.assay/results/{spec-name}/`
- Include assay_version in GateRunRecord for future schema migration

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 14-run-history-core*
*Context gathered: 2026-03-04*
