# Phase 6: Spec Files - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can write TOML spec files with criteria, and the system can parse, validate, and enumerate them. Includes spec loading (`from_str`, `load`), validation, directory scanning, and CLI commands (`spec show`, `spec list`). Gate evaluation of criteria is Phase 7.

</domain>

<decisions>
## Implementation Decisions

### Spec identity and naming
- Filename is the primary identity for CLI lookup (`assay spec show hello-world` looks for `hello-world.toml`)
- The `name` field inside the TOML is for display/reporting — does not need to match filename
- Recommend kebab-case filenames as convention but don't enforce it
- Duplicate `name` field values across spec files is an error — reject at scan time

### CLI output format
- `assay spec show <name>` displays a table format: header with spec name/description, then criteria table with columns for #, criterion name, type (executable/descriptive), and command
- Terminal colors enabled — differentiate criteria types visually. Respect `NO_COLOR` env var
- `--json` flag on `assay spec show` for machine-readable output (parsed spec as JSON)
- Add `assay spec list` command alongside `spec show` — lists all spec names from specs directory

### Validation behavior
- Accumulate all validation errors and report together (consistent with config's `Vec<ConfigError>` pattern)
- A spec must have at least one criterion — zero criteria is a validation error

### Spec file structure
- Spec struct: `name`, `description`, `criteria: Vec<Criterion>` — no additional fields
- `#[serde(deny_unknown_fields)]` on both `Spec` and `Criterion` types — strict parsing, catches typos
- Existing `[[criteria]]` TOML array-of-tables syntax (already established in example template)

### Claude's Discretion
- Whether to traverse subdirectories in `specs/` or scan flat only
- Warning tier vs errors-only for validation severity
- How scan errors are handled (fail entire scan vs skip invalid + report)
- Error type pattern — dedicated `SpecError` enum mirroring config, or reuse `AssayError` variants

</decisions>

<specifics>
## Specific Ideas

- Table output should look like: `# | Criterion | Type | Command` with separator line
- Existing `hello-world.toml` template already demonstrates the target TOML structure
- `deny_unknown_fields` on both Spec and Criterion means new fields (like future `prompt`, `timeout`) require explicit code changes — strictness chosen over forward-compatibility

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-spec-files*
*Context gathered: 2026-03-01*
