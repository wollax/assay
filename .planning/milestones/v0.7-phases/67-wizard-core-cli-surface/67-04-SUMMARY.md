---
phase: 67-wizard-core-cli-surface
plan: "04"
subsystem: cli
tags: [wizard, assay-cli, criteria, dialoguer, tdd]

requires:
  - phase: 67-02
    provides: "apply_criteria_wizard from assay_core::wizard"
  - phase: 67-03
    provides: "wizard_helpers::prompt_slug, wizard_helpers::prompt_criteria_loop from assay-cli"

provides:
  - "CriteriaCommand enum (List, New variants) in assay-cli"
  - "handle_list: scan + format criteria libraries (default/verbose/json)"
  - "handle_new: TTY-guarded interactive criteria library authoring"
  - "build_input: pure helper for CriteriaWizardInput construction (testable without dialoguer)"

affects:
  - "68-mcp-tools: CriteriaCommand shape and ListArgs flags documented for reference"
  - "69-tui: handle_new flow (slug -> criteria -> metadata opt-in) mirrors gate wizard; TUI can adapt"

tech-stack:
  added: []
  patterns:
    - "TTY guard first: std::io::stdin().is_terminal() checked before current_dir() or any I/O"
    - "render_list takes &mut W: Write — production passes stdout(), tests pass Vec<u8>"
    - "build_input pure fn: CriteriaWizardInput construction extracted for unit-testability"
    - "Metadata opt-in gate: Confirm default=false keeps happy path fast (slug + criteria only)"
    - "assay_dir via crate::commands::assay_dir(&root) — consistent with gate.rs pattern"

key-files:
  created:
    - "crates/assay-cli/src/commands/criteria.rs"
  modified:
    - "crates/assay-cli/src/commands/mod.rs"
    - "crates/assay-cli/src/main.rs"

key-decisions:
  - "config.assay_dir does not exist on Config struct — use crate::commands::assay_dir(&root) consistent with all other CLI commands (Rule 1 auto-fix during Task 1)"
  - "Both Task 1 and Task 2 implemented in a single commit — tests for both were green together and the file is atomic; no reason to split an incomplete handle_new stub into its own commit"
  - "build_input helper extracted (not skipped) — provides a clean unit-test seam without mocking dialoguer; aligns with plan's recommended path"

metrics:
  duration: 2min
  completed: "2026-04-12"
  tasks: 2
  files: 3
---

# Phase 67 Plan 04: Criteria CLI Surface Summary

**CriteriaCommand (List/New) implemented; render_list supports default/verbose/json; handle_new TTY-guards and delegates to apply_criteria_wizard; 6 unit tests green; 2439 workspace tests pass**

## Performance

- **Duration:** 2 min
- **Started:** 2026-04-12T16:16:12Z
- **Completed:** 2026-04-12T16:18:24Z
- **Tasks:** 2
- **Files modified/created:** 3

## Accomplishments

- Created `crates/assay-cli/src/commands/criteria.rs` with `CriteriaCommand` enum, `ListArgs`, `handle`, `handle_list`, `render_list`, `handle_new`, `build_input`
- `render_list<W: Write>` accepts a generic writer for testability; production passes `std::io::stdout()`
- Default mode: `{slug:<32}  {N} criteria` per line
- Verbose mode: appends `description:`, `version:`, `tags:` lines when set
- JSON mode: `serde_json::to_string_pretty(libs)` — no ANSI sequences, valid JSON
- `handle_new` TTY-guards first (before `current_dir()`), then drives slug -> criteria -> metadata opt-in flow
- `build_input` extracted as a pure function for unit testing
- Registered `pub mod criteria` in `commands/mod.rs`
- Added `Command::Criteria` variant and dispatch arm in `main.rs`
- 6 unit tests: `criteria_list_format_default`, `criteria_list_format_json`, `criteria_list_format_verbose`, `criteria_list_empty`, `handle_new_non_tty`, `handle_new_builds_input`
- `just ready` fully green (2439 tests across workspace)

## CriteriaCommand Clap Shape

```rust
// crates/assay-cli/src/commands/criteria.rs
#[derive(clap::Subcommand, Debug)]
pub enum CriteriaCommand {
    /// List all criteria libraries under `.assay/criteria/`.
    List(ListArgs),
    /// Interactively create a new criteria library.
    New,
}

#[derive(clap::Args, Debug)]
pub struct ListArgs {
    /// Include description, version, and tags for each library.
    #[arg(long)]
    pub verbose: bool,
    /// Emit the full Vec<CriteriaLibrary> as JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}
```

## `criteria list` Output Format

**Default (`assay criteria list`):**
```
lib-a                             3 criteria
lib-b                             0 criteria
```
Format: `{slug:<32}  {N} criteria` per library (pads slug column to 32 chars, never truncates).

**Verbose (`assay criteria list --verbose`):**
```
lib-a                             3 criteria
    description: Standard Rust CI checks
    version:     1.0.0
    tags:        rust, build
```
Description omitted when empty; version omitted when None; tags omitted when empty.

**JSON (`assay criteria list --json`):**
`serde_json::to_string_pretty(Vec<CriteriaLibrary>)` — valid JSON, no ANSI codes.

## `handle_new` Flow

1. TTY guard: `std::io::stdin().is_terminal()` — returns `Ok(1)` if not a terminal
2. Slug: `wizard_helpers::prompt_slug("Library slug", None)` with inline validate_with
3. Criteria: `wizard_helpers::prompt_criteria_loop(&[])` — shared with gate wizard
4. Metadata opt-in: `Confirm::new().with_prompt("Add metadata...").default(false)`
   - If yes: description (allow_empty), version (allow_empty -> None if blank), tags (comma-split)
   - If no: empty description, None version, empty tags
5. `build_input(...)` -> `apply_criteria_wizard(&input, &assay_dir)?`
6. Print: created name, path, criterion count

## `build_input` Helper (for Phase 68/69 Reference)

```rust
fn build_input(
    name: String,
    description: String,
    version: Option<String>,
    tags: Vec<String>,
    criteria: Vec<CriterionInput>,
    overwrite: bool,
) -> CriteriaWizardInput
```

`build_input` is private (`fn`, not `pub`/`pub(crate)`) — only the test module calls it directly. MCP and TUI construct `CriteriaWizardInput` inline (they have their own input collection).

## Task Commits

1. **Tasks 1 + 2: Implement criteria list and new commands** — `01f7db5` (feat)

(Both tasks implemented atomically in a single commit — all tests were green together and the file has no testable intermediate state.)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] config.assay_dir does not exist on the Config struct**
- **Found during:** Task 1 compile check
- **Issue:** The plan's template code called `config.assay_dir` but `Config` in `assay-types` has no such field. The correct pattern (consistent with all other CLI commands including `gate.rs`) is `crate::commands::assay_dir(&root)`
- **Fix:** Replaced `assay_core::config::load` + `config.assay_dir` with `crate::commands::project_root()` + `crate::commands::assay_dir(&root)`
- **Files modified:** `crates/assay-cli/src/commands/criteria.rs`
- **Commit:** `01f7db5` (inline fix)

## Self-Check

- [x] `crates/assay-cli/src/commands/criteria.rs` — FOUND
- [x] `crates/assay-cli/src/commands/mod.rs` contains `pub mod criteria` — FOUND
- [x] `crates/assay-cli/src/main.rs` contains `Command::Criteria` — FOUND
- [x] Commit `01f7db5` — FOUND

## Self-Check: PASSED
