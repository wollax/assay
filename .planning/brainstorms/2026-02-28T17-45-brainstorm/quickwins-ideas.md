# Quick Wins — Explorer Ideas (Round 2)

> Explorer: explorer-quickwins | Date: 2026-02-28
> Context: Second brainstorm. First brainstorm produced 5 proposals (~10hrs). This session revisits with fresh eyes, considering what might be missing or reordered.

## Codebase Reality Check

What actually exists today:
- **assay-types:** 5 flat structs with `pub` fields, serde + schemars derives. No validation, no enums, no newtypes.
- **assay-core:** 5 empty modules with doc comments only. Zero logic. `thiserror` is a dep but unused.
- **assay-cli:** Clap skeleton, prints version. No subcommands.
- **assay-tui:** Ratatui skeleton, renders title, quits on `q`. Functional but inert.
- **plugins/:** Three plugin scaffolds (claude-code, codex, opencode) with empty dirs and READMEs. Claude-code has a hooks.json.
- **schemas/:** Empty except README.
- **Workspace deps:** serde, serde_json, schemars, tokio, clap, ratatui, crossterm, thiserror, color-eyre. All declared, not all used.

The previous brainstorm's proposals are solid but can be sharpened. Here are my ideas for the first milestone — what to build, in what order, and why.

---

## Idea 1: Error Types + Result Alias

**What:** Define `AssayError` in `assay-core::error` using thiserror. Include a `pub type Result<T> = std::result::Result<T, AssayError>` alias for ergonomics. Start with only the variants needed by other quick wins: `Config`, `Validation`, `Io`, `Gate`.

**Why:** Every other quick win returns `Result`. Without a unified error type, you'll either use `anyhow` (which you don't want in a library crate) or proliferate ad-hoc error handling. This is 90 minutes that unblocks everything. The Result alias is a tiny addition that eliminates `-> Result<T, AssayError>` boilerplate across the entire core crate.

**Scope:** ~1.5 hours. Error enum, ConfigError sub-enum, Result alias, display tests.

**Risks:**
- Over-scoping variants before they're needed. Mitigated by `#[non_exhaustive]` and the discipline of "add when consumed."
- The Result alias could shadow `std::result::Result` — but this is idiomatic Rust (tokio, axum, anyhow all do it).

---

## Idea 2: Domain Model Hardening (Types Redesign)

**What:** Redesign `assay-types` in one pass rather than spread across proposals 3-5 of the previous brainstorm:
- Make `Spec` fields private, add getters
- Replace `Gate { passed: bool }` with `Gate { name, kind: GateKind }` where `GateKind` is a `#[serde(tag = "type")]` enum
- Add `GateResult { gate_name, passed, evidence, duration, timestamp }` for runtime state
- Add `#[serde(default)]` where forgiving parsing is desired
- Add `SpecCriteria` enum to Spec (deterministic shell command vs. agent-evaluated assertion) — the dual-track differentiator shows up in the type system on day one

**Why:** The previous brainstorm correctly identifies all these changes but spreads them across 3 proposals with interleaved CLI work. Doing the type redesign as a single atomic pass is cleaner: one PR, one review, one coherent type system. CLI commands that consume these types come after, not interleaved. This respects the project's own decision to "start with domain model before any UI/orchestration."

**Scope:** ~2-3 hours. This is larger than any single previous proposal but replaces the type-related work from proposals 3, 4, and 5.

**Risks:**
- Larger diff = harder to review. Mitigated by it all being in one crate (`assay-types`) with no logic, just data shapes.
- Designing `SpecCriteria` without having built the spec engine is speculative. Counter: the dual-track concept is a key decision, not an open question. Getting it into types early means everything else builds on the right foundation.
- `GateResult` might be premature if workflows aren't scoped yet. But it's a pure data type with no behavior — costs almost nothing to define, and it formally separates config from state.

---

## Idea 3: Schema Generation Pipeline

**What:** Create `crates/assay-types/examples/generate-schemas.rs` that uses `schemars::schema_for!()` to emit JSON Schema for every public type into `schemas/`. Add `just schemas` recipe.

**Why:** This was proposal #2 in the previous brainstorm and remains a great quick win. It's independent of everything else, produces visible artifacts, and validates that your type derives are correct. Once the domain model is hardened (Idea 2), the schemas become much more interesting — showing GateKind variants, SpecCriteria variants, etc.

**Scope:** ~1 hour. Trivially parallelizable with any other work.

**Risks:**
- Nearly none. Worst case: schemas reveal schemars doesn't handle your `#[serde(tag)]` patterns well and you discover this early. That's a feature, not a bug.

---

## Idea 4: Config Loading (Core Only, No CLI)

**What:** Implement `Config::load(path) -> Result<Config, AssayError>` in `assay-core::config`. Add `toml` as a workspace dependency. TOML only. Pure function: takes a path, returns a parsed+validated Config or an error.

**Why:** The previous brainstorm bundles config loading with `assay init`. I'd split them. Config loading is core domain logic that the TUI, CLI, MCP server, and tests all need. The `init` subcommand is presentation-layer sugar. Shipping config loading as a standalone unit means:
- It's testable in isolation (no CLI harness needed)
- It's consumable from day one by any surface
- The init command can come later as a trivial consumer

**Scope:** ~1.5-2 hours. `Config::load()`, `Config::from_str()`, 4-5 tests (valid, invalid TOML, missing fields with defaults, missing file).

**Risks:**
- Without `assay init`, how do users create their first config? Answer: They don't need to yet. The README/docs can show a template. `init` is UX polish, not a foundation block.
- Config shape might change as types evolve. Mitigated: `#[serde(default)]` makes the format tolerant of additions.

---

## Idea 5: Spec Validation (Core Only, No CLI)

**What:** Implement `Spec::new(name, description) -> Result<Spec, AssayError>` with validation rules (non-empty after trim, name ≤ 128 chars). Pure function, no side effects, no CLI.

Also add a `Config::validate(&self) -> Result<(), AssayError>` that validates all specs within a loaded config — connecting config loading to spec validation at the domain level.

**Why:** Same reasoning as Idea 4. Validation is domain logic. The `assay validate` subcommand is just `Config::load(path)?.validate()?` with CLI output formatting. By keeping these in core, the TUI and MCP server get validation for free.

**Scope:** ~1.5-2 hours. `Spec::new()`, `Config::validate()`, 6-7 tests.

**Risks:**
- Validation rules are somewhat arbitrary (why 128 chars?). Accept this — pick reasonable limits, document them, change later if needed.
- `Config::validate()` couples config to spec validation. Counter: Config contains specs — it's natural for config validation to validate its contents.

---

## Idea 6: Gate Evaluation (Sync, Core Only)

**What:** Implement `evaluate_gate(gate: &Gate) -> Result<GateResult, AssayError>` in `assay-core::gate`. Match on `GateKind`:
- `AlwaysPass` → returns success immediately
- `Command { cmd }` → runs `sh -c "$cmd"`, captures exit code + stdout/stderr into GateResult

Return a proper `GateResult` with evidence, not just `bool`. This is where the dual-track story starts: deterministic gates execute shell commands; agent-evaluated gates will come later but the type system already has room for them via `GateKind`.

**Why:** This is the first piece of real runtime behavior in the entire project. It connects the type system to actual execution. Every future demo, test, and integration depends on being able to evaluate a gate. And by returning `GateResult` (with evidence, duration) instead of a bare `bool`, you get observability from day one.

**Scope:** ~2-2.5 hours. `evaluate_gate()`, GateResult construction, 4-5 tests (AlwaysPass, command success, command failure, command not found, evidence capture).

**Risks:**
- Shell execution (`sh -c`) is inherently platform-dependent. macOS + Linux are fine; Windows would need `cmd /c`. Accept this limitation — Assay targets Unix-like environments.
- Capturing stdout/stderr adds complexity vs. just checking exit code. Worth it: evidence capture is what makes gates useful. A gate that says "test failed" is worse than a gate that says "test failed: 3 tests failed in auth module."

---

## Idea 7: CLI Subcommands (`init` + `validate` + `gate run`)

**What:** Wire up three CLI subcommands that delegate to core:
- `assay init` — writes a template `assay.toml` with comments (hardcoded template string, not serialized)
- `assay validate [path]` — loads config, validates, prints results
- `assay gate run <gate-name> [--config path]` — loads config, finds gate by name, evaluates, prints result

**Why:** This is the capstone that makes the foundation visible and usable. After ideas 1-6 build the engine, this puts a steering wheel on it. Three commands are enough to demo the full loop: create a project, validate it, run a gate. It's also the first time a user can interact with Assay without reading code.

The key insight vs. the previous brainstorm: this comes LAST, not interleaved. The previous proposals mixed CLI work into config and spec work. I'd rather ship 6 units of core logic and then one CLI pass that wires them together.

**Scope:** ~2-3 hours. Three subcommands, human-friendly output formatting, integration tests.

**Risks:**
- Three subcommands in one proposal is ambitious. Mitigated: each is a thin delegation to core. The complexity is in core (already done by this point), not in CLI arg parsing.
- `gate run` requires a config with gates defined. Needs a good template in `init` that includes an example gate.

---

## Proposed Execution Order

```
Phase A (parallel):
  [1] Error Types + Result Alias     (~1.5h)
  [3] Schema Generation Pipeline     (~1h)

Phase B (parallel, after Error Types):
  [2] Domain Model Hardening         (~2-3h)

Phase C (parallel, after Domain Model):
  [4] Config Loading (core)          (~1.5-2h)
  [6] Gate Evaluation (core)         (~2-2.5h)

Phase D (sequential, after Config Loading):
  [5] Spec Validation (core)         (~1.5-2h)

Phase E (after all core, regenerate schemas):
  [3] Re-run schema generation       (~5min)
  [7] CLI Subcommands                (~2-3h)
```

**Total: ~12-15 hours.** Slightly more than the previous brainstorm's 10 hours, but delivers:
- A coherent type system in one pass (not three)
- Core logic that's testable and consumable from any surface
- A CLI that wires everything together at the end
- The dual-track differentiator visible in the type system from day one

## What the Previous Brainstorm Gets Right

1. **Error types first.** Absolutely correct — this unblocks everything.
2. **Schema generation as an independent quick win.** Low effort, high visibility, no dependencies.
3. **Gate dispatch via enum, not trait objects.** Right call for this stage.
4. **TOML only.** No dual-format complexity.
5. **Template-based init, not serialized.** Preserves comments, avoids round-trip issues.
6. **Removing `passed: bool` from Gate.** Config vs. state separation is critical.

## What the Previous Brainstorm Might Miss

1. **Interleaving core logic with CLI.** The previous proposals bundle `Config::load()` with `assay init` and `Spec::new()` with `assay validate`. This couples domain logic to presentation in the first milestone. Better: build all core logic first, then wire CLI.

2. **No `GateResult` type.** The previous proposal has `evaluate_gate()` return `bool`. That throws away evidence (stdout, stderr, duration). GateResult is cheap to define and makes gates actually useful for debugging.

3. **Dual-track criteria not in types yet.** The previous proposals add `GateKind::Command` but don't sketch where agent-evaluated criteria will live. Adding a `SpecCriteria` enum or extending `GateKind` with a placeholder variant signals the differentiator in the type system.

4. **No `Config::validate()`.** The previous brainstorm validates specs but doesn't connect validation to config loading. `Config::validate()` is the natural integration point.

5. **Missing evidence capture in gates.** Just checking exit code vs. capturing stdout/stderr is the difference between "gate failed" and "gate failed because tests X, Y, Z errored." Evidence makes gates debuggable.
