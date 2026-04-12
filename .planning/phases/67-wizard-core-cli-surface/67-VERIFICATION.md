---
phase: 67-wizard-core-cli-surface
verified: 2026-04-12T16:45:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
human_verification:
  - test: "Run `assay gate wizard` in an interactive terminal"
    expected: "Prompts for gate name, description, extends, include libraries, criteria, and optional preconditions; writes gates.toml to <specs_dir>/<slug>/ on confirmation"
    why_human: "dialoguer TTY interaction cannot be exercised in automated cargo test context"
  - test: "Run `assay gate wizard --edit <existing-gate>` against an existing spec"
    expected: "Pre-fills all prompts with current values via with_initial_text; atomically replaces file on confirmation; fuzzy suggestion shown for unknown gate slug"
    why_human: "Interactive edit flow requires TTY; fuzzy suggestion needs a real project with known spec slugs"
  - test: "Run `assay criteria list` in a project with existing libraries"
    expected: "Prints one line per library formatted as `<slug>  <N> criteria`; --verbose adds description/version/tags; --json emits valid JSON"
    why_human: "Requires a real .assay/criteria/ directory; formatted output is a UX judgment call"
  - test: "Run `assay criteria new` and attempt an invalid slug (e.g., `../evil`)"
    expected: "User is re-prompted inline without advancing; slug rejection happens before any file write"
    why_human: "dialoguer validate_with requires TTY"
---

# Phase 67: Wizard Core + CLI Surface Verification Report

**Phase Goal:** `assay-core::wizard` exposes `apply_gate_wizard()` usable by any surface, and the CLI provides `assay gate wizard` (create/edit) and `assay criteria list/new` commands backed entirely by core validation logic.
**Verified:** 2026-04-12T16:45:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `assay gate wizard` launches an interactive prompt flow that creates a new gate TOML file with user-supplied criteria, parent reference, and library includes | VERIFIED (automated) + HUMAN NEEDED (interactive flow) | `GateCommand::Wizard { edit }` registered; `handle_wizard` calls `apply_gate_wizard`; 5 integration tests in `wizard_gate.rs` confirm file creation, field roundtrip, slug validation |
| 2 | `assay gate wizard --edit <gate>` loads an existing gate and allows modifying its criteria and composability fields, writing the result back | VERIFIED (automated) + HUMAN NEEDED (interactive flow) | `apply_gate_wizard_edit_overwrites` integration test passes; `handle_wizard` build path sets `overwrite: edit.is_some()`; `load_gate_for_edit` populates defaults via `with_initial_text` |
| 3 | `assay criteria list` displays all criteria libraries with slug and criterion count | VERIFIED | `handle_list` calls `compose::scan_libraries`; 4 render_list unit tests (default, verbose, json, empty) all pass; `Command::Criteria` wired in `main.rs` |
| 4 | `assay criteria new` creates a new criteria library via interactive prompt, rejecting invalid slugs before writing | VERIFIED (automated) + HUMAN NEEDED (interactive) | `apply_criteria_wizard` calls `compose::validate_slug` before any I/O; `handle_new` TTY-guards and delegates; `handle_new_non_tty` test passes; `handle_new_builds_input` confirms field mapping |

**Score:** 4/4 truths verified (3 also need human verification for interactive flow)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/assay-types/src/wizard_input.rs` | GateWizardInput, GateWizardOutput, CriteriaWizardInput, CriteriaWizardOutput, CriterionInput definitions + 13 unit tests | VERIFIED | All 5 structs present; all derive Serialize/Deserialize/JsonSchema; deny_unknown_fields on inputs; 13 tests pass |
| `crates/assay-types/src/lib.rs` | Re-exports wizard_input types | VERIFIED | `pub mod wizard_input;` present; `pub use wizard_input::{CriteriaWizardInput, CriteriaWizardOutput, CriterionInput, GateWizardInput, GateWizardOutput}` at line 88 |
| `crates/assay-core/src/wizard/mod.rs` | Split module root with pub use re-exports; shared `write_gate_spec` helper | VERIFIED | `pub(crate) fn write_gate_spec` implemented; `pub use` re-exports milestone items + gate + criteria; old `wizard.rs` absent |
| `crates/assay-core/src/wizard/gate.rs` | `apply_gate_wizard` implementation + slug rejection unit tests | VERIFIED | Full implementation with slug/extends/include validation, AlreadyExists collision check, atomic write via NamedTempFile; 3 unit slug tests + 5 integration tests green |
| `crates/assay-core/src/wizard/criteria.rs` | `apply_criteria_wizard` implementation + slug rejection unit tests | VERIFIED | Full implementation with slug validation, AlreadyExists collision check, delegates to `compose::save_library`; 2 unit tests + 4 integration tests green |
| `crates/assay-core/tests/wizard_gate.rs` | Integration tests: create, collision, edit-overwrite, empty_criteria, output_roundtrip | VERIFIED | 5 tests, all pass |
| `crates/assay-core/tests/wizard_criteria.rs` | Integration tests: create, collision, edit-overwrite, scan_finds_created_library | VERIFIED | 4 tests, all pass |
| `crates/assay-cli/src/commands/gate.rs` | `GateCommand::Wizard { edit }` variant + `handle_wizard` | VERIFIED | Variant at line 56; dispatch at line 98; `handle_wizard` at line 855; calls `apply_gate_wizard` at line 1008 |
| `crates/assay-cli/src/commands/wizard_helpers.rs` | `prompt_criteria_loop`, `prompt_slug`, `select_from_list`, `multi_select_from_list` (pub(crate)) | VERIFIED | All 4 helpers present and pub(crate); prompt_criteria_loop_stub_exists test passes |
| `crates/assay-cli/src/commands/criteria.rs` | `CriteriaCommand` enum (List, New) + `handle_list` + `handle_new` | VERIFIED | All present; render_list takes generic Writer; build_input extracted; 6 tests pass |
| `crates/assay-cli/src/main.rs` | `Command::Criteria` variant + dispatch arm | VERIFIED | `Criteria { command: commands::criteria::CriteriaCommand }` at line 203; dispatch at line 307 |
| `crates/assay-cli/src/commands/mod.rs` | `pub mod criteria` + `pub mod wizard_helpers` | VERIFIED | Both module registrations present at lines 3 and 16 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `wizard_input.rs` | `gates_spec.rs (SpecPreconditions)` | `use crate::{CriteriaLibrary, GatesSpec, SpecPreconditions}` | WIRED | Direct import at line 11 |
| `assay-types/src/lib.rs` | `wizard_input` module | `pub mod wizard_input; pub use wizard_input::*` | WIRED | Lines 36, 88-90 |
| `wizard/gate.rs` | `spec/compose.rs validate_slug` | `compose::validate_slug` called for slug/extends/include | WIRED | Lines 31, 33, 36 of gate.rs |
| `wizard/criteria.rs` | `spec/compose.rs save_library` | `compose::save_library(assay_dir, &library)` | WIRED | Line 60 of criteria.rs |
| `wizard/gate.rs` | `NamedTempFile atomic write` | `write_gate_spec` via `super::write_gate_spec` | WIRED | `super::write_gate_spec(&spec, specs_dir)` at line 56; NamedTempFile in mod.rs lines 70-84 |
| `gate.rs handle_wizard` | `assay_core::wizard::apply_gate_wizard` | direct function call after building `GateWizardInput` | WIRED | `assay_core::wizard::apply_gate_wizard(&input, &assay_dir, &specs_dir)` at line 1008 |
| `handle_wizard` | `dialoguer::Input::validate_with` | closure wrapping `compose::validate_slug` for inline slug rejection | WIRED | `prompt_slug` in wizard_helpers.rs uses `validate_with` at line 19 |
| `criteria.rs handle_list` | `compose::scan_libraries` | `assay_core::spec::compose::scan_libraries(&assay_dir)` | WIRED | Line 36 of criteria.rs |
| `criteria.rs handle_new` | `apply_criteria_wizard` | `assay_core::wizard::apply_criteria_wizard(&input, &assay_dir)` | WIRED | Line 118 of criteria.rs |
| `criteria.rs handle_new` | `wizard_helpers::prompt_criteria_loop` | `crate::commands::wizard_helpers::prompt_criteria_loop(&[])` | WIRED | Line 82 of criteria.rs |
| `wizard/mod.rs` | `assay_types::CriterionInput` | `pub use assay_types::CriterionInput` | WIRED | Line 31 of mod.rs — Plan 01 handoff re-export in place |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| WIZC-01 | 67-01, 67-02, 67-03 | User can create new gate definitions via `assay gate wizard` interactive flow | SATISFIED | `GateCommand::Wizard { edit: None }` path writes gates.toml via `apply_gate_wizard`; 5 integration tests confirm create semantics |
| WIZC-02 | 67-01, 67-02, 67-03 | User can edit existing gate definitions via the wizard | SATISFIED | `GateCommand::Wizard { edit: Some(slug) }` loads spec via `load_gate_for_edit`, passes `overwrite: true` to `apply_gate_wizard`; `apply_gate_wizard_edit_overwrites` integration test confirms atomic replacement |
| WIZC-03 | 67-01, 67-02, 67-04 | User can manage criteria libraries via `assay criteria list/new` commands | SATISFIED | `Command::Criteria` wired in main.rs; `handle_list` calls `scan_libraries`; `handle_new` calls `apply_criteria_wizard` with TTY guard; 6 unit tests cover formatting and non-TTY path |

No orphaned requirements — all three WIZC requirements claimed by plans 67-01 through 67-04 have confirmed implementations.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/assay-cli/src/commands/gate.rs` | 250 | `TODO(M024/S02): stream_criterion should accept events slice...` | Info | Pre-existing debt from M024, not introduced by Phase 67; scoped to event streaming path unrelated to wizard |

No Phase 67 anti-patterns found. No stubs, placeholders, or incomplete implementations in any of the 12 artifacts verified.

### Human Verification Required

#### 1. Gate Wizard — Create Flow

**Test:** Run `cargo run -p assay-cli -- gate wizard` in a project with an existing `.assay/` directory and at least one criteria library
**Expected:** Linear prompt flow: Gate name (slug validated inline, re-prompted on bad input) → Description → Extends (select from gate list or none) → Include libraries (multi-select) → Add criteria loop → Optional preconditions → Confirm write → `gates.toml` written to `<specs_dir>/<slug>/`
**Why human:** dialoguer TTY interaction cannot be exercised in cargo test context; prompt flow ordering and with_initial_text behavior require visual confirmation

#### 2. Gate Wizard — Edit Flow

**Test:** Run `cargo run -p assay-cli -- gate wizard --edit <known-gate>` against an existing spec; also run with `--edit does-not-exist` to verify fuzzy error
**Expected:** Existing gate values pre-filled in prompts; on confirm, file atomically replaced; for missing gate slug, error contains "Did you mean" suggestion or "not found" message
**Why human:** Edit pre-fill requires visual confirmation; fuzzy suggestion content varies by project state

#### 3. Criteria List — Output Format

**Test:** Run `assay criteria list`, `assay criteria list --verbose`, and `assay criteria list --json` in a project with multiple criteria libraries
**Expected:** Default: `<slug>  <N> criteria` per line; verbose: adds description/version/tags; json: valid pretty-printed JSON deserialisable as `Vec<CriteriaLibrary>`
**Why human:** Requires real .assay/criteria/ content; column alignment and field presence are UX judgments

#### 4. Criteria New — Slug Inline Rejection

**Test:** Run `assay criteria new` and enter `../evil` as the library slug
**Expected:** dialoguer re-prompts inline with the validation error message from `compose::validate_slug`; no file is written; entering a valid slug proceeds normally
**Why human:** validate_with inline rejection behavior requires TTY interaction to observe

---

*Verified: 2026-04-12T16:45:00Z*
*Verifier: Claude (kata-verifier)*
