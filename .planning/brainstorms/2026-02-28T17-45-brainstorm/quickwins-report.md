# Quick Wins — Final Report

> Brainstorm: 2026-02-28 | Explorer: explorer-quickwins | Challenger: challenger-quickwins
> Status: **Converged after 3 rounds of debate**

## Summary

7 pressure-tested quick-win proposals for Assay's first milestone, scoped to ~13-16 hours total. Each delivers working, tested code. Ordered by dependency chain.

**Key thesis vs. previous brainstorm:** Build all core logic first as free functions in `assay-core`, then wire CLI subcommands as a final pass. The previous brainstorm interleaved domain logic with CLI presentation (e.g., `Config::load()` bundled with `assay init`, `Spec::new()` bundled with `assay validate`). This revision separates concerns cleanly: `assay-types` stays a pure DTO crate with pub fields, `assay-core` owns all behavior, and `assay-cli` is a thin last-mile wiring.

**Changes from debate:**
| Change | Before | After | Round |
|--------|--------|-------|-------|
| Error variants | Pre-define 4 | Start with `Io` only, add per-consumer | 1 |
| SpecCriteria enum | On Spec type | Dropped. Dual-track lives on GateKind | 1 |
| Spec fields | Private + getters | Pub. Types stay DTOs | 1 |
| Validation | Impls on types structs | Free functions in assay-core | 1 |
| Idea 2 estimate | 2-3h | 3-4h | 1 |
| Gate evaluate | No working dir | Explicit `working_dir: &Path` param | 1 |
| CLI tests | Integration tests | Unit tests of core only | 1 |
| GateResult.evidence | `Vec<String>` | `stdout: String, stderr: String` | 2 |
| Timestamp/duration | `DateTime`/`Duration` | `duration_ms: u64`, timestamp as `String` (ISO 8601) | 2 |
| GateKind hint | Only in docs | Commented-out `AgentEvaluated` variant in code | 2 |

---

## Proposal 1: Error Types + Result Alias

**Effort:** ~1.5 hours | **Priority:** P0 (unblocks everything) | **Crate:** assay-core

**What:** Define a unified `AssayError` enum in `assay-core::error` using `thiserror`. Include a `pub type Result<T> = std::result::Result<T, AssayError>` alias.

**Design:**

```rust
// crates/assay-core/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssayError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AssayError>;
```

**Key decisions:**
- **Start with `Io` only.** Each subsequent proposal adds its own variant when consumed. `#[non_exhaustive]` allows growth without breaking changes.
- **Result alias** eliminates `-> Result<T, AssayError>` boilerplate. Idiomatic Rust (tokio, axum, anyhow all do this).
- **Unified enum over per-module errors.** At 5 modules with zero consumers, per-module errors add ceremony without benefit.

**Deliverables:**
- `crates/assay-core/src/error.rs` — error types + Result alias
- Re-export from `crates/assay-core/src/lib.rs`
- 2-3 tests confirming error display output

---

## Proposal 2: Domain Model Hardening (Types Redesign)

**Effort:** ~3-4 hours | **Priority:** P1 (everything builds on these types) | **Crate:** assay-types

**What:** Redesign `assay-types` in one atomic pass:
- Replace `Gate { name, passed }` with `Gate { name, kind: GateKind }`
- Add `GateKind` enum with `#[serde(tag = "type")]`
- Add `GateResult` struct (runtime state, separate from gate config)
- Add `#[serde(default)]` where forgiving parsing is desired
- All fields remain `pub` — types crate stays a pure DTO layer

**Design:**

```rust
// crates/assay-types/src/lib.rs

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
    // Future: AgentEvaluated { prompt: String }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timestamp: String, // ISO 8601
}
```

**Key decisions (from debate):**
- **Fields stay `pub`.** assay-types is a DTO crate. Private fields + getters on DTOs is fighting the crate's contract. Validation belongs in assay-core as free functions.
- **SpecCriteria dropped.** Dual-track criteria live on `GateKind`, not `Spec`. Spec defines WHAT to build; Gate defines HOW to verify. The project's own abstractions separate these concerns — don't re-merge them.
- **GateResult in types, not core.** Multiple surfaces consume it (CLI display, TUI render, future MCP serialization). It's pure data with no logic — fits the DTO contract.
- **`stdout`/`stderr` over `evidence: Vec<String>`.** More precise, better for display, preserves provenance of output.
- **`duration_ms: u64` and `timestamp: String`.** Avoids chrono dependency. Portable, schemars-friendly.
- **Commented-out `AgentEvaluated` variant.** Makes dual-track intent visible in code without committing to a design.
- **`passed: bool` removed from Gate.** Gate is config ("what to check"). Runtime state lives in GateResult.

**Deliverables:**
- Redesigned types in `crates/assay-types/src/lib.rs`
- Verify schemars derives correctly with `#[serde(tag = "type")]` on GateKind
- Ensure all existing code still compiles (assay-core, assay-cli, assay-tui)

---

## Proposal 3: Schema Generation Pipeline

**Effort:** ~1 hour | **Priority:** P1 (independent, visible output) | **Crate:** assay-types

**What:** Create a standalone binary that generates JSON Schema files for all public types. Add a `just schemas` recipe.

**Design:**
- Uses `schemars::schema_for!()` per type
- Writes to `schemas/{type_name}.json`
- Run once after Proposal 2 to validate the redesigned types
- Run again after all proposals to produce final schemas

**Deliverables:**
- `crates/assay-types/examples/generate-schemas.rs`
- `just schemas` recipe in justfile
- Generated `.json` files in `schemas/`

---

## Proposal 4: Config Loading (Core Only)

**Effort:** ~1.5-2 hours | **Priority:** P2 (first real I/O) | **Crate:** assay-core

**What:** Implement config loading as a free function in `assay-core::config`. TOML only. No CLI `init` command — that's presentation sugar shipped in Proposal 7.

**Design:**

```rust
// crates/assay-core/src/config.rs
use assay_types::Config;
use crate::error::{AssayError, Result};

pub fn load(path: &Path) -> Result<Config> { ... }
pub fn from_str(toml_str: &str) -> Result<Config> { ... }
```

**Key decisions (from debate):**
- **Free function, not impl on Config.** Preserves the types-are-pure-data contract. `toml` dependency goes on assay-core only, not assay-types.
- **No `assay init` bundled.** Config loading is domain logic consumed by all surfaces. `init` is CLI-specific sugar — decoupled into Proposal 7.
- **`from_str()` for testability.** Tests don't need to touch the filesystem.

**New error variant added:**
```rust
// Added to AssayError when this proposal is implemented
#[error("config error: {0}")]
Config(#[from] ConfigError),
```

**Deliverables:**
- Add `toml` to workspace deps and assay-core's Cargo.toml
- `load()` and `from_str()` in `assay-core::config`
- Add `ConfigError` sub-enum to `AssayError`
- 4-5 tests (valid config, invalid TOML, missing fields with defaults, missing file)

---

## Proposal 5: Spec and Config Validation (Core Only)

**Effort:** ~1.5-2 hours | **Priority:** P3 (first domain logic) | **Crate:** assay-core

**What:** Implement validation as free functions in `assay-core::spec` and `assay-core::config`:
- `spec::validate(spec: &Spec) -> Result<()>` — validates a single spec
- `config::validate(config: &Config) -> Result<()>` — validates all specs within a config

**Validation rules:**
- **Name:** non-empty after trimming, ≤128 characters
- **Description:** non-empty after trimming, no max length
- **Whitespace-only strings:** fail validation (trim then check empty)

**Key decisions (from debate):**
- **Free functions, not impls.** Validation is business logic → assay-core. Types stay pub and logic-free.
- **`config::validate()` connects config loading to spec validation.** Natural integration point: `config::load(path)?.validate()?` becomes `let c = config::load(path)?; config::validate(&c)?;`

**New error variant added:**
```rust
// Added to AssayError when this proposal is implemented
#[error("validation failed for '{field}': {message}")]
Validation { field: String, message: String },
```

**Deliverables:**
- `spec::validate()` in assay-core/spec
- `config::validate()` in assay-core/config
- 6-7 tests (valid spec, empty name, whitespace-only, too-long name, empty description, trimming, config with invalid spec)

---

## Proposal 6: Gate Evaluation (Sync, Core Only)

**Effort:** ~2-2.5 hours | **Priority:** P3 (first runtime behavior) | **Crate:** assay-core

**What:** Implement gate evaluation as a free function in `assay-core::gate`. Returns `GateResult` with evidence.

**Design:**

```rust
// crates/assay-core/src/gate.rs
use assay_types::{Gate, GateKind, GateResult};
use crate::error::Result;

/// Evaluate a gate and return a structured result.
///
/// Blocks the calling thread. Use `tokio::task::spawn_blocking`
/// in async contexts.
pub fn evaluate(gate: &Gate, working_dir: &Path) -> Result<GateResult> {
    match &gate.kind {
        GateKind::AlwaysPass => Ok(GateResult { passed: true, .. }),
        GateKind::Command { cmd } => {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .output()?;
            Ok(GateResult {
                passed: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into(),
                stderr: String::from_utf8_lossy(&output.stderr).into(),
                duration_ms: elapsed.as_millis() as u64,
                timestamp: now_iso8601(),
                ..
            })
        }
    }
}
```

**Key decisions (from debate):**
- **Sync only.** `std::process::Command`, not tokio. Async is a future migration. Doc comment makes this explicit.
- **Explicit `working_dir` parameter.** Mirrors Cargo.toml convention — commands run relative to the config file's location. Callers (CLI, TUI) determine this from the config path they loaded.
- **`GateResult` with stdout/stderr.** Evidence capture is what makes gates useful for debugging. A gate that says "test failed: 3 tests failed in auth module" is better than "test failed."
- **New error variant:** `Gate(String)` added to `AssayError` for gate-specific failures (e.g., command not found).

**Deliverables:**
- `gate::evaluate()` in assay-core/gate
- 4-5 tests (AlwaysPass, command success, command failure, command not found, evidence capture)

---

## Proposal 7: CLI Subcommands (`init` + `validate` + `gate run`)

**Effort:** ~2-3 hours | **Priority:** P4 (capstone) | **Crate:** assay-cli

**What:** Wire up three CLI subcommands that delegate to assay-core:
- `assay init` — writes a template `assay.toml` with comments (hardcoded template string)
- `assay validate [path]` — loads config, validates, prints results
- `assay gate run <gate-name> [--config path]` — loads config, finds gate by name, evaluates, prints GateResult

**Key decisions (from debate):**
- **Comes last.** All core logic is built and tested before CLI wires it together. CLI is pure delegation.
- **Template-based init.** Writes a handcrafted TOML string with helpful comments, not a serialized Rust struct. Preserves comments, avoids round-trip issues.
- **Unit tests of core only.** No `assert_cmd` integration tests this milestone. CLI subcommands are thin enough that passing core tests implies correct CLI behavior.
- **Template includes example gate.** `assay init` produces a config with an `AlwaysPass` gate and a `Command` gate (e.g., `echo "hello"`) so `assay gate run` works out of the box.

**Deliverables:**
- Three subcommands in assay-cli
- Human-friendly output formatting (gate results show pass/fail, stdout/stderr, duration)
- Template `assay.toml` with comments and example gates

---

## Dependency Graph

```
Proposal 1: Error Types          Proposal 3: Schema Gen
    ↓                                ↓ (re-run after 2)
Proposal 2: Domain Model ───────────┘
    ↓               ↓
Proposal 4       Proposal 6
Config Load      Gate Eval
    ↓
Proposal 5
Spec Validation
    ↓
Proposal 7: CLI Subcommands (depends on 4, 5, 6)
```

## Execution Order

| Phase | Proposals | Effort | Dependencies |
|-------|-----------|--------|-------------|
| A (parallel) | 1: Error Types, 3: Schema Gen (first run) | ~2.5h | — |
| B | 2: Domain Model Hardening | ~3-4h | Phase A complete |
| C (parallel) | 4: Config Loading, 5: Spec Validation (validate_spec only), 6: Gate Evaluation | ~5-6h | Phase B complete |
| D | 5: Config Validation (config::validate) | ~0.5h | Proposal 4 complete |
| E | 3: Schema Gen (re-run), 7: CLI Subcommands | ~2-3h | Phase C+D complete |

**Parallelization notes:**
- Phase A: Error types and schema gen are fully independent
- Phase C: Config loading, spec validation (`validate_spec`), and gate evaluation all depend only on the hardened types from Phase B, not on each other
- Phase D: `config::validate()` needs config loading (Proposal 4) to be done since it validates a loaded Config
- Phase E: CLI subcommands need all core proposals (4, 5, 6) to be done

**Total estimate: ~13-16 hours of focused work.**

---

## Rejected / Deferred Ideas

### SpecCriteria Enum — DROPPED (Round 1)
Originally proposed dual-track criteria on the Spec type. Challenger correctly identified this conflates "what to build" (Spec) with "how to verify" (Gate). Dual-track criteria belong on `GateKind` via a future `AgentEvaluated` variant.

### Private Spec Fields + Getters — DROPPED (Round 1)
assay-types is explicitly a DTO crate ("shared serializable types, no business logic"). Private fields with getters fights the crate's contract. Validation belongs in assay-core free functions.

### Integration Tests for CLI — DEFERRED (Round 1)
`assert_cmd` integration tests require a new test dependency and pattern decisions. Deferred to a future milestone. Unit tests of core functions provide sufficient coverage since CLI subcommands are thin delegation.

### chrono Dependency — AVOIDED (Round 2)
`duration_ms: u64` and ISO 8601 `String` for timestamps avoid adding chrono to the dependency tree. Portable and schemars-friendly.

---

## Design Principles Established

These emerged from the debate and should guide implementation:

1. **assay-types = pub DTOs, zero logic.** No private fields, no validation, no impls with behavior. Pure data contract consumed by all surfaces.
2. **assay-core = free functions, all behavior.** Config loading, validation, gate evaluation — all free functions that take types as input and return types as output.
3. **CLI = thin last-mile wiring.** Delegates to core, formats output. No business logic.
4. **Add error variants when consumed.** `#[non_exhaustive]` provides the escape hatch. Don't pre-define variants speculatively.
5. **Config ≠ state.** Gate is "what to check." GateResult is "what happened when we checked." Never mix them.
6. **Document sync/async boundaries.** When a function blocks, say so in the doc comment with guidance for async callers.
