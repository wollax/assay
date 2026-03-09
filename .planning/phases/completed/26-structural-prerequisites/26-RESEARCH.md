# Phase 26: Structural Prerequisites - Research

**Researched:** 2026-03-09
**Confidence:** HIGH (primary source is the existing codebase)

## Summary

Phase 26 decomposes into three independent workstreams:

1. **CLI monolith extraction** — Split the 2563-line `main.rs` into `commands/` modules (one per subcommand group)
2. **Error variant + ergonomics** — Add `Json` variant to `AssayError`, add constructor helpers
3. **TUI-core wiring verification** — Confirm `assay-tui` can import `assay-core` types (dependency already declared)

The codebase investigation found clear, natural extraction boundaries in `main.rs`, two concrete locations where `serde_json` errors are currently shoehorned into `AssayError::Io`, and a TUI that already declares `assay-core` as a dependency but never imports from it.

## Standard Stack

| Concern | Use | Version | Confidence |
|---------|-----|---------|------------|
| CLI framework | `clap` (derive) | 4.x | HIGH — already in workspace |
| Error derivation | `thiserror` | 2.x | HIGH — already in workspace |
| TUI error handling | `color-eyre` | 0.6 | HIGH — keep for TUI, no change this phase |
| Serialization | `serde_json` | 1.x | HIGH — already in workspace |

**Zero new workspace dependencies required.** All work uses existing crates.

## Architecture Patterns

### CLI Module Extraction

**Use flat files, not nested directories.** Each subcommand group is a single handler module — none are complex enough to warrant `commands/gate/mod.rs` directory structure. The largest group (gate) is ~500 lines; the smallest (mcp) is ~15 lines.

Recommended module layout:

```
crates/assay-cli/src/
├── main.rs              # Cli struct, Command enum, dispatch, main()
├── commands/
│   ├── mod.rs           # pub mod declarations + shared helpers (colors, formatting)
│   ├── checkpoint.rs    # CheckpointCommand enum + handle_checkpoint_*
│   ├── context.rs       # ContextCommand + GuardCommand enums + handle_context_*, handle_guard_*
│   ├── gate.rs          # GateCommand enum + handle_gate_*, stream_criterion, StreamConfig, etc.
│   ├── init.rs          # handle_init (inline, ~10 lines of dispatch)
│   ├── mcp.rs           # McpCommand enum + handle_mcp_serve
│   └── spec.rs          # SpecCommand enum + handle_spec_*
```

**Key decisions:**

1. **`init` gets its own module** — even though it's small (~10 lines of dispatch), it keeps `main.rs` purely structural and every `Command` variant has a corresponding module file.

2. **Subcommand enums move to their respective modules.** `GateCommand` goes to `gate.rs`, `SpecCommand` to `spec.rs`, etc. The top-level `Command` enum stays in `main.rs` with subcommand references like `Gate { #[command(subcommand)] command: gate::GateCommand }`.

3. **Shared CLI helpers go in `commands/mod.rs`.** These are the formatting/color utilities used across multiple modules:
   - `colors_enabled()`, `colorize()`, `format_pass()`, `format_fail()`, `format_warn()`, `format_count()`
   - `format_size()`, `format_number()`, `format_duration_ms()`, `format_relative_timestamp()`, `format_relative_time()`
   - `ANSI_COLOR_OVERHEAD`, `ASSAY_DIR_NAME`, `assay_dir()`, `project_root()`
   - `gate_kind_label()`, `criterion_label()`, `format_criteria_type()`

4. **`main.rs` retains only:**
   - `Cli` struct with `#[derive(Parser)]`
   - Top-level `Command` enum with `#[derive(Subcommand)]`
   - `run()` async fn with the dispatch match
   - `main()` fn

### Approximate line counts per module (post-extraction)

| Module | Lines | Functions |
|--------|-------|-----------|
| `main.rs` | ~80 | `run()`, `main()` |
| `commands/mod.rs` | ~150 | shared helpers, constants |
| `commands/gate.rs` | ~500 | `GateCommand`, `StreamConfig`, `StreamCounters`, `stream_criterion`, `handle_gate_run`, `handle_gate_run_all`, `handle_gate_history`, `handle_gate_history_detail`, `print_gate_summary`, `save_run_record`, `streaming_summary`, `print_evidence` |
| `commands/context.rs` | ~400 | `ContextCommand`, `GuardCommand`, `handle_context_diagnose`, `handle_context_list`, `handle_context_prune`, `handle_guard_*`, `log_level_rank`, `levels_at_or_above` |
| `commands/spec.rs` | ~200 | `SpecCommand`, `handle_spec_show`, `handle_spec_list`, `handle_spec_new`, `print_spec_table`, `print_criteria_table` |
| `commands/checkpoint.rs` | ~130 | `CheckpointCommand`, `handle_checkpoint_save`, `handle_checkpoint_show`, `handle_checkpoint_list` |
| `commands/mcp.rs` | ~30 | `McpCommand`, `init_mcp_tracing` |
| `commands/init.rs` | ~20 | (dispatch only, or inline `show_status` here) |

### Error Variant Pattern

**Add a `Json` variant as a flat variant on `AssayError`.** Do NOT introduce domain sub-enums (GuardError, SessionError) this phase — the flat enum is working well and the scope should stay tight.

Current problem (found in `crates/assay-core/src/history/mod.rs`):

```rust
// Line 124: serde_json serialize error shoehorned into Io
serde_json::to_string_pretty(record).map_err(|e| AssayError::Io {
    operation: "serializing gate run record".into(),
    path: results_dir.clone(),
    source: std::io::Error::other(e),
})?;

// Line 191: serde_json deserialize error shoehorned into Io
serde_json::from_str(&content).map_err(|e| AssayError::Io {
    operation: "deserializing gate run record".into(),
    path,
    source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
})
```

Also found in `crates/assay-core/src/checkpoint/persistence.rs` line 227 — `serde_json` deserialization error mapped to `CheckpointRead` with stringified message.

New variant design:

```rust
/// JSON serialization or deserialization failed.
#[error("{operation} at `{path}`: {source}")]
Json {
    /// What was being attempted (e.g., "serializing gate run record").
    operation: String,
    /// The file path involved.
    path: PathBuf,
    /// The underlying serde_json error.
    source: serde_json::Error,
}
```

**Use inherent methods (not free functions) for constructors.** This is idiomatic Rust — the constructors live on the type they construct.

**Use `impl Into<String>` for `operation` parameter.** This allows both `&str` and `String` at call sites without `.to_string()` noise, which is the existing pattern in the codebase (fields are `String`, constructed from `"literal".into()`).

Constructor design:

```rust
impl AssayError {
    /// Create an I/O error with context.
    pub fn io(operation: impl Into<String>, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            operation: operation.into(),
            path: path.into(),
            source,
        }
    }

    /// Create a JSON serialization/deserialization error with context.
    pub fn json(operation: impl Into<String>, path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::Json {
            operation: operation.into(),
            path: path.into(),
            source,
        }
    }
}
```

### TUI Wiring

`assay-tui/Cargo.toml` already declares `assay-core.workspace = true` (line 10). The TUI `main.rs` (42 lines) currently imports nothing from `assay-core`. Verification is a single `use assay_core::AssayError;` or similar import + `cargo check -p assay-tui`.

**Keep `color-eyre` for the TUI this phase.** The TUI is a 42-line skeleton — swapping error handling is premature. Just verify the dependency wires up.

**Do NOT start importing core types for real TUI views this phase.** That's feature work for later phases.

## Don't Hand-Roll

| Problem | Use Instead | Rationale |
|---------|-------------|-----------|
| Error display formatting | `thiserror` `#[error(...)]` | Already used; don't build custom Display impl |
| Error source chains | `thiserror` `#[source]` or `source:` named field | Already used; automatic Error::source() impl |
| CLI argument parsing | `clap` derive macros | Already used; subcommand enums stay clap-derived |
| Module re-exports | Standard `pub use` in `commands/mod.rs` | Don't build a macro or registration system |

## Common Pitfalls

### CLI Extraction

1. **Visibility mistakes** — Functions extracted to modules need `pub(crate)` or `pub`. Currently they're private (`fn`). Each handler needs exactly `pub(crate)` since they're only called from `main.rs` dispatch.

2. **Circular imports** — Shared helpers (colors, formatting) must go in `commands/mod.rs`, not in individual command modules. If `gate.rs` and `spec.rs` both need `format_pass()`, it must live in the parent module.

3. **`use` path churn** — After extraction, `main.rs` uses change from bare function calls to qualified `commands::gate::handle_gate_run(...)` or `use commands::gate::*`. Prefer explicit qualified paths or targeted `use` imports over glob imports.

4. **Cfg-gated functions** — `handle_guard_start()` and `handle_guard_stop()` have `#[cfg(unix)]` / `#[cfg(not(unix))]` variants. These must stay paired when moved to `context.rs`.

5. **Async boundary** — `handle_guard_start()` creates its own `tokio::runtime::Runtime`. The `run()` function is `async`. MCP serve is also async. Ensure the module extraction preserves these correctly.

### Error Changes

1. **`serde_json::Error` is not `Send + Sync`-safe by default** — FALSE, it IS `Send + Sync`. No issue here. (Confidence: HIGH — verified from serde_json docs, it implements `std::error::Error + Send + Sync`.)

2. **Adding a variant to a `#[non_exhaustive]` enum is non-breaking** — correct. Downstream code already has wildcard arms. But `assay-core` internal match statements (if any) need updating.

3. **`serde_json` is already a workspace dependency** — no new dependency needed. Just need to add it to `assay-core/Cargo.toml` if not already there.

4. **Existing tests construct `AssayError::Io` with literal struct syntax** — after adding constructors, do NOT change existing tests to use constructors. The tests validate the struct variants directly. New code should use constructors; existing test code stays as-is.

### TUI Wiring

1. **`color-eyre` and `thiserror` coexist fine** — `color-eyre::Result` wraps `eyre::Report` which accepts any `std::error::Error`. `AssayError` implements `Error` via `thiserror`. No conflict.

## Code Examples

### Dispatch pattern in main.rs after extraction

```rust
mod commands;

// In run():
match cli.command {
    Some(Command::Init { name }) => commands::init::handle(name),
    Some(Command::Mcp { command }) => commands::mcp::handle(command).await,
    Some(Command::Spec { command }) => commands::spec::handle(command),
    Some(Command::Gate { command }) => commands::gate::handle(command),
    Some(Command::Context { command }) => commands::context::handle(command),
    Some(Command::Checkpoint { command }) => commands::checkpoint::handle(command),
    None => { /* status display */ }
}
```

### Error constructor usage at call sites

```rust
// Before (current):
serde_json::to_string_pretty(record).map_err(|e| AssayError::Io {
    operation: "serializing gate run record".into(),
    path: results_dir.clone(),
    source: std::io::Error::other(e),
})?;

// After:
serde_json::to_string_pretty(record)
    .map_err(|e| AssayError::json("serializing gate run record", &results_dir, e))?;
```

```rust
// Before (current):
std::fs::read_to_string(&path).map_err(|source| AssayError::Io {
    operation: "reading gate run record".into(),
    path: path.clone(),
    source,
})?;

// After:
std::fs::read_to_string(&path)
    .map_err(|e| AssayError::io("reading gate run record", &path, e))?;
```

### Module file template (e.g., commands/spec.rs)

```rust
use anyhow::{Context, bail};
use assay_core::spec::SpecEntry;
use clap::Subcommand;

use super::{assay_dir, colors_enabled, project_root, /* ... */};

#[derive(Subcommand)]
pub(crate) enum SpecCommand {
    Show { name: String, #[arg(long)] json: bool },
    List,
    New { name: String },
}

pub(crate) fn handle(command: SpecCommand) -> anyhow::Result<i32> {
    match command {
        SpecCommand::Show { name, json } => handle_spec_show(&name, json),
        SpecCommand::List => handle_spec_list(),
        SpecCommand::New { name } => handle_spec_new(&name),
    }
}

fn handle_spec_show(name: &str, json: bool) -> anyhow::Result<i32> {
    // ... existing code moved here
}
```

## Sources

| Source | What Was Learned | Confidence |
|--------|-----------------|------------|
| `crates/assay-cli/src/main.rs` (2563 lines) | Complete monolith structure, all subcommand groups, all handler functions, shared helpers | HIGH |
| `crates/assay-core/src/error.rs` (337 lines) | Full `AssayError` enum with 20 variants, `#[non_exhaustive]`, `thiserror` patterns, existing tests | HIGH |
| `crates/assay-core/src/history/mod.rs` L124, L191 | Two concrete sites where `serde_json` errors are shoehorned into `AssayError::Io` | HIGH |
| `crates/assay-core/src/checkpoint/persistence.rs` L227 | Third site where `serde_json` error is stringified into `CheckpointRead` | HIGH |
| `crates/assay-tui/Cargo.toml` | `assay-core.workspace = true` already declared | HIGH |
| `crates/assay-tui/src/main.rs` (42 lines) | Skeleton TUI, no core imports, uses `color-eyre` | HIGH |
| `crates/assay-core/src/lib.rs` | Module structure of core crate | HIGH |
| `Cargo.toml` (workspace root) | All workspace dependencies, confirms `thiserror = "2"`, `serde_json = "1"` | HIGH |
| `crates/assay-cli/Cargo.toml` | CLI dependencies — already has `assay-core`, `assay-types`, `serde_json`, `anyhow` | HIGH |

## Locations Requiring Changes

### For serde_json error variant (CORE-01 + CORE-05)

| File | Lines | Change |
|------|-------|--------|
| `crates/assay-core/src/error.rs` | New | Add `Json` variant + `io()` and `json()` constructors |
| `crates/assay-core/src/history/mod.rs` | 124, 191 | Replace `AssayError::Io` with `AssayError::json()` |
| `crates/assay-core/src/checkpoint/persistence.rs` | 227 | Consider replacing stringified error with `AssayError::json()` |

### For CLI extraction

| File | Change |
|------|--------|
| `crates/assay-cli/src/main.rs` | Reduce to ~80 lines: Cli, Command, dispatch, main |
| `crates/assay-cli/src/commands/mod.rs` | New: shared helpers + pub mod declarations |
| `crates/assay-cli/src/commands/spec.rs` | New: SpecCommand + handlers |
| `crates/assay-cli/src/commands/gate.rs` | New: GateCommand + handlers |
| `crates/assay-cli/src/commands/context.rs` | New: ContextCommand + GuardCommand + handlers |
| `crates/assay-cli/src/commands/checkpoint.rs` | New: CheckpointCommand + handlers |
| `crates/assay-cli/src/commands/mcp.rs` | New: McpCommand + handler |
| `crates/assay-cli/src/commands/init.rs` | New: init handler (+ show_status) |

### For TUI wiring

| File | Change |
|------|--------|
| `crates/assay-tui/src/main.rs` | Add `use assay_core::AssayError;` (or similar) as smoke test |

---

*Phase: 26-structural-prerequisites*
*Research completed: 2026-03-09*
