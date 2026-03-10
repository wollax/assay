# Phase 32: CLI Polish — Verification

**Verified:** 2026-03-10
**Status:** passed

---

## Must-Have 1: NO_COLOR handling uses `var_os().is_none()` and TTY check

**Result:** PASS

`colors_enabled()` in `crates/assay-cli/src/commands/mod.rs:37-39`:

```rust
pub(crate) fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}
```

- Uses `var_os("NO_COLOR").is_none()` — correct per no-color.org (any value, including empty, disables color)
- Uses `std::io::stdout().is_terminal()` — auto-disables color when stdout is piped
- `use std::io::IsTerminal;` present at `mod.rs:10`

---

## Must-Have 2: Gate command help text appears once (no duplication between top-level and subcommand)

**Result:** PASS

`crates/assay-cli/src/main.rs:68-71` — the top-level `Gate` variant has no `after_long_help`:

```rust
/// Manage quality gates
Gate {
    #[command(subcommand)]
    command: commands::gate::GateCommand,
}
```

No `#[command(after_long_help = "...")]` on the `Gate` variant. The `Spec` variant (lines 63-66) similarly has no `after_long_help`.

The detailed examples live exclusively in `crates/assay-cli/src/commands/gate.rs:15-33` on `GateCommand::Run`. No duplication.

---

## Must-Have 3: Enforcement check logic exists in one place (shared between `handle_gate_run_all` and `handle_gate_run`)

**Result:** PASS

The exit-code decision is extracted to `gate_exit_code()` at `gate.rs:327-329`:

```rust
fn gate_exit_code(counters: &StreamCounters) -> i32 {
    if counters.gate_blocked() { 1 } else { 0 }
}
```

Both handlers call this shared function:
- `handle_gate_run_all` at `gate.rs:469`: `Ok(gate_exit_code(&counters))`
- `handle_gate_run` at `gate.rs:562`: `Ok(gate_exit_code(&counters))`

The enforcement check (`counters.failed > 0`) is not duplicated inline; it is encapsulated in `gate_blocked()` (see Must-Have 4) and called through `gate_exit_code()`.

---

## Must-Have 4: `StreamCounters` has doc comments, a `tally()` method, and a `gate_blocked()` method

**Result:** PASS

`crates/assay-cli/src/commands/gate.rs:171-194`:

- Struct has doc comment: `/// Tracks pass/fail/warn/skip counts during streaming gate execution.`
- All four fields have doc comments (`passed`, `failed`, `warned`, `skipped`)
- `tally()` method (lines 185-188): returns `self.passed + self.failed + self.warned + self.skipped`
- `gate_blocked()` method (lines 191-193): returns `self.failed > 0`
- `#[derive(Default)]` on the struct (line 172) eliminates initialization boilerplate

---

## Must-Have 5: The `[srs]` magic string is extracted to a named constant

**Result:** PASS

`crates/assay-types/src/lib.rs:38-41`:

```rust
/// Marker prefix for directory-based specs in CLI output (e.g., `[srs] auth-flow`).
///
/// Directory specs store criteria across multiple files rather than in a single TOML.
pub const DIRECTORY_SPEC_INDICATOR: &str = "[srs]";
```

All three former hardcoded sites now use this constant:
- `crates/assay-cli/src/commands/init.rs:74`: `assay_types::DIRECTORY_SPEC_INDICATOR`
- `crates/assay-cli/src/commands/spec.rs:97`: `assay_types::DIRECTORY_SPEC_INDICATOR`
- `crates/assay-cli/src/commands/spec.rs:262`: `let indicator = assay_types::DIRECTORY_SPEC_INDICATOR;`

The constant carries a doc comment satisfying `assay-types`'s `#![deny(missing_docs)]`.

---

## Additional Deliverables Verified

**`COLUMN_GAP` constant** — `commands/mod.rs:20`: `pub(crate) const COLUMN_GAP: &str = "  ";` with doc comment. Used in `init.rs:61`, `spec.rs` (multiple sites).

**`StreamConfig` field doc comments** — All four fields documented at `gate.rs:198-206`.

**Color branch dedup in `spec.rs`** — `print_criteria_table` computes `tw` once before the `println!` rather than duplicating the call in two branches (`spec.rs:193-196`).

**`just ready` status:** Confirmed passing (per task instructions — no build failures to investigate).

---

## Overall Assessment

All five must-haves are implemented correctly in the actual source files. Evidence is direct (code read from disk), not inferred from summary claims. Phase 32 is complete.
