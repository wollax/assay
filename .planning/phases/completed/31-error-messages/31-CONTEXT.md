# Phase 31: Error Messages - Context

**Gathered:** 2026-03-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Make all error messages actionable — command-not-found errors name the missing binary, spec-not-found errors list available specs, and TOML parse errors include file path and line number. Requirements: ERR-01, ERR-02, ERR-03. Improvements apply to both CLI and MCP surfaces.

</domain>

<decisions>
## Implementation Decisions

### Command-not-found error differentiation (ERR-01)
- Branch on `io::ErrorKind` from the spawn failure:
  - `NotFound` → "Command 'X' not found. Is it installed and in PATH?" (exact wording is illustrative — match the intent)
  - `PermissionDenied` → "Command 'X' found but not executable. Check file permissions."
  - Everything else → current generic gate execution error with the IO message
- Extract the binary name (first token) from multi-word `cmd` strings for the error message
- Drop the working directory from the NotFound error message (noise for missing-binary case)
- During `gate run --all`, print the improved message and continue to the next criterion (same pattern as today)
- Same improved messages in MCP surface — no CLI-only behavior

### Spec-not-found diagnostics (ERR-02)
- Inline format: `spec 'auth-flow' not found. Available specs: login, signup, billing`
- Truncate at 10 specs with count of remaining; user runs `assay spec list` for the full list
- Zero specs: `"No specs found in .assay/specs/."`  (fact only, no hint)
- Specs that failed to parse appear in the list with a marker: `billing (invalid)` AND a separate warning about unparseable specs
- Single fuzzy/Levenshtein match for typos: `Did you mean 'auth-flow'?` — only suggest if there's one close match
- Same behavior in MCP surface

### TOML parse error enrichment (ERR-03)
- Primary goal: ensure file path is always prominently shown in the error
- Show the offending source line with a caret pointer at the error column
- Truncate displayed source lines longer than ~80 chars, showing context around the error column
- Index into the already-loaded file content string (no re-read)
- For directory-based specs, make clear which file within the spec directory failed (gates.toml vs spec.toml)
- Review validation errors (valid TOML, invalid schema) — fix file path display if inconsistent, leave alone if already good
- Apply TOML error improvements to config file parsing too (`.assay/config.toml`), not just spec files

### Claude's Discretion
- Exact Levenshtein distance threshold for fuzzy matching
- Caret pointer formatting details (e.g., `^^^` vs `^`)
- Whether to use colored output for error emphasis (respecting NO_COLOR)
- How to format the truncated source line (ellipsis placement)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Error messages should be clear and actionable, following Rust CLI conventions (similar to cargo/rustc error formatting for TOML errors).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 31-error-messages*
*Context gathered: 2026-03-10*
