# Phase 13: Enforcement Levels - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Add required/advisory enforcement to criteria so gate evaluation distinguishes blocking failures from informational warnings. Requirements ENFC-01 and ENFC-02 from the roadmap.

</domain>

<decisions>
## Implementation Decisions

### Pass/fail semantics
- Binary `passed: bool` stays — no third state. Advisory failures don't affect the boolean.
- `passed = (required_passed == required_total)` — strict formula. Skipped required criteria block.
- Skipped criteria are always advisory regardless of declared enforcement — they go in the skipped bucket only, never in enforcement counts.
- A gate with zero required criteria is a parse-time error — prevents toothless gates.

### Spec file UX
- Field name: `enforcement` (values: `"required"`, `"advisory"`)
- Spec-level default with per-criterion override via a new `[gate]` section:
  ```toml
  [gate]
  enforcement = "required"  # default for all criteria

  [[criteria]]
  name = "lint"
  cmd = "cargo clippy"
  enforcement = "advisory"  # overrides gate default
  ```
- `[gate]` section is optional for existing specs (backward compat) but required for new specs.
- Specs without enforcement field default to `required` but emit a deprecation warning.

### Result reporting shape
- `GateRunSummary` gets a nested `enforcement: EnforcementSummary` struct (always present, counts default to 0):
  ```
  EnforcementSummary {
    required_passed: usize,
    required_failed: usize,
    advisory_passed: usize,
    advisory_failed: usize,
  }
  ```
- Existing `passed_count`/`failed_count`/`skipped_count` fields remain for backward compat.
- Each `CriterionResult` carries an `enforcement: Enforcement` enum field (Required | Advisory).

### Edge cases & defaults
- `Enforcement` enum is strict: only lowercase `"required"` and `"advisory"` accepted. Parse errors include helpful suggestion of closest valid value.
- "At least one required criterion" validation happens at parse time, not evaluation time.
- Skipped criteria (no cmd, no path) go into `skipped_count` only — they don't appear in any enforcement bucket, regardless of their declared enforcement.

### Claude's Discretion
- Exact deprecation warning message text
- Internal helper functions for computing enforcement summary from results
- Whether `EnforcementSummary` implements `Default` or has a `new()` constructor
- Test fixture organization

</decisions>

<specifics>
## Specific Ideas

- The `[gate]` section separates gate configuration from spec metadata — follows TOML convention of grouping related fields.
- The strict pass formula (`required_passed == required_total`) was chosen intentionally to make skipped required criteria blocking — users should fix their specs, not silently pass.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 13-enforcement-levels*
*Context gathered: 2026-03-04*
