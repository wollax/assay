# Phase 5: Config and Initialization - Context

**Gathered:** 2026-03-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can initialize an Assay project (`assay init`) and the system can load/validate its configuration. Creates `.assay/` directory with `config.toml`, `specs/` subdirectory, and an example spec file. Config loading and validation live in `assay-core` as free functions.

</domain>

<decisions>
## Implementation Decisions

### Config file structure
- Template includes three sections: `project_name`, `specs_dir`, and `[gates]` table
- `project_name` — inferred from current directory name (sanitization approach is Claude's discretion)
- `specs_dir` — defaults to `"specs/"`, letting users customize where specs live
- `[gates]` section with `default_timeout` and `working_dir` — both as **commented-out hints** with explanatory comments
  - `# default_timeout = 300` — code uses 300s when absent
  - `# working_dir = "."` — comment references GATE-04 (explicit working_dir required per call site)

### Init command behavior
- Accepts optional `--name <name>` flag to override inferred project name; zero-arg otherwise
- Output follows cargo-style: `Created assay project \`my-project\`` with one line per created artifact
- On existing `.assay/`: error with hint — `"Error: .assay/ already exists. Remove it first to reinitialize."`
- Creates a minimal `.gitignore` inside `.assay/` ignoring transient files (results, caches) while tracking config and specs

### Example spec template
- Includes **both** a runnable criterion (`cmd = "echo hello"` or similar) and a descriptive-only criterion (no `cmd`) — demonstrates both modes
- TOML comments throughout explaining what each field does — self-documenting for new users
- Spec name/theme and filename convention are Claude's discretion

### Validation behavior
- **Collect all errors** — report every validation issue at once so user fixes everything in one pass
- **Strict validation** — `project_name` required, unknown keys rejected, type checking on all fields
- Error messages include **full path + field path**: `.assay/config.toml: [gates].default_timeout: expected positive integer, got "abc"`
- **Composable API**: `load()` (from file) always parses + validates; `from_str()` just parses without validation. Tests and tools can skip validation via `from_str()`

### Claude's Discretion
- Project name sanitization approach (how to handle special chars in directory names)
- Example spec name, theme, and filename convention
- Exact `.gitignore` contents (what counts as "transient")
- Config template formatting and comment wording

</decisions>

<specifics>
## Specific Ideas

- Cargo-style init output — `Created assay project \`name\`` is the reference for tone and brevity
- Comments in template files serve as documentation — users shouldn't need to look elsewhere to understand the format
- The example spec should prove the system works immediately (runnable criterion) while also showing the descriptive-only mode

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 05-config-and-initialization*
*Context gathered: 2026-03-01*
