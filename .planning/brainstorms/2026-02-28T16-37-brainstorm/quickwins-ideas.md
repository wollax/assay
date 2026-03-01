# Quick-Win Ideas — Assay Brainstorm

> Explorer: explorer-quickwins | Date: 2026-02-28

These are low-effort, high-impact proposals targeting the gap between the current skeleton and a minimally usable product. Each is scoped to ≤1 day of focused work.

---

## 1. Error Type Foundation

**What:** Define a unified `AssayError` enum in `assay-core` using `thiserror` (already a dependency). Cover the obvious variants: `SpecNotFound`, `InvalidSpec`, `GateFailure`, `ConfigError(io/parse)`, `WorkflowError`. Re-export from `assay-core`'s public API.

**Why:** Every module is a stub. You can't build anything real until errors can propagate. This is the load-bearing wall that unblocks all other domain logic. Without it, the next person to touch `assay-core` will either invent ad-hoc errors or use `anyhow`/`String`, creating inconsistency. Getting this right early shapes the entire API surface.

**Scope:** ~2 hours. One file (`crates/assay-core/src/error.rs`), one `thiserror` enum, re-export in `lib.rs`.

**Risks:**
- Over-engineering the enum with too many variants before the domain is understood. Mitigate by keeping it minimal (5-7 variants) and annotating with `#[non_exhaustive]`.
- Choosing between `thiserror` (library errors) vs `color-eyre` (application errors). Both are already in the workspace — use `thiserror` in core, `color-eyre` in binaries.

---

## 2. Config File Loading (TOML/JSON Round-Trip)

**What:** Implement `Config::load(path)` and `Config::save(path)` in `assay-core/src/config/mod.rs`. Support both TOML and JSON formats (detect by extension). Add `toml` to workspace dependencies. Write tests that serialize → deserialize → assert equality.

**Why:** Config loading is the entry point for every user interaction. Without it, neither the CLI nor TUI can actually do anything useful. This also exercises the `assay-types` structs for the first time with real serialization, which will surface any type design issues early. It's the quickest path to "something that reads a file and does a thing."

**Scope:** ~3-4 hours. Add `toml` dep, implement two functions, write 4-5 tests.

**Risks:**
- TOML representation of nested types (Vec<Spec>, Vec<Gate>) may be awkward, pushing toward JSON-only. Mitigate by testing both and accepting TOML as "good enough" for flat configs.
- Config schema may need to evolve. Mitigate by not committing to backwards compat at 0.1.0.

---

## 3. JSON Schema Generation Pipeline

**What:** Create a binary/example in `assay-types` that uses `schemars` to generate JSON Schema files for all public types (`Spec`, `Gate`, `Review`, `Workflow`, `Config`) and writes them to `schemas/`. Add a `just schemas` recipe.

**Why:** The `schemas/` directory and its README already promise this exists but it doesn't. This is the plugin contract — Claude Code, Codex, and OpenCode plugins need schema files to know how to talk to Assay. It's a force-multiplier: once schemas exist, plugin authors (including AI agents) can validate their payloads without reading Rust source. Also exercises `schemars` derives that are already on every type.

**Scope:** ~1-2 hours. One example binary, one justfile recipe, generated `.json` files.

**Risks:**
- `schemars 0.8` output may differ from what plugin tooling expects. Acceptable at this stage — the schemas are for guidance, not formal contracts.
- Types may need `#[schemars(rename)]` or `#[schemars(description)]` attributes for cleaner output. Easy to add incrementally.

---

## 4. CLI Subcommands Skeleton (`init`, `validate`, `status`)

**What:** Add three clap subcommands to `assay-cli`: `init` (create a default `assay.toml` in cwd), `validate` (parse and validate a config file), `status` (print workflow status summary). Each prints placeholder output initially but structures the CLI for real functionality.

**Why:** The CLI currently prints a version string and exits. That's useless. Three subcommands give users a mental model of what Assay does: you initialize a project, validate your config, and check status. Even with stub implementations, it makes the project *feel* real and testable. If config loading (idea #2) lands first, `init` and `validate` become immediately functional.

**Scope:** ~2-3 hours. Expand `Cli` struct with subcommands, add match arms, implement stubs.

**Risks:**
- Committing to a CLI UX too early. Mitigate by keeping subcommands minimal and marking as unstable.
- Subcommand proliferation. Mitigate by limiting to 3 and requiring justification for more.

---

## 5. Spec Validation with Builder Pattern

**What:** Implement a `SpecBuilder` in `assay-core/src/spec/mod.rs` that validates specs before construction: name must be non-empty, description must be non-empty, optional acceptance criteria field. Return `Result<Spec, AssayError>` (depends on idea #1). Add unit tests for valid and invalid specs.

**Why:** `Spec` is the core domain object — everything flows from specs. Right now it's a plain struct with public fields, meaning anyone can create an invalid `Spec { name: "", description: "" }`. A builder with validation establishes the pattern for all domain objects and makes the "spec-driven" promise real. It's also the simplest domain logic to implement because specs have no dependencies on other types.

**Scope:** ~2-3 hours. Builder struct, validation logic, 5-6 unit tests.

**Risks:**
- Builder pattern may feel heavy for a struct with 2 fields. Counter: it's about validation, not construction complexity. The builder enforces invariants.
- Public fields on `Spec` conflict with builder-enforced validation. May need to make fields private + add getters. This is a small breaking change but worth doing early.

---

## 6. Gate as Pluggable Check (Trait-Based Design)

**What:** Define a `GateCheck` trait in `assay-core/src/gate/mod.rs` with a single method: `fn check(&self, context: &GateContext) -> Result<GateResult, AssayError>`. Implement two built-in checks: `AlwaysPass` (testing) and `CommandGate` (runs a shell command, passes if exit code 0). The current `Gate` struct becomes metadata; `GateCheck` is the execution interface.

**Why:** The `Gate` struct currently has a `passed: bool` field — a static snapshot, not a dynamic check. This is the biggest type design smell in the codebase. Gates are the product's core differentiator ("gated quality checks"). Making them trait-based means plugins can define custom gates (linter gates, test gates, LLM review gates) without touching core. This single abstraction unlocks the entire plugin ecosystem.

**Scope:** ~3-4 hours. Trait definition, context struct, two implementations, tests.

**Risks:**
- Trait design may need async (`async fn check`). Tokio is already in the workspace but unused. Could start sync and migrate later, or use `async-trait` / RPITIT from Rust 2024 edition.
- `CommandGate` running shell commands is inherently unsafe. Mitigate with documentation + a `--no-exec` flag for dry runs.
- Over-abstraction this early. Counter: a single trait with one method is minimal; the risk is *not* doing this and having `passed: bool` calcify.

---

## 7. First Plugin Skill: `assay init` for Claude Code

**What:** Create a Claude Code skill in `plugins/claude-code/skills/` that runs `assay init` in the user's project directory. The skill prompt guides the agent to create an `assay.toml` with appropriate specs based on the project's README and existing code. Wire up the plugin.json to register the skill.

**Why:** The plugins directory is scaffolded but completely empty (only `.gitkeep` files). Shipping one real skill demonstrates the end-to-end flow: plugin → CLI → core. It also dogfoods the tool — if Assay is an "agentic development kit," at least one agent should actually use it. This gives the project a compelling demo: "install the plugin, run the skill, and get a spec-driven workflow in 30 seconds."

**Scope:** ~2-3 hours. One markdown skill file, update plugin.json, write skill prompt.

**Risks:**
- Depends on the CLI `init` subcommand existing (idea #4). Without it, the skill has no tool to invoke.
- Skill quality is hard to test without running it in Claude Code. Mitigate by keeping the skill prompt simple and focused.
- Plugin contract isn't formally defined yet. Mitigate by treating this as a prototype that validates the contract.
