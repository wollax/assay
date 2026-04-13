# Phase 67: Wizard Core + CLI Surface - Research

**Researched:** 2026-04-12
**Domain:** Rust interactive CLI wizard, TOML authoring, core/surface separation
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Core API shape**
- Single function `apply_gate_wizard(input: GateWizardInput, assay_dir: &Path, specs_dir: &Path) -> Result<WizardOutput>` — pure validate + write, matches existing `create_spec_from_params` pattern
- `GateWizardInput` type lives in `assay-types` with `schemars::JsonSchema` derive
- Edit mode takes a full new `GatesSpec`, not a patch/diff
- Errors are fail-on-first structured `AssayError` variants — surfaces re-prompt the offending field
- `apply_criteria_wizard()` sibling function — reuse `compose::save_library` internally
- Zero-trait, closures only if needed (consistent with Phase 65 convention)

**Gate file location & edit semantics**
- New gates write to `<specs_dir>/<slug>/gates.toml`
- `--edit <gate>` identifies target by gate name/spec slug (not path); fuzzy suggestions on not-found (reuse `enriched_error_display` pattern from `gate/mod.rs:474`)
- Edit mode allows modifying all `GatesSpec` fields except name/slug
- Create mode fails if `gates.toml` already exists at target path; no `--force` flag in v1
- Edit mode unconditionally overwrites via atomic tempfile-then-rename

**CLI wizard UX flow**
- Linear one-pass `dialoguer` flow — no back-navigation, Ctrl+C aborts
- Prompt order: name → description → extends → includes → criteria (inline loop) → preconditions (opt-in) → final confirm → write
- Criteria entry: inline add-another loop (name → description → optional cmd → "add another?" y/N)
- `extends`/`include` selection: `dialoguer::Select` for extends (with "(none)" option) and `MultiSelect` for includes, populated by scanning `<specs_dir>` for gates and `.assay/criteria/` via `compose::scan_libraries()`
- Preconditions: opt-in via `Confirm "add preconditions?" default=No`; if yes: `requires` via MultiSelect of spec slugs, `commands` via inline add-another loop
- Edit mode: same linear sequence with each prompt pre-filled with existing value

**`criteria` subcommand behavior**
- `assay criteria list` default output: `<slug>  <N criteria>` per line; `--verbose` adds description/version/tags; `--json` emits full `Vec<CriteriaLibrary>`
- `assay criteria new` uses progressive field prompts; metadata gated behind `Confirm "add metadata?" default=No`
- Slug validation inline via `dialoguer::Input::validate_with(compose::validate_slug)`
- Criteria-entry helper extracted as a shared CLI function used by both `gate wizard` and `criteria new`

### Claude's Discretion
- Exact `GateWizardInput` / `WizardOutput` field shapes (naming, which fields are `Option<T>`) — follow `WizardChunkInput` / `WizardResult` conventions
- Whether edit-mode surface pre-load helper (`load_gate_for_edit`) lives in core or CLI
- How discovered-gate scan is implemented (iterate `spec::scan()` results vs dedicated walker)
- Error message copy for fuzzy suggestions and re-prompts
- Whether final-confirm uses `Confirm` or a summary-then-Select
- Internal module layout: extend `wizard.rs` vs split into `wizard/gate.rs` + `wizard/criteria.rs` (lean toward split)

### Deferred Ideas (OUT OF SCOPE)
- Non-interactive/scriptable flags (`--name`, `--criterion`, `--from-toml`)
- `assay gate wizard` JSON output
- Gate rename during `--edit`
- Menu-driven or back-navigation flows
- `$EDITOR`-launched TOML editing mode
- `criteria edit <slug>` command
- Multi-level `extends` validation inside the wizard (INHR-05)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WIZC-01 | User can create new gate definitions via `assay gate wizard` interactive flow | `apply_gate_wizard()` in core + `GateCommand::Wizard` in CLI; reuses `write_gates_toml` pattern, `dialoguer` primitives from `plan.rs` |
| WIZC-02 | User can edit existing gate definitions via the wizard | Edit branch in `apply_gate_wizard()`: `load_spec_entry` to read existing, same write path; `--edit <gate>` CLI flag; `SpecNotFoundDiagnostic` for fuzzy errors |
| WIZC-03 | User can manage criteria libraries via `assay criteria list/new` commands | `apply_criteria_wizard()` wrapping `compose::save_library`; `CriteriaCommand::{ List, New }` module; `compose::scan_libraries()` already returns sorted `Vec<CriteriaLibrary>` |
</phase_requirements>

## Summary

Phase 67 builds the shared gate-authoring wizard in `assay-core` and exposes it through the CLI. The foundation is entirely in place from Phases 64-66: `GatesSpec` carries all composability fields (`extends`, `include`, `preconditions`), `compose::validate_slug` / `save_library` / `scan_libraries` / `load_library_by_slug` are implemented, and the existing `wizard.rs` already contains `write_gates_toml` (atomic write) and the `CriterionInput` / `WizardChunkInput` patterns that the new types must mirror.

The CLI layer uses `dialoguer` 0.12.0 (already a workspace dep, already used in `commands/plan.rs`). The prompt patterns — `Input::new().with_prompt(...).interact_text()`, `Confirm::new().with_prompt(...).default(...).interact()`, `Select::new()...interact()`, `MultiSelect::new()...interact()` — are established. The new `validate_with` capability (Input validation before acceptance) is the one API surface not yet exercised in the codebase; it needs a closure wrapping `compose::validate_slug`.

The core functions (`apply_gate_wizard`, `apply_criteria_wizard`) are pure I/O: they receive fully-constructed input structs and write files. The CLI collects prompts, constructs those structs, then calls core. MCP (Phase 68) and TUI (Phase 69) will construct the same structs from their own surfaces. This clean separation means no validation logic leaks into the CLI.

**Primary recommendation:** Add `GateWizardInput` / `GateWizardOutput` to `assay-types`, implement `apply_gate_wizard()` / `apply_criteria_wizard()` in `assay-core::wizard` (preferably as a split `wizard/` submodule), then wire `GateCommand::Wizard` and `CriteriaCommand` in `assay-cli`. Test core functions with `tempfile::TempDir` integration tests; test CLI non-TTY guard and `criteria list` output with unit tests.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `dialoguer` | 0.12.0 | Interactive prompts (Input, Confirm, Select, MultiSelect) | Workspace dep; already used in `commands/plan.rs`; TTY-aware |
| `tempfile` | 3 | Atomic write (NamedTempFile) | Workspace dep; already used in `wizard.rs`, `save_library` |
| `toml` | 1 | Serialization of `GatesSpec` / `CriteriaLibrary` to TOML | Workspace dep; established pattern |
| `schemars` | 1 | `JsonSchema` derive on new input types | Workspace dep; required for Phase 68 MCP schema generation |
| `serde` | 1 (derive) | Serialize/Deserialize on all types | Workspace dep; universal |
| `clap` | 4 (derive) | CLI arg parsing for `Wizard { edit: Option<String> }` | Workspace dep; established pattern |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `anyhow` | (workspace) | CLI error propagation (`anyhow::Result<i32>`) | CLI command handlers only; core uses `AssayError` |
| `std::io::IsTerminal` | stdlib | TTY guard (same as `plan.rs`) | Top of every interactive handler |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `dialoguer` | `inquire` | `inquire` is more featureful but not in workspace; `dialoguer` is already established — do not change |
| Split `wizard/` submodule | Extend monolithic `wizard.rs` | Split preferred: avoids 600-line files; CONTEXT.md explicitly leans toward split |

**No additional installation required.** All dependencies are already in workspace `Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure

```
crates/assay-types/src/
└── wizard_input.rs          # GateWizardInput, GateWizardOutput, CriteriaWizardInput
    (re-exported from lib.rs)

crates/assay-core/src/
└── wizard/
    ├── mod.rs               # pub use; shared helpers (write_gates_toml, CriterionInput)
    ├── milestone.rs         # Existing create_from_inputs, create_spec_from_params (moved here)
    ├── gate.rs              # NEW: apply_gate_wizard(), load_gate_for_edit()
    └── criteria.rs          # NEW: apply_criteria_wizard()

crates/assay-cli/src/commands/
├── gate.rs                  # Add Wizard { edit: Option<String> } variant + handle_wizard()
├── criteria.rs              # NEW: CriteriaCommand::{ List, New }, handlers
└── mod.rs                   # Shared prompt helper: prompt_criteria_loop()

crates/assay-cli/src/
└── main.rs                  # Add Command::Criteria variant + dispatch
```

### Pattern 1: Core Wizard Function Shape

The core functions are pure (no TTY dependency): they receive input structs, validate, write atomically, return a result path.

**What:** Validate slug + collision + write gate TOML atomically.
**When to use:** Called by CLI after collecting prompts, by MCP directly, by TUI after form submit.

```rust
// In crates/assay-types/src/wizard_input.rs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateWizardInput {
    pub slug: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub include: Vec<String>,
    pub criteria: Vec<CriterionInput>,
    pub preconditions: Option<SpecPreconditions>,
    // Edit mode: true = overwrite existing, false = fail if exists
    pub overwrite: bool,
}

#[derive(Debug)]
pub struct GateWizardOutput {
    pub path: PathBuf,
    pub spec: GatesSpec,
}

// In crates/assay-core/src/wizard/gate.rs
pub fn apply_gate_wizard(
    input: &GateWizardInput,
    assay_dir: &Path,
    specs_dir: &Path,
) -> Result<GateWizardOutput> {
    compose::validate_slug(&input.slug)?;
    // Validate extends/include slugs
    if let Some(ref ext) = input.extends { compose::validate_slug(ext)?; }
    for inc in &input.include { compose::validate_slug(inc)?; }

    let gate_path = specs_dir.join(&input.slug).join("gates.toml");
    if gate_path.exists() && !input.overwrite {
        return Err(AssayError::Io { operation: format!("gate '{}' already exists", input.slug), path: gate_path, source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "...") });
    }

    let spec = build_gates_spec(input);
    write_gates_toml_full(&spec, specs_dir)?;  // atomic
    Ok(GateWizardOutput { path: gate_path, spec })
}
```

### Pattern 2: CLI Handler with TTY Guard

**What:** Collect prompts, build input struct, call core, print result.
**When to use:** All interactive CLI handlers.

```rust
// Mirrors plan.rs exactly
pub(crate) fn handle_wizard(edit: Option<String>) -> anyhow::Result<i32> {
    if !std::io::stdin().is_terminal() {
        tracing::error!("assay gate wizard requires an interactive terminal.");
        return Ok(1);
    }
    // ... collect prompts ...
    let input = GateWizardInput { ... };
    let output = assay_core::wizard::apply_gate_wizard(&input, &assay, &specs)?;
    println!("  Created gate '{}'", input.slug);
    println!("    written {}", output.path.display());
    Ok(0)
}
```

### Pattern 3: `Input::validate_with` for Inline Slug Validation

This is the key new `dialoguer` feature for this phase. Wraps `compose::validate_slug` to reject invalid input before the user advances.

```rust
let slug: String = dialoguer::Input::new()
    .with_prompt("Gate name (slug)")
    .validate_with(|input: &String| -> Result<(), String> {
        assay_core::spec::compose::validate_slug(input)
            .map_err(|e| e.to_string())
    })
    .interact_text()?;
```

### Pattern 4: Select/MultiSelect Populated from Scan

**What:** Enumerate existing gates (for `extends`) and libraries (for `include`) from disk, display as selectable lists.

```rust
// For extends (single select with explicit "(none)" option)
let scan = assay_core::spec::scan(&specs_dir)?;
let mut gate_options: Vec<String> = scan.entries.iter().map(|e| e.slug().to_string()).collect();
gate_options.insert(0, "(none)".to_string());

let extends_idx = dialoguer::Select::new()
    .with_prompt("Extends (parent gate)")
    .items(&gate_options)
    .default(0)
    .interact()?;
let extends = if extends_idx == 0 { None } else { Some(gate_options[extends_idx].clone()) };

// For include (multi select)
let libs = assay_core::spec::compose::scan_libraries(&assay_dir)?;
let lib_names: Vec<&str> = libs.iter().map(|l| l.name.as_str()).collect();
let include_indices = dialoguer::MultiSelect::new()
    .with_prompt("Include criteria libraries")
    .items(&lib_names)
    .interact()?;
let include: Vec<String> = include_indices.iter().map(|&i| lib_names[i].to_string()).collect();
```

### Pattern 5: Shared Criteria-Entry Loop (CLI)

Extract as a standalone function in `commands/mod.rs` (or a shared `commands/wizard_helpers.rs`) to avoid duplication between `gate wizard` and `criteria new`.

```rust
pub(crate) fn prompt_criteria_loop() -> anyhow::Result<Vec<CriterionInput>> {
    let mut criteria = Vec::new();
    loop {
        let add = dialoguer::Confirm::new()
            .with_prompt("  Add a criterion?")
            .default(criteria.is_empty())
            .interact()?;
        if !add { break; }

        let name: String = dialoguer::Input::new().with_prompt("    Criterion name").interact_text()?;
        let description: String = dialoguer::Input::new().with_prompt("    Description").allow_empty(true).interact_text()?;
        let cmd_raw: String = dialoguer::Input::new().with_prompt("    Command (Enter to skip)").allow_empty(true).interact_text()?;
        let cmd = if cmd_raw.trim().is_empty() { None } else { Some(cmd_raw.trim().to_string()) };

        criteria.push(CriterionInput { name, description, cmd });
    }
    Ok(criteria)
}
```

### Pattern 6: Edit Mode Pre-Load

Load existing gate, pass current values as defaults to each `dialoguer` prompt.

```rust
// CLI: load existing spec for edit mode
fn load_gate_for_edit(slug: &str, specs_dir: &Path) -> anyhow::Result<GatesSpec> {
    assay_core::spec::load_spec_entry_with_diagnostics(slug, specs_dir)
        .map(|entry| match entry {
            SpecEntry::Directory { gates, .. } => Ok(gates),
            SpecEntry::Legacy { .. } => Err(anyhow::anyhow!("legacy flat-file specs cannot be edited with gate wizard")),
        })?
}
```

Then use `.with_initial_text(&existing.description)` on `Input` prompts and pre-select indices on `Select`/`MultiSelect`.

### Pattern 7: `criteria list` Output Format

```rust
// Default: "<slug>  <N criteria>" per line
for lib in &libs {
    println!("{:<32}  {} criteria", lib.name, lib.criteria.len());
}

// --verbose adds description/version/tags
// --json: serde_json::to_string_pretty(&libs)
```

### Anti-Patterns to Avoid

- **Validation logic in CLI handler:** All slug/field validation must go through `compose::validate_slug` and `apply_gate_wizard`. The CLI only uses `validate_with` for inline UX; the core function re-validates before any I/O.
- **Non-atomic write for edit mode:** Edit must use the same `NamedTempFile` → `write_all` → `sync_all` → `persist` chain as `save_library` and `write_gates_toml`. Never overwrite directly.
- **`write_gates_toml` duplication:** The private `write_gates_toml` helper in `wizard.rs` (now `wizard/milestone.rs`) must be made accessible (pub within crate) or promoted into a shared `write_gate_spec` function that `gate.rs` also calls. Do not copy-paste.
- **Blocking on empty criteria:** Allow saving a gate with zero criteria (useful for "template" gates that inherit everything). Do not add a minimum-criteria requirement.
- **Hardcoding specs_dir path:** Always resolve through `assay_core::config::load(&root)?.specs_dir`. Never construct `.assay/specs` by hand in CLI code.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Slug validation | Custom regex | `compose::validate_slug` | Already handles empty, length, first-char, charset, returns structured `AssayError::InvalidSlug` |
| Atomic TOML write | `fs::write` | `NamedTempFile` → `sync_all` → `persist` | Pattern already in `write_gates_toml` and `save_library`; hand-roll risks partial writes on crash |
| Criteria library I/O | Custom walker | `compose::save_library` / `scan_libraries` | Both already implemented with error handling and sort order |
| Fuzzy not-found errors | Custom edit-distance | `load_spec_entry_with_diagnostics` | Wraps `find_fuzzy_match` with `SpecNotFoundDiagnostic` — the same path `gate run` uses |
| Gate enumeration for Select | `std::fs::read_dir` | `spec::scan(&specs_dir)` | Returns `ScanResult` with valid `SpecEntry` list, already sorted; skips invalid entries gracefully |
| Interactive prompt loop | State machine | `dialoguer` `Input` / `Confirm` / `Select` / `MultiSelect` | TTY detection, Ctrl+C handling, theming, all handled; `validate_with` for inline rejection |

**Key insight:** The wizard is thin glue between dialoguer prompts and the compose/spec I/O functions already built in Phases 64-66. Any validation or I/O that isn't in those functions belongs there, not in the wizard.

## Common Pitfalls

### Pitfall 1: `dialoguer::Input::validate_with` Closure Lifetime

**What goes wrong:** `validate_with` takes a `Fn(&T) -> Result<(), E>` closure. If the closure captures a reference with a shorter lifetime than the `Input` builder, rustc reports a lifetime error.
**Why it happens:** The `Input` builder type is parameterized on the closure; closures capturing `&str` from a local variable can cause issues.
**How to avoid:** Use `|input: &String| -> Result<(), String>` with `e.to_string()` conversion. Do not capture references; call `compose::validate_slug(input.as_str())` directly.
**Warning signs:** Compiler error mentioning "closure may outlive the current function" or "borrowed value does not live long enough."

### Pitfall 2: `GatesSpec` Has `deny_unknown_fields`

**What goes wrong:** When constructing `GatesSpec` for write and then reading back during tests, any unknown field added to the struct during construction will panic at deserialization.
**Why it happens:** `#[serde(deny_unknown_fields)]` on `GatesSpec` is enforced at deserialization, not serialization.
**How to avoid:** Always construct `GatesSpec` with all fields explicitly set. Match the exact field list: `name`, `description`, `gate`, `depends`, `milestone`, `order`, `extends`, `include`, `preconditions`, `criteria`. Do not add new fields to `GatesSpec` itself — the wizard constructs it from `GateWizardInput`.
**Warning signs:** Tests fail with "unknown field" at `toml::from_str` in roundtrip assertions.

### Pitfall 3: Module Reorganization Breaking `crate::wizard` Public API

**What goes wrong:** Splitting `wizard.rs` into `wizard/mod.rs` + `wizard/milestone.rs` + `wizard/gate.rs` + `wizard/criteria.rs` breaks the `crates/assay-core/tests/wizard.rs` integration test imports and the `assay-cli` imports.
**Why it happens:** `use assay_core::wizard::{CriterionInput, WizardChunkInput, ...}` expects these to be exported from the `wizard` module path.
**How to avoid:** In `wizard/mod.rs`, re-export everything with `pub use milestone::{...}; pub use gate::{...}; pub use criteria::{...};` so the public path is unchanged. All existing imports continue to work.
**Warning signs:** Compile error "use of unresolved import `assay_core::wizard::CriterionInput`."

### Pitfall 4: Edit Mode Loading Legacy Specs

**What goes wrong:** `assay gate wizard --edit <gate>` calls `load_spec_entry_with_diagnostics`, which returns either `SpecEntry::Directory` or `SpecEntry::Legacy`. `GatesSpec` only exists on `Directory` variants.
**Why it happens:** Legacy flat-file specs (`.assay/specs/<slug>.toml`) use the `Spec` type, not `GatesSpec` — they have no `extends`/`include` fields.
**How to avoid:** Match on `SpecEntry::Directory { gates, .. }` and return a user-facing error for `SpecEntry::Legacy`. Message: "this spec uses the legacy flat-file format; gate wizard only supports directory-based specs."
**Warning signs:** Panic or type mismatch when user tries `--edit` on an old spec.

### Pitfall 5: `dialoguer::Select` Returns Index, Not Value

**What goes wrong:** Treating the return value of `.interact()` as the selected string rather than the selected index.
**Why it happens:** `Select::interact()` returns `usize` (the 0-based index into the items slice). New code mistakenly indexes the wrong vector or confuses item index with item value.
**How to avoid:** Always dereference: `let value = &items[idx]`. For `MultiSelect`, iterate over returned indices: `indices.iter().map(|&i| items[i].clone())`.
**Warning signs:** Wrong value selected, off-by-one when "(none)" option is inserted at index 0.

### Pitfall 6: `write_gates_toml` Private Function Inaccessibility

**What goes wrong:** The helper `write_gates_toml` in `wizard.rs` is currently `fn` (private). The new `wizard/gate.rs` module cannot call it unless visibility is adjusted.
**Why it happens:** The split reorganization creates sibling submodules that cannot see each other's private items.
**How to avoid:** Either (a) promote `write_gates_toml` to `pub(crate)` in `wizard/mod.rs` and have both `milestone.rs` and `gate.rs` call it, or (b) create a separate `write_gate_spec(spec: &GatesSpec, specs_dir: &Path) -> Result<PathBuf>` public-within-crate function that both use.
**Warning signs:** Compile error "function `write_gates_toml` is private."

## Code Examples

Verified patterns from existing codebase:

### Atomic Write Pattern (from `wizard.rs` write_gates_toml)
```rust
// Source: crates/assay-core/src/wizard.rs lines 405-422
let mut tmpfile = NamedTempFile::new_in(&chunk_dir)
    .map_err(|e| AssayError::io("creating temp file for gates.toml", &chunk_dir, e))?;
tmpfile.write_all(content.as_bytes())
    .map_err(|e| AssayError::io("writing gates.toml", &final_path, e))?;
tmpfile.as_file().sync_all()
    .map_err(|e| AssayError::io("syncing gates.toml", &final_path, e))?;
tmpfile.persist(&final_path)
    .map_err(|e| AssayError::io("persisting gates.toml", &final_path, e.error))?;
```

### Dialoguer Confirm Pattern (from `plan.rs`)
```rust
// Source: crates/assay-cli/src/commands/plan.rs lines 38-49
let has_description: bool = dialoguer::Confirm::new()
    .with_prompt("Add a description?")
    .default(false)
    .interact()?;
```

### Dialoguer Add-Another Loop (from `plan.rs`)
```rust
// Source: crates/assay-cli/src/commands/plan.rs lines 75-106
loop {
    let add_more = dialoguer::Confirm::new()
        .with_prompt("  Add a criterion?")
        .default(criteria.is_empty())
        .interact()?;
    if !add_more { break; }
    // ... collect criterion fields ...
}
```

### scan_libraries returning sorted Vec (from `compose.rs`)
```rust
// Source: crates/assay-core/src/spec/compose.rs lines 136-162
pub fn scan_libraries(assay_dir: &Path) -> Result<Vec<CriteriaLibrary>> {
    let criteria_dir = assay_dir.join("criteria");
    if !criteria_dir.is_dir() { return Ok(vec![]); }
    // ... reads dir, filters .toml, sorts by name ...
}
```

### validate_slug returning AssayError::InvalidSlug (from `compose.rs`)
```rust
// Source: crates/assay-core/src/spec/compose.rs lines 21-56
pub fn validate_slug(value: &str) -> Result<()> {
    // non-empty, <= 64 chars, first char [a-z0-9], body [a-z0-9-_]
    // Returns Err(AssayError::InvalidSlug { slug, reason }) on violation
}
```

### AssayError variants for gate wizard (from `error.rs`)
Already available without new variants:
- `AssayError::Io` with `AlreadyExists` kind — for create mode collision
- `AssayError::SpecNotFoundDiagnostic` — for `--edit <gate>` not-found
- `AssayError::InvalidSlug` — from `validate_slug`
- `AssayError::LibraryNotFound` — from `load_library_by_slug`
- `AssayError::ParentGateNotFound` — from `compose::resolve` (if dry-run validation is added)

New variants needed: `GateAlreadyExists` — or reuse `AssayError::Io { source: AlreadyExists }` with a descriptive operation string (preferred: reuse pattern from `create_spec_from_params` line 307-315, which uses `Io` with `AlreadyExists` kind — no new variant needed).

### CriteriaLibrary struct (from `criteria_library.rs`)
```rust
// Source: crates/assay-types/src/criteria_library.rs
pub struct CriteriaLibrary {
    pub name: String,                          // slug
    pub description: String,                   // #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: Option<String>,               // #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Vec<String>,                     // #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<crate::Criterion>,
}
```

### Wiring a new top-level command (from `main.rs`)
```rust
// Pattern from main.rs: add to Command enum, add to tracing_config_for match, add to run() match
Command::Criteria { command } => commands::criteria::handle(command),
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual `fs::write` | `NamedTempFile` + `persist` | Phase 65+ | Crash-safe; must be used for all gate/criteria writes |
| No composability fields on `GatesSpec` | `extends`, `include`, `preconditions` all present | Phase 64 | Wizard must populate all three; old field list is wrong |
| Monolithic `wizard.rs` | Split `wizard/` submodule (planned) | Phase 67 (this phase) | Enables growth for MCP/TUI without bloat |
| `compose::resolve` not available | All four `compose::*_library` functions present | Phase 65 | `criteria new` just needs a thin wrapper |

**Deprecated/outdated:**
- The existing `write_gates_toml` private function writes `extends: None, include: vec![], preconditions: None` hardcoded — this is intentional for milestone wizard (no composability needed), but the gate wizard's equivalent must populate all three from input.

## Open Questions

1. **Where does `load_gate_for_edit` helper live?**
   - What we know: CONTEXT.md marks this as Claude's Discretion. The function calls `load_spec_entry_with_diagnostics` and pattern-matches on `SpecEntry::Directory`.
   - What's unclear: Whether the CLI needs it (it does, for prompting defaults) or core needs it (core only needs `GatesSpec` input, not to load it).
   - Recommendation: Put in CLI (`commands/gate.rs`), not core. Core receives the full `GatesSpec` in `GateWizardInput` (edit mode passes `overwrite: true`). Loading the existing spec is a surface concern.

2. **Final confirm prompt style**
   - What we know: CONTEXT.md says Claude's Discretion — either `Confirm` or summary-then-Select.
   - Recommendation: Use `Confirm::new().with_prompt("Write gate? [y/N]").default(false)` for consistency with `plan.rs` which uses `Confirm` for boolean gates. The summary-then-Select approach adds complexity without benefit for a linear flow.

3. **`dialoguer::Input::with_initial_text` for edit-mode defaults**
   - What we know: `dialoguer` 0.12.0 ships `with_initial_text` on `Input`. Not used in the current codebase.
   - Confidence: MEDIUM — verified in dialoguer 0.11 release notes; 0.12 changelog confirms it's present.
   - Recommendation: Use `with_initial_text` for all edit-mode defaults (name, description, extends, etc.). For `Select`/`MultiSelect`, find the index of the current value and pass it as `.default(idx)`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `tempfile::TempDir` |
| Config file | `Cargo.toml` `[dev-dependencies]` (workspace) |
| Quick run command | `cargo test -p assay-core --lib wizard 2>&1` |
| Full suite command | `just test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WIZC-01 | `apply_gate_wizard()` creates new `gates.toml` with criteria, extends, include | Integration (TempDir) | `cargo test -p assay-core --test wizard_gate apply_gate_wizard_creates` | ❌ Wave 0 |
| WIZC-01 | Create mode fails if `gates.toml` already exists | Integration (TempDir) | `cargo test -p assay-core --test wizard_gate apply_gate_wizard_collision` | ❌ Wave 0 |
| WIZC-01 | Slug validation rejects invalid input | Unit | `cargo test -p assay-core --lib wizard::gate::tests slug_rejected` | ❌ Wave 0 |
| WIZC-01 | `assay gate wizard` non-TTY returns exit code 1 | Unit (CLI) | `cargo test -p assay-cli -- handle_wizard_non_tty` | ❌ Wave 0 |
| WIZC-02 | `apply_gate_wizard(overwrite: true)` overwrites existing gate atomically | Integration (TempDir) | `cargo test -p assay-core --test wizard_gate apply_gate_wizard_edit_overwrites` | ❌ Wave 0 |
| WIZC-02 | Edit mode `--edit missing-gate` returns `SpecNotFoundDiagnostic` | Unit (CLI) | `cargo test -p assay-cli -- handle_wizard_edit_not_found` | ❌ Wave 0 |
| WIZC-03 | `apply_criteria_wizard()` creates `<assay_dir>/criteria/<slug>.toml` | Integration (TempDir) | `cargo test -p assay-core --test wizard_criteria apply_criteria_wizard_creates` | ❌ Wave 0 |
| WIZC-03 | `scan_libraries()` returns all libraries for `criteria list` | Integration (TempDir) | Already tested via Phase 65; reuse `compose::scan_libraries` tests | ✅ |
| WIZC-03 | `assay criteria list` prints `<slug>  <N>` format | Unit (output capture) | `cargo test -p assay-cli -- criteria_list_format` | ❌ Wave 0 |
| WIZC-03 | `assay criteria new` non-TTY returns exit code 1 | Unit (CLI) | `cargo test -p assay-cli -- handle_criteria_new_non_tty` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-core --lib` (unit tests, < 5s)
- **Per wave merge:** `just test` (full suite)
- **Phase gate:** `just ready` (fmt-check + lint + test + deny) before `/kata:verify-work`

### Wave 0 Gaps
- [ ] `crates/assay-core/tests/wizard_gate.rs` — covers WIZC-01, WIZC-02 core functions
- [ ] `crates/assay-core/tests/wizard_criteria.rs` — covers WIZC-03 core function
- [ ] CLI unit tests in `crates/assay-cli/src/commands/criteria.rs` `#[cfg(test)]` block — non-TTY guard, `criteria list` output format

*(Existing `crates/assay-core/tests/wizard.rs` covers the milestone wizard and must not be broken by the module split.)*

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection: `crates/assay-core/src/wizard.rs` — existing wizard patterns, `write_gates_toml`, `CriterionInput`
- Direct codebase inspection: `crates/assay-core/src/spec/compose.rs` — `validate_slug`, `save_library`, `scan_libraries`, `load_library_by_slug`, `resolve`
- Direct codebase inspection: `crates/assay-core/src/error.rs` — all existing `AssayError` variants; no `GateAlreadyExists` variant exists (uses `Io` with `AlreadyExists` kind)
- Direct codebase inspection: `crates/assay-cli/src/commands/plan.rs` — dialoguer prompt patterns to mirror exactly
- Direct codebase inspection: `crates/assay-cli/src/main.rs` — `Command` enum; `Criteria` variant wiring pattern
- Direct codebase inspection: `crates/assay-cli/src/commands/gate.rs` — `GateCommand` enum; `Wizard` variant insertion point
- Direct codebase inspection: `crates/assay-types/src/gates_spec.rs` — `GatesSpec` full field list with serde attrs
- Direct codebase inspection: `crates/assay-types/src/criteria_library.rs` — `CriteriaLibrary` full field list
- Direct codebase inspection: `Cargo.toml` — `dialoguer = "0.12.0"` confirmed as workspace dep; no additional deps needed

### Secondary (MEDIUM confidence)
- `dialoguer` 0.12 API: `Input::with_initial_text`, `Input::validate_with`, `MultiSelect::new()` — confirmed via codebase version pin and dialoguer changelog; not exercised in current codebase but documented

### Tertiary (LOW confidence)
- None — all findings are HIGH or MEDIUM, grounded in direct codebase inspection.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps confirmed in `Cargo.toml` workspace, codebase already using them
- Architecture: HIGH — directly derived from existing patterns in `wizard.rs`, `plan.rs`, `compose.rs`, `main.rs`
- Pitfalls: HIGH — identified from actual code (private fn accessibility, `deny_unknown_fields`, module split hazards)

**Research date:** 2026-04-12
**Valid until:** Indefinite for this codebase — findings are based on committed code, not external sources
