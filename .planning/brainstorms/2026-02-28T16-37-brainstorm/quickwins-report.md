# Quick Wins — Final Report

> Brainstorm: 2026-02-28 | Explorer: explorer-quickwins | Challenger: challenger-quickwins
> Status: **Converged after 3 rounds of debate**

## Summary

5 pressure-tested quick-win proposals for Assay, scoped to ~10 hours total. Each delivers working, tested code. Ordered by dependency chain — each builds on the previous.

2 original proposals were **cut** (stub CLI subcommands, plugin skill) and 2 were **merged** (CLI subcommands folded into config loading and spec validation). The original 7 ideas became 5 tighter deliverables.

---

## Proposal 1: Error Type Foundation

**Effort:** ~1.5 hours | **Priority:** P0 (unblocks everything) | **Crate:** assay-core

**What:** Define a unified `AssayError` enum in `assay-core::error` using `thiserror` (already a dependency). Start minimal — only the variants needed by proposals 3-5.

**Design:**

```rust
// crates/assay-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssayError {
    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    #[error("validation failed for '{field}': {message}")]
    Validation { field: String, message: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[source] std::io::Error),

    #[error("failed to parse config: {0}")]
    Parse(String),
}
```

**Key decisions (from debate):**
- **Unified enum over per-module errors.** At 5 modules with zero consumers, per-module errors add ceremony without benefit. `#[non_exhaustive]` provides the escape hatch for future growth.
- **Structured `Validation { field, message }` over `Validation(String)`.** Enables clean CLI error output (`"Error: field 'name' cannot be empty"`) without string parsing. Zero extra cost.
- **Add variants only when needed.** No speculative variants. Proposals 3-5 define what's needed.

**Deliverables:**
- `crates/assay-core/src/error.rs` — error types
- Re-export from `crates/assay-core/src/lib.rs`
- 2-3 tests confirming error display output

---

## Proposal 2: JSON Schema Generation Pipeline

**Effort:** ~1 hour | **Priority:** P1 (independent, visible output) | **Crate:** assay-types

**What:** Create a standalone binary in `assay-types` that generates JSON Schema files for all public types and writes them to `schemas/`. Add a `just schemas` recipe.

**Design:**
- Binary (not build script) — schemas are generated on demand, not every build
- Uses `schemars::schema_for!()` per type: `Spec`, `Gate`, `Review`, `Workflow`, `Config`
- Writes to `schemas/{type_name}.json`

**Key decisions (from debate):**
- **Standalone binary over build script.** Schema generation is an explicit action, not a build artifact. `cargo run -p assay-types --example generate-schemas` or `just schemas`.
- **Types are flat today and that's fine.** The pipeline has value independent of type richness. Schemas will become more interesting as types evolve (especially after Gate gains `GateKind`).

**Deliverables:**
- `crates/assay-types/examples/generate-schemas.rs` — schema generation binary
- `just schemas` recipe in justfile
- Generated `.json` files in `schemas/`

---

## Proposal 3: Config Loading + CLI `init`

**Effort:** ~2-3 hours | **Priority:** P2 (first real I/O) | **Crates:** assay-types, assay-core, assay-cli

**What:** Implement `Config::load(path)` in assay-core (TOML format). Add `#[serde(default)]` to assay-types for forgiving config parsing. Add `assay init` CLI subcommand that writes a template `assay.toml`.

**Design:**
- **TOML only.** Rust ecosystem convention (Cargo.toml, rustfmt.toml — this project already uses TOML everywhere). JSON stays for data exchange / schemas.
- **Load only, no save.** Round-trip lossless TOML (preserving comments, ordering) is a rabbit hole. `init` writes a static template string, not serialized Rust structs.
- **`#[serde(default)]` on assay-types.** Makes config loading forgiving for new users (missing fields get sensible defaults).

**Key decisions (from debate):**
- **Template-based init over Config::save().** The `init` subcommand writes a handcrafted TOML string with helpful comments. This avoids serialization direction entirely and produces better user-facing output.
- **`toml` dep goes to assay-core, not assay-types.** assay-types stays serialization-framework-agnostic (has serde derives but doesn't pick a format). Config loading is domain logic.
- **Dual-format (TOML+JSON) was cut.** Doubles test surface, forces format-detection design decisions. Not justified at this stage.

**Dependency:** Error types (proposal 1)

**Deliverables:**
- Add `toml` to workspace deps and assay-core's Cargo.toml
- Add `#[serde(default)]` to types in assay-types where appropriate
- `Config::load(path) -> Result<Config, AssayError>` in assay-core/config
- `init` subcommand in assay-cli (writes template `assay.toml`)
- 4-5 tests (valid config, missing fields with defaults, invalid TOML, missing file)

---

## Proposal 4: Spec Validation + CLI `validate`

**Effort:** ~2-3 hours | **Priority:** P3 (first domain logic) | **Crates:** assay-types, assay-core, assay-cli

**What:** Implement `Spec::new(name, description) -> Result<Spec, AssayError>` with validation. Make Spec fields private + add getters. Add `assay validate` CLI subcommand.

**Design:**

```rust
// crates/assay-types - Spec fields become private
pub struct Spec {
    name: String,
    description: String,
}

impl Spec {
    pub fn name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }
}
```

```rust
// crates/assay-core/src/spec
impl Spec {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Result<Spec, AssayError> {
        let name = name.into().trim().to_string();
        let description = description.into().trim().to_string();
        // validate...
    }
}
```

**Validation rules (explicit):**
- **Name:** non-empty after trimming, ≤128 characters
- **Description:** non-empty after trimming, no max length
- **Whitespace-only strings:** fail validation (trim then check empty)
- **No Unicode normalization** (over-engineering for day 1)
- **Trim-then-validate, not reject-then-force-trim.** Accept `"  My Spec  "`, store `"My Spec"`. Forgiving input, strict storage.

**Key decisions (from debate):**
- **`Spec::new()` over builder pattern.** Builder for a 2-field struct is over-engineering. The functional approach (`new()` returns `Result`) is preferred per CLAUDE.md conventions and is idiomatic Rust.
- **Private fields is a breaking change — and that's free.** Zero consumers at 0.1.0. Worth doing now before the API calcifies.
- **Note:** Making fields private means serde `Deserialize` still works (serde can access private fields), but direct struct construction (`Spec { name: "..".into(), .. }`) won't compile. Deserialized specs bypass validation — a known trade-off. Could add `#[serde(try_from)]` later if this matters.

**Dependency:** Error types (proposal 1), Config loading (proposal 3) for the `validate` subcommand

**Deliverables:**
- Make Spec fields private + add getters in assay-types
- `Spec::new()` with validation in assay-core/spec
- `validate` subcommand in assay-cli (loads config, validates all specs)
- 5-6 tests (valid spec, empty name, whitespace-only, too-long name, empty description, trimming behavior)

---

## Proposal 5: Gate Enum Dispatch

**Effort:** ~2.5-3 hours | **Priority:** P4 (core differentiator) | **Crates:** assay-types, assay-core

**What:** Add `GateKind` enum to Gate in assay-types. Remove the `passed: bool` field. Implement sync `evaluate_gate()` function in assay-core with match-based dispatch.

**Design:**

```rust
// crates/assay-types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Gate {
    pub name: String,
    pub kind: GateKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum GateKind {
    AlwaysPass,
    Command { cmd: String },
}
```

```rust
// crates/assay-core/src/gate
pub fn evaluate_gate(gate: &Gate) -> Result<bool, AssayError> {
    match &gate.kind {
        GateKind::AlwaysPass => Ok(true),
        GateKind::Command { cmd } => {
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .status()?;
            Ok(status.success())
        }
    }
}
```

**Key decisions (from debate):**

- **`passed` field removed entirely.** Gate is pure configuration ("what to check"), not a record of whether it was checked. Runtime state (pass/fail, timestamps, logs) belongs to a future `WorkflowRun` type, not the gate definition. Mixing config and state in one type is a design smell.

- **Enum dispatch over trait objects.** `GateKind` enum with a match in `evaluate_gate()` is simpler, serializable, and sufficient. Trait-based dispatch adds complexity (trait objects aren't Serialize/Deserialize/Clone) with no benefit until a plugin system exists.

- **Sync only.** `std::process::Command`, not tokio. Tokio stays unused. Async is a migration for when it's needed.

- **Architectural intent documented outside source.** When plugins require custom gate types, extract `evaluate_gate()` to a `GateEvaluator` trait. This note belongs in CLAUDE.md or a design doc, not a TODO comment in source (TODO comments rot).

**Dependency:** Error types (proposal 1)

**Deliverables:**
- `GateKind` enum in assay-types, `passed` field removed from `Gate`
- `evaluate_gate()` in assay-core/gate
- 3-4 tests (AlwaysPass, Command success, Command failure, invalid command)

---

## Rejected / Deferred Ideas

### CLI Subcommands Skeleton (originally #4) — MERGED
Stub subcommands that print "not implemented" are negative UX. Instead, `init` and `validate` ship as part of proposals 3 and 4 respectively, with real behavior from day one. The `status` subcommand was cut entirely — there's no state to report on.

### Plugin Skill for Claude Code (originally #7) — DEFERRED
This has a 3-deep dependency chain (error types → config loading → CLI init → plugin skill). It's a capstone, not a quick win. Belongs in the high-value features brainstorm once the CLI actually does something meaningful.

---

## Dependency Graph

```
Proposal 1: Error Types
    ↓           ↓           ↓
Proposal 3   Proposal 4   Proposal 5
Config+Init  Spec+Validate Gate Dispatch
    ↓
Proposal 4 (validate subcommand loads config)

Proposal 2: Schema Generation (independent)
```

## Execution Order

| Order | Proposal | Effort | Depends On |
|-------|----------|--------|------------|
| 1 | Error Type Foundation | 1.5hrs | — |
| 2 | Schema Generation Pipeline | 1hr | — |
| 3 | Config Loading + CLI `init` | 2-3hrs | #1 |
| 4 | Spec Validation + CLI `validate` | 2-3hrs | #1, #3 |
| 5 | Gate Enum Dispatch | 2.5-3hrs | #1 |

**Proposals 1 and 2 can be parallelized.** Proposals 3 and 5 can be parallelized after 1 completes. Proposal 4 depends on both 1 and 3.

**Total estimate: ~10 hours of focused work.**
