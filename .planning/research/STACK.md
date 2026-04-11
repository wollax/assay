# Technology Stack

**Project:** Assay v0.7.0 — Gate Composability
**Researched:** 2026-04-11
**Scope:** NEW capabilities only. Existing stack is pre-validated and not repeated here.

---

## What Is Already In Place (Context Only)

| Capability | Current State |
|------------|---------------|
| Workspace | 7 crates: assay-types, assay-core, assay-cli, assay-tui, assay-mcp, assay-harness, assay-backends |
| Serialization | serde 1, serde_json 1, schemars 1, toml 1 |
| CLI interactive | **dialoguer 0.12.0** — in workspace, actively used in `assay-cli/commands/plan.rs` |
| TUI interactive | ratatui 0.30 + crossterm 0.28 — wizard state machine in `assay-tui/src/wizard.rs` |
| Wizard core | `assay-core/src/wizard.rs` — pure functions, surface-agnostic, called by CLI + MCP + TUI |
| Graph/cycle | Custom DAG in `assay-core/src/orchestrate/dag.rs` using Vec adjacency lists (no petgraph) |
| Ordered maps | **indexmap 2** (with serde feature) — workspace dep |
| Gate config type | `GatesSpec` with `#[serde(deny_unknown_fields)]` and `#[serde(default, skip_serializing_if)]` on all optional fields |
| Criterion type | `Criterion` with `#[serde(deny_unknown_fields)]`, 9 optional fields, established extension pattern |

---

## New Capabilities Required

### Capability 1: Gate Inheritance (`gate.extends`)

**What:** `GatesSpec` gains `extends: Option<String>` referencing a parent gate by slug.
At evaluation load time, the resolver walks the chain, merges criteria (child overrides
parent by name), and detects cycles.

**New fields on `GatesSpec` in `assay-types`:**

```toml
# Example gates.toml using extends
name = "api-auth"
extends = "base-quality"   # inherits criteria from .assay/specs/base-quality/gates.toml

[[criteria]]
name = "auth-header-present"  # overrides a criterion of the same name in parent, if any
description = "All API responses include auth header"
cmd = "cargo test auth_header"
```

**Type change:** Add `extends: Option<String>` to `GatesSpec` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`. This is the
exact same pattern used by 5 other optional fields on `GatesSpec` today.
No migration needed — absent field deserializes to `None`.

**Merge logic:** Use `IndexMap<String, Criterion>` from the existing `indexmap 2` workspace dep.
Walk the inheritance chain (deepest ancestor first), insert criteria into the map; later
insertions (closer to the child) override by name. Collect back to `Vec<Criterion>`. This
gives last-wins-by-name semantics with insertion-order preservation.

**Cycle detection:** New `assay-core/src/gate/compose.rs`. The algorithm is a DFS with
a `visited` set — identical to the existing `assay-core/src/milestone/cycle.rs`. No new
data structure needed beyond a `HashSet<String>` (stdlib).

**New code, zero new deps.**
Confidence: HIGH.

---

### Capability 2: Criteria Libraries (`includes`)

**What:** A `CriteriaLibrary` type stored in `.assay/criteria/<slug>.criteria.toml`.
`GatesSpec` gains `includes: Vec<String>` listing library slugs whose criteria are
prepended before the spec's own criteria (before `extends` resolution).

**New type in `assay-types/src/criteria_library.rs`:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CriteriaLibrary {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub criteria: Vec<Criterion>,
}
```

**New field on `GatesSpec`:** `includes: Vec<String>` with
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`.

**Load path:** New `assay-core/src/gate/library.rs` with a `load_criteria_library(slug,
assay_dir)` function. Pattern is identical to `load_spec_entry_with_diagnostics` in
`assay-core/src/spec/` — read TOML file, deserialize, return typed result with
`AssayError::Io` on failure.

**Breaking change risk:** None. `includes` is absent from all existing `gates.toml` files;
`#[serde(default)]` means they parse to empty Vec. `GatesSpec` has `deny_unknown_fields`
but the new field is being *added*, not encountered unexpectedly.

**New code, zero new deps.**
Confidence: HIGH.

---

### Capability 3: Spec Preconditions

**What:** `GatesSpec` gains `preconditions: Vec<Criterion>` — criteria evaluated before
the main gate criteria. If any precondition fails, the main criteria are not evaluated and
a distinct `PreconditionFailed` result is returned.

**New field on `GatesSpec`:** `preconditions: Vec<Criterion>` with
`#[serde(default, skip_serializing_if = "Vec::is_empty")]`.

**Evaluation change:** In `assay-core/src/gate/mod.rs`'s `evaluate_all_gates`, prepend a
precondition phase. Return a new result variant or a new field on `GateRunRecord`
(e.g. `precondition_result: Option<EnforcementSummary>`) that CLI and MCP surfaces can
display distinctly.

**New code, zero new deps.**
Confidence: HIGH.

---

### Capability 4: Gate Wizard — CLI Surface

**What:** A new `assay gate wizard` subcommand (or `assay gate create`) that interactively
builds a `gates.toml` with composability options: choose a parent to extend, select library
includes, add preconditions, add criteria.

**Existing dialoguer 0.12.0 capabilities sufficient:**
- `Input::new()` — free text (criterion name, description, command)
- `Select::new()` — single-choice list (choose parent gate from discovered specs)
- `MultiSelect::new()` — multi-choice (choose library includes from discovered libraries)
- `Confirm::new()` — yes/no (add another criterion?, add preconditions?)

**Pattern:** New `assay-cli/src/commands/gate_wizard.rs`. Identical structure to the
existing `plan.rs`: TTY guard at top, dialoguer prompts, collect into a typed input struct,
delegate to a pure `assay-core` function. The pure core function is the single source of
truth used by CLI, MCP, and TUI.

**No new deps.** dialoguer is already a workspace dep, already imported in `assay-cli`.
Confidence: HIGH.

---

### Capability 5: Gate Wizard — TUI Surface

**What:** A new wizard flow in `assay-tui` for creating/editing gate definitions via
keyboard, consistent with the existing `WizardState` / `WizardAction` / `handle_wizard_event`
/ `draw_wizard` pattern in `assay-tui/src/wizard.rs`.

**Extension approach:**
- Add `GateWizardState` struct (mirrors `WizardState` shape)
- Add `GateWizardAction` enum (mirrors `WizardAction`)
- Add `handle_gate_wizard_event()` and `draw_gate_wizard()` functions
- For selection steps (parent gate, library includes): extend state with
  `selection_options: Vec<String>` and `selected_indices: Vec<usize>` fields,
  handled by `KeyCode::Up/Down/Space` in the event handler

**Why not `tui-input` or `ratatui-textarea`:** The existing wizard uses raw
`KeyCode::Char` accumulation into `Vec<String>` buffers, rendered with
`ratatui::widgets::Paragraph`. This handles all needed input shapes. Introducing a
text-input widget library for one feature would be inconsistent with the established
pattern and adds a dep for zero functional gain.

**No new deps.**
Confidence: HIGH.

---

### Capability 6: Gate Wizard — MCP Surface

**What:** New MCP tools that expose composability to agent-driven workflows without TTY:
- `gate_library_list` — list available criteria libraries in `.assay/criteria/`
- `gate_library_get` — fetch a named library's criteria
- `gate_wizard_create` — create a `gates.toml` from structured parameters (extends, includes, preconditions, criteria)

**Pattern:** New tool registrations in `assay-mcp/src/`. Each calls the same pure core
functions used by CLI/TUI via `tokio::task::spawn_blocking` (existing pattern for sync
work in async MCP handlers).

**No new deps.**
Confidence: HIGH.

---

## No New Workspace Dependencies Required

All six capabilities are implemented with existing workspace dependencies:

| Existing Dep | Role in v0.7.0 |
|---|---|
| `indexmap 2` | `IndexMap<String, Criterion>` for name-keyed criterion merging in `extends` resolution |
| `toml 1` | Loading `.assay/criteria/<slug>.criteria.toml` files |
| `serde` + `schemars` | New `CriteriaLibrary` type derives; new optional fields on `GatesSpec` |
| `dialoguer 0.12.0` | `Select` + `MultiSelect` for parent/library picker in CLI wizard |
| `ratatui 0.30` + `crossterm 0.28` | TUI wizard steps for selection/multi-selection |
| `std::collections::HashSet` | Cycle detection DFS visited set (stdlib) |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Inheritance graph | Custom DFS with `HashSet` (stdlib) | petgraph | Codebase explicitly avoids petgraph (see `dag.rs`); gate graphs are tiny (tens of nodes) |
| Name-keyed criterion merge | `indexmap 2` (already present) | `HashMap` | indexmap preserves insertion order (deterministic output); already a workspace dep |
| TUI text input | Existing raw char accumulation | `ratatui-textarea` | Inconsistent with existing wizard; adds dep for no functional gain |
| CLI interactive | `dialoguer 0.12.0` (already present) | `inquire` | dialoguer already in use; MultiSelect covers the new use case |
| Library file format | TOML (existing `toml 1`) | JSON, YAML | TOML is the project convention for all config files; `serde_yaml` is present but used for different purposes |

---

## Integration Points Summary

| Feature | assay-types | assay-core | assay-cli | assay-mcp | assay-tui |
|---------|-------------|------------|-----------|-----------|-----------|
| `gate.extends` | `GatesSpec.extends: Option<String>` | `gate/compose.rs` — chain loader + cycle detector + IndexMap merger | No change (transparent at load time) | No change | No change |
| Criteria libraries | New `CriteriaLibrary` type; `GatesSpec.includes: Vec<String>` | `gate/library.rs` — library loader | `gate library list/get` subcommands | `gate_library_list/get` tools | Library picker in wizard |
| Spec preconditions | `GatesSpec.preconditions: Vec<Criterion>` | Pre-flight phase in `evaluate_all_gates` | New output section | New field on `gate_run` result | Result panel display |
| Gate wizard | None (reuses existing types) | `wizard.rs` extended with `create_gate_from_inputs()` pure fn | `gate wizard` subcommand | `gate_wizard_create` tool | New `GateWizardState` |

---

## Sources

- Codebase inspection (2026-04-11):
  - `crates/assay-types/src/gates_spec.rs` — `GatesSpec` struct, `deny_unknown_fields`, optional field patterns
  - `crates/assay-types/src/criterion.rs` — `Criterion` struct and all 9 optional fields
  - `crates/assay-core/src/wizard.rs` — pure wizard function pattern (surface-agnostic)
  - `crates/assay-tui/src/wizard.rs` — TUI wizard state machine (raw char accumulation, no widget lib)
  - `crates/assay-cli/src/commands/plan.rs` — dialoguer usage (Input, Select, Confirm confirmed; MultiSelect available)
  - `crates/assay-core/src/orchestrate/dag.rs` — custom DAG; explicit comment rejecting petgraph
  - `crates/assay-core/src/milestone/cycle.rs` — DFS cycle detection pattern to reuse
  - Root `Cargo.toml` — dep versions: dialoguer 0.12.0, ratatui 0.30, crossterm 0.28, indexmap 2, toml 1
