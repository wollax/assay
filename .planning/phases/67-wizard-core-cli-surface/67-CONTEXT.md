# Phase 67: Wizard Core + CLI Surface - Context

**Gathered:** 2026-04-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the shared gate-authoring wizard in `assay-core` and expose it via the CLI. Deliverables: `assay-core::wizard::apply_gate_wizard()` usable by any surface, plus `assay gate wizard [--edit <gate>]`, `assay criteria list`, and `assay criteria new` CLI commands. All validation lives in core; the CLI only collects input via `dialoguer` and renders output. MCP tools (Phase 68) and TUI (Phase 69) will consume this same core API without changes.

Out of scope: MCP tools, TUI screens, non-interactive/scriptable flags (`--json`, batch mode), gate renaming.

</domain>

<decisions>
## Implementation Decisions

### Core API shape
- Single function `apply_gate_wizard(input: GateWizardInput, assay_dir: &Path, specs_dir: &Path) -> Result<WizardOutput>` — pure validate + write, matches existing `create_spec_from_params` pattern
- `GateWizardInput` type lives in `assay-types` with `schemars::JsonSchema` derive — Phase 68's `gate_wizard` MCP tool gets schema generation for free
- Edit mode takes a full new `GatesSpec`, not a patch/diff — surface loads existing, presents current values as defaults, wizard writes full replacement
- Errors are fail-on-first structured `AssayError` variants — consistent with `compose::resolve()` and `save_library`; surfaces re-prompt the offending field
- `apply_criteria_wizard()` sibling function with the same shape for `criteria new` — reuse `compose::save_library` internally
- Zero-trait, closures only if needed (consistent with milestone-wide convention from Phase 65)

### Gate file location & edit semantics
- New gates write to `<specs_dir>/<slug>/gates.toml` — matches existing `create_spec_from_params` layout; the gate IS the spec in current architecture
- `--edit <gate>` identifies the target by gate name / spec slug (not path) — consistent with `assay gate run <name>` and `assay spec show <name>`; fuzzy suggestions on not-found (reuse `enriched_error_display` pattern from `gate/mod.rs:474`)
- Edit mode allows modifying all `GatesSpec` fields except name/slug (rename = delete + create) — satisfies WIZC-02's "modify its criteria and composability fields" and then some
- Create mode fails if `gates.toml` already exists at target path — matches existing wizard semantics; no `--force` flag in v1
- Edit mode unconditionally overwrites via atomic tempfile-then-rename (same pattern as `save_library` / `work_session` persistence)

### CLI wizard UX flow
- Linear one-pass `dialoguer` flow — no back-navigation, Ctrl+C aborts; mirrors the established `plan.rs` pattern exactly
- Prompt order: name → description → extends → includes → criteria (inline loop) → preconditions (opt-in) → final confirm → write
- **Criteria entry:** inline add-another loop — prompt `name` → `description` → optional `cmd` → `Confirm "add another criterion?" y/N`. Identical to the existing chunk-criteria flow in `plan.rs`
- **`extends` / `include` selection:** `dialoguer::Select` for extends (with explicit `(none)` option) and `MultiSelect` for includes, populated by scanning `<specs_dir>` for gates and `.assay/criteria/` via `compose::scan_libraries()`. Users cannot pick missing targets; no free-text fallback needed for v1
- **Preconditions:** opt-in — `Confirm "add preconditions?" default=No`. If yes: `requires` via MultiSelect of spec slugs, `commands` via inline add-another loop. Most gates will skip; keeps happy path short
- **Edit mode:** same linear sequence but each prompt pre-fills with the existing value as `dialoguer`'s default

### `criteria` subcommand behavior
- `assay criteria list` default output: `<slug>  <N criteria>` per line — matches success criterion WIZC-03 wording exactly. `--verbose` adds description/version/tags; `--json` emits full `Vec<CriteriaLibrary>` payload
- `assay criteria new` uses progressive field prompts — always: slug, criteria (shared inline loop with gate wizard). Then `Confirm "add metadata (description / version / tags)?" default=No` gates the remaining optional fields
- Slug validation inline via `dialoguer::Input::validate_with(compose::validate_slug)` — satisfies "rejecting invalid slugs before writing"; user can't proceed past a bad slug
- Criteria-entry helper extracted as a shared CLI function used by both `gate wizard` and `criteria new` — one source of truth for the prompt loop

### Claude's Discretion
- Exact `GateWizardInput` / `WizardOutput` field shapes (naming, which fields are `Option<T>`) — follow `WizardChunkInput` / `WizardResult` conventions in existing `wizard.rs`
- Whether edit-mode surface pre-load helper (`load_gate_for_edit(&name) -> GatesSpec`) lives in core or CLI
- How discovered-gate scan is implemented (iterate `spec::scan()` results vs dedicated walker)
- Error message copy for fuzzy suggestions and re-prompts
- Whether final-confirm uses `Confirm` or a summary-then-Select (`[write] [cancel]`)
- Internal module layout: extend existing `wizard.rs` vs split into `wizard/gate.rs` + `wizard/criteria.rs` (lean toward split given growth)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/assay-core/src/wizard.rs` — Existing milestone+spec wizard (`create_from_inputs`, `create_spec_from_params`, `write_gates_toml`, `slugify`). `write_gates_toml` already writes a `GatesSpec` with `extends: None`, `include: vec![]`, `preconditions: None` — extend it to accept populated values rather than duplicating.
- `compose::validate_slug` (`spec/compose.rs:21`) — Reuse for gate name and library slug validation throughout the wizard.
- `compose::save_library`, `load_library`, `scan_libraries`, `load_library_by_slug` (`spec/compose.rs`) — All four functions already exist from Phase 65; `criteria new` just needs a wizard wrapper.
- `compose::resolve()` — Not called during wizard authoring itself, but validation can optionally run resolve-in-dry-run to surface composability errors before write.
- `enriched_error_display` (`gate/mod.rs:474`) — Fuzzy matching for `--edit <gate>` not-found errors.
- `dialoguer` crate (workspace dep, version 0.12.0) — Prompt primitives already in use in `commands/plan.rs`; patterns to mirror: `Input::new().with_prompt(...).interact_text()`, `Confirm::new().with_prompt(...).default(...).interact()`, `Select::new()...interact()`.
- Atomic write pattern (`NamedTempFile::new_in` → `write_all` → `sync_all` → `persist`) from `write_gates_toml` and `save_library` — reuse for edit-mode overwrites.
- `spec::scan()` (`spec/mod.rs:683`) — For enumerating existing gates when populating the `extends` Select.

### Established Patterns
- Free functions with closures, no traits
- `GateWizardInput` / `WizardOutput` follow the `WizardChunkInput` / `WizardResult` naming convention already set in `wizard.rs`
- All wizard input types carry `schemars::JsonSchema` derive for MCP schema generation
- `#[serde(default, skip_serializing_if)]` on every optional field; `#[serde(deny_unknown_fields)]` on TOML-authored types (library files)
- Structured `AssayError` variants with context fields; `#[non_exhaustive]`
- Inline TDD tests in `#[cfg(test)]` modules alongside implementation
- CLI command modules expose a `XxxCommand` clap subcommand enum and a thin `handle()` / dispatch function (`commands/gate.rs`, `commands/spec.rs` pattern)

### Integration Points
- `crates/assay-types/src/`: New `GateWizardInput`, `WizardOutput` (or `GateWizardOutput`) types, plus re-exports in `lib.rs`. Likely a new `wizard_input.rs` module to avoid bloating existing type files.
- `crates/assay-core/src/wizard.rs` (or new `wizard/` submodule): New `apply_gate_wizard()`, `apply_criteria_wizard()` functions. Consider splitting the existing monolithic `wizard.rs` into `wizard/mod.rs` + `wizard/milestone.rs` + `wizard/gate.rs` + `wizard/criteria.rs`.
- `crates/assay-cli/src/commands/gate.rs`: New `Wizard { edit: Option<String> }` variant on `GateCommand` enum, new `handle_wizard()` function that drives `dialoguer` and calls `apply_gate_wizard`.
- `crates/assay-cli/src/commands/`: New `criteria.rs` module with `CriteriaCommand::{ List, New }` and handlers. Wire into `main.rs` as a new `Command::Criteria` variant.
- `crates/assay-cli/src/main.rs`: Register the new `Criteria` top-level subcommand in both the `Command` enum and the `match` dispatch.
- New `AssayError` variants if existing set doesn't cover wizard-specific cases (e.g., `GateAlreadyExists`, `GateNotFound` — though latter may already exist).

</code_context>

<specifics>
## Specific Ideas

- UX should feel like the existing `assay plan` flow — same prompt styling, same "add another?" pattern, same terse `(y/N)` conventions. No new interaction metaphors.
- The wizard is the happy path for composability in v0.7.0 — users who aren't reading TOML docs should still discover `extends` / `include` / `preconditions` by walking through prompts.
- Phase 68 (MCP) and Phase 69 (TUI) will consume `apply_gate_wizard()` / `apply_criteria_wizard()` with zero changes — if a decision forces Phase 68/69 to reimplement logic, it's wrong.

</specifics>

<deferred>
## Deferred Ideas

- Non-interactive / scriptable flags on `gate wizard` (e.g., `--name`, `--criterion`, `--from-toml`) — would belong in a later CLI-ergonomics phase; MCP covers the agent-driven case.
- `assay gate wizard` producing JSON output for piping — same reasoning; add when a caller needs it.
- Gate rename (changing the slug during `--edit`) — separate concern, requires cross-referencing existing specs/milestones.
- Menu-driven or final-review-with-back-navigation flows — revisit after real user feedback on the linear flow.
- `$EDITOR`-launched TOML editing mode — power-user shortcut; not needed if the linear wizard covers 80% of cases.
- `criteria edit <slug>` command — symmetric to `gate wizard --edit` but not in scope for WIZC-03 ("list/new" only).
- Multi-level `extends` validation inside the wizard (INHR-05) — already deferred at milestone level.

</deferred>

---

*Phase: 67-wizard-core-cli-surface*
*Context gathered: 2026-04-12*
