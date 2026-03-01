# Phase 3: Error Types and Domain Model - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish the shared type system and error handling that every downstream crate depends on. Types are pure data (DTOs) in `assay-types` — no business logic. Covers: `AssayError`, `Result<T>`, `GateKind`, `GateResult`, and `Criterion`.

</domain>

<decisions>
## Implementation Decisions

### Error experience
- Full context chain — every layer adds context (e.g. "Failed to load config at .assay/config.toml: invalid TOML at line 12: expected string, found integer")
- Strictly add-as-consumed — only `Io` variant now; each downstream phase adds its own variants when consumed
- `#[non_exhaustive]` on AssayError so new variants don't break downstream

### Evidence capture (GateResult)
- GateResult includes a `kind` field so consumers know HOW the gate was evaluated (command, file-exists, always-pass) — agents can distinguish result origins without needing spec context

### Claude's Discretion
- **Error structure**: Whether errors carry structured fields (path, line as typed fields) vs formatted strings — decide based on what CLI and MCP consumers actually need
- **Error presentation**: Whether CLI and MCP get different error formats, or a single thiserror Display serves both — decide based on v0.1 practicality
- **Stdout/stderr capture limits**: Full capture vs configurable truncation — decide sensible v0.1 default
- **Output type**: String (UTF-8 lossy) vs Vec<u8> for stdout/stderr — decide based on JSON serialization needs
- **Timestamp format**: ISO 8601 vs epoch millis for GateResult — decide based on dual CLI/MCP audience
- **Criterion kind modeling**: Single struct with optional cmd vs enum-based distinction — decide what's cleanest for downstream
- **Prompt field**: Reserve `prompt: Option<String>` now vs document-only — decide based on serde forward-compatibility
- **Criteria severity**: All-equal pass/fail vs optional severity levels — decide what's appropriate for v0.1 scope
- **Criterion naming**: Freeform-unique vs constrained slug format — decide what works for both human authoring and programmatic access

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. User's main concern is full error context chains so failures are diagnosable.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-error-types-and-domain-model*
*Context gathered: 2026-03-01*
