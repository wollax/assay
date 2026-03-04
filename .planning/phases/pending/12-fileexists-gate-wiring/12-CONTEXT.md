# Phase 12: FileExists Gate Wiring - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Connect the existing `FileExists` gate kind to the evaluation dispatch so it produces real results instead of being dead code. A spec with a `FileExists` criterion should evaluate to passed/failed with evidence. No new gate kinds or capabilities — purely wiring what already exists.

</domain>

<decisions>
## Implementation Decisions

### Evidence content
- Claude's discretion on what evidence a passing check includes (path only vs path + metadata) — decide based on consistency with how Command gates report evidence
- Claude's discretion on whether resolved path appears in both pass and fail evidence
- Claude's discretion on whether only regular files count or any path type (files, directories, symlinks)

### Path resolution
- Claude's discretion on resolution base (working directory vs project root) — decide based on consistency with Command gate path behavior
- Claude's discretion on whether absolute paths are allowed or only relative
- Claude's discretion on symlink following behavior (standard fs::metadata vs fs::symlink_metadata)
- Claude's discretion on environment variable expansion — decide based on phase scope appropriateness

### Failure reason format
- Claude's discretion on verbosity level — decide based on what's most useful for debugging
- Claude's discretion on permission error handling (pass because it exists, or fail because unverifiable)
- Claude's discretion on differentiating "file not found" vs "parent directory not found"

### Claude's Discretion
All three areas were delegated to Claude's judgment. The guiding principles are:
- **Consistency** — match behavior of existing Command gates wherever possible
- **Debugging value** — failure messages should help users fix their specs
- **Simplicity** — this is a wiring phase, not a feature-rich file checker

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. User trusts Claude to make pragmatic decisions consistent with the existing gate evaluation patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 12-fileexists-gate-wiring*
*Context gathered: 2026-03-04*
