# Phase 32: CLI Polish - Research

**Researched:** 2026-03-10
**Confidence:** HIGH (all code locations verified against current source)

---

## Standard Stack

No new external dependencies needed. Everything uses:
- `std::io::IsTerminal` (stdlib, already used in `worktree.rs:315`)
- `std::env::var_os` (already used in `colors_enabled()`)
- `clap` derive macros (already used throughout)

---

## Architecture Patterns

### Pattern: Color Flag Resolution
Currently `colors_enabled()` is called at each use site (11 call sites across 4 files). The function lives in `commands/mod.rs:32-34`.

```rust
// Current implementation (commands/mod.rs:32-34)
pub(crate) fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}
```

The codebase passes `color: bool` through `StreamConfig` and as a parameter to formatting functions. This pattern is correct and should be preserved.

### Pattern: Command Module Structure
Each command module (`gate.rs`, `spec.rs`, etc.) defines a clap enum, a public `handle()` function, and private handler functions. Shared helpers live in `commands/mod.rs`.

### Pattern: Help Text
Help text uses two levels:
- `#[command(about = "...")]` or doc comment `///` for short help (`-h`)
- `#[command(after_long_help = "...")]` for examples shown with `--help`

Top-level commands in `main.rs` have `after_long_help` with example blocks.
Subcommands in their respective files also have `after_long_help` with example blocks.

---

## CLI-01: NO_COLOR Handling

### Current State
- **File:** `crates/assay-cli/src/commands/mod.rs:32-34`
- **Implementation:** `std::env::var_os("NO_COLOR").is_none()` -- already correct per no-color.org
- **Call sites:** 11 across `gate.rs` (lines 368, 501, 641, 727), `spec.rs` (line 146), `context.rs` (lines 226, 439, 472), `worktree.rs` (lines 160, 191, 272)
- **TTY detection:** NOT implemented. The CONTEXT.md decision says "Auto-disable color when stdout is not a TTY."

### What Needs to Change
1. Add TTY check to `colors_enabled()`: `std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()`
2. Need `use std::io::IsTerminal;` in `commands/mod.rs`
3. The `context.rs` module has a `--plain` flag that independently gates color: `let color = !plain && colors_enabled();` (lines 226, 439, 472). This pattern is correct and should be preserved.

### Discretion Recommendation: Per-call-site vs Startup
Keep per-call-site. The function is trivial (two checks), already called at each use site, and the `context.rs` `--plain` flag pattern requires per-call-site flexibility. No performance concern.

**Confidence:** HIGH

---

## CLI-02: Help Text Duplication

### Current State
The top-level `Gate` variant in `main.rs:78-91` has `after_long_help` with examples:
```
assay gate run auth-flow
assay gate run auth-flow --verbose
assay gate run auth-flow --timeout 60
assay gate run auth-flow --json
```

The `GateCommand::Run` variant in `gate.rs:15-33` has `after_long_help` with overlapping examples:
```
assay gate run auth-flow
assay gate run --all
assay gate run auth-flow --verbose
assay gate run auth-flow --timeout 60
assay gate run auth-flow --json
assay gate run --all --json
```

**Duplication:** 4 of the 6 examples in `GateCommand::Run` are identical to the 4 in the top-level `Gate` command. The top-level help should provide a brief overview (or no examples -- just the `about` text), while the subcommand `Run` should have the detailed examples.

Similarly, the `Spec` command in `main.rs:63-73` duplicates examples from `SpecCommand::Show` in `spec.rs:10-16`.

### What Needs to Change
1. Remove `after_long_help` from top-level `Gate` in `main.rs:78-91` (or reduce to a single representative example)
2. Remove `after_long_help` from top-level `Spec` in `main.rs:63-73` (or reduce similarly)
3. The subcommand-level help (`gate.rs`, `spec.rs`) keeps full examples
4. Ensure `assay gate` with no subcommand shows help (currently `GateCommand::Run { .. } => bail!("specify a spec name or use --all")` at `gate.rs:102-104` -- but this only fires for `gate run` without args, not bare `gate`)

**Note:** Bare `assay gate` already shows help because clap auto-generates help for subcommand groups. No code change needed for that behavior.

**Confidence:** HIGH

---

## CLI-03: Enforcement Check Deduplication

### Current State
Both `handle_gate_run_all` (lines 306-437) and `handle_gate_run` (lines 440-540) in `gate.rs` share this pattern for streaming mode:

**`handle_gate_run_all` (lines 368-436):**
```rust
let color = colors_enabled();
let cfg = StreamConfig { cli_timeout, config_timeout, verbose, color };
let mut counters = StreamCounters { passed: 0, failed: 0, warned: 0, skipped: 0 };
// ... loop over entries, stream_criterion for each ...
print_gate_summary(&counters, color, &format!("Results ({spec_count} specs)"));
Ok(if counters.failed > 0 { 1 } else { 0 })
```

**`handle_gate_run` (lines 501-540):**
```rust
let color = colors_enabled();
let cfg = StreamConfig { cli_timeout, config_timeout, verbose, color };
let mut counters = StreamCounters { passed: 0, failed: 0, warned: 0, skipped: 0 };
// ... stream_criterion for each criterion ...
print_gate_summary(&counters, color, "Results");
Ok(if counters.failed > 0 { 1 } else { 0 })
```

The duplicated enforcement check is the exit code decision: `if counters.failed > 0 { 1 } else { 0 }`. This appears at:
- `gate.rs:436` in `handle_gate_run_all`
- `gate.rs:539` in `handle_gate_run`

Also duplicated:
- `StreamConfig` construction (lines 369-374 and 502-507)
- `StreamCounters` initialization (lines 375-380 and 508-513)
- The streaming loop pattern (iterate criteria, call `stream_criterion`)
- The `print_gate_summary` + exit code block

### What Can Be Extracted
A `gate_blocked()` method on `StreamCounters` that returns `bool` (any required failures). This replaces the `counters.failed > 0` check. Combined with a `tally()` method, this addresses CLI-05 at the same time.

The `StreamConfig` construction and `StreamCounters` initialization could share a helper, but the functions already call `stream_criterion` which takes `&StreamConfig` and `&mut StreamCounters`. The real duplication is the exit-code decision logic.

**Confidence:** HIGH

---

## CLI-04: Spec Show Color Branch Duplication

### Current State
**File:** `crates/assay-cli/src/commands/spec.rs:176-206`

The `print_criteria_table` function has duplicated `println!` calls differing only by column width:

```rust
if color {
    println!(
        "  {:<num_w$}  {:<name_w$}  {:<type_w$}  {cmd_display}",
        i + 1, criterion.name, type_label,
        num_w = num_width, name_w = name_width,
        type_w = type_width + ANSI_COLOR_OVERHEAD,  // <-- only difference
    );
} else {
    println!(
        "  {:<num_w$}  {:<name_w$}  {:<type_w$}  {cmd_display}",
        i + 1, criterion.name, type_label,
        num_w = num_width, name_w = name_width,
        type_w = type_width,
    );
}
```

### What Needs to Change
Compute `type_w` once before the branch:
```rust
let type_w = if color { type_width + ANSI_COLOR_OVERHEAD } else { type_width };
println!("  {:<num_w$}  {:<name_w$}  {:<type_w$}  {cmd_display}",
    i + 1, criterion.name, type_label,
    num_w = num_width, name_w = name_width, type_w = type_w);
```

Same pattern exists in `gate.rs:687-688` for the history table status column width.

**Confidence:** HIGH

---

## CLI-05: StreamCounters Improvements

### Current State
**File:** `crates/assay-cli/src/commands/gate.rs:172-177`

```rust
struct StreamCounters {
    passed: usize,
    failed: usize,
    warned: usize,
    skipped: usize,
}
```

- No doc comments on struct or fields
- No methods -- all logic is inline at call sites
- `print_gate_summary` (line 289) computes total: `counters.passed + counters.failed + counters.warned + counters.skipped`
- Exit code check: `counters.failed > 0` at lines 436, 539
- Initialized at lines 375-380 and 508-513

### What Needs Adding
1. **Doc comments** on struct and each field
2. **`tally()` method** returning total count: `self.passed + self.failed + self.warned + self.skipped`
3. **`gate_blocked()` method** returning `bool`: `self.failed > 0` (whether any required criterion failed)
4. Consider a `Default` impl or `new()` constructor to reduce initialization boilerplate

### Where Used
- `stream_criterion` (line 191) mutates counters
- `print_gate_summary` (line 289) reads counters
- `handle_gate_run_all` (lines 375-380, 436) creates and checks counters
- `handle_gate_run` (lines 508-513, 539) creates and checks counters

**Confidence:** HIGH

---

## CLI-06: StreamConfig Doc Comments

### Current State
**File:** `crates/assay-cli/src/commands/gate.rs:179-185`

```rust
/// Display configuration for streaming criterion evaluation.
struct StreamConfig {
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
    verbose: bool,
    color: bool,
}
```

The struct has a doc comment but the 4 fields have none.

### What Needs Adding
Doc comments for each field:
- `cli_timeout`: CLI-provided timeout override (seconds)
- `config_timeout`: Config file default timeout (seconds)
- `verbose`: Show evidence for all criteria, not just failures
- `color`: Whether to emit ANSI color codes

**Confidence:** HIGH

---

## CLI-07: Command Column Separator Data-Driven

### Current State
The column separator pattern `"  "` (two spaces) is used implicitly throughout table formatting. Looking for explicit "column separator" patterns:

In `gate.rs` history table (lines 644-669), `spec.rs` criteria table (lines 156-174), `checkpoint.rs` (lines 137-154), and `context.rs` (lines 480-506), the column gap is embedded in format strings as literal `"  "` (two spaces between format fields).

However, the more specific issue from the requirements ("Command column separator is data-driven, not hardcoded") likely refers to the hardcoded `"  "` between columns in `println!` format strings throughout the CLI.

Looking at the issue tracker and broader context, this is about having a consistent column gap that can be changed in one place rather than being scattered across many format strings.

### What Needs to Change
Define a constant like `const COLUMN_GAP: &str = "  ";` in `commands/mod.rs` and use it in table formatting. However, since Rust format strings don't allow runtime separators easily (they're compile-time), the practical approach is to define a column gap width constant: `const COLUMN_GAP: usize = 2;` and use it in width calculations.

**Confidence:** MEDIUM -- the exact scope of "data-driven" is ambiguous. The simplest interpretation: extract the column gap to a named constant so it's consistent and changeable.

---

## CLI-08: Magic String Extraction

### Current State
`[srs]` appears as a hardcoded string literal in 3 locations:

1. **`crates/assay-cli/src/commands/init.rs:71`**
   ```rust
   "  {:<width$}  [srs] {total} criteria ({executable} executable)",
   ```

2. **`crates/assay-cli/src/commands/spec.rs:92`**
   ```rust
   println!("Spec: {} [srs]", gates.name);
   ```

3. **`crates/assay-cli/src/commands/spec.rs:259`**
   ```rust
   let indicator = "[srs]";
   ```

### What Needs to Change
Per CONTEXT.md decision: constant lives in `assay-types` (protocol-level marker shared across crates).

Options for the constant:
- `pub const SRS_INDICATOR: &str = "[srs]";` -- includes brackets (ready for display)
- `pub const SRS_TAG: &str = "srs";` -- bare value, brackets added at display sites

**Recommendation:** Include brackets in the constant value (`"[srs]"`) since all 3 usage sites display it with brackets. Name: `SRS_INDICATOR` or `DIRECTORY_SPEC_INDICATOR`.

**Location in assay-types:** Add to `crates/assay-types/src/lib.rs` as a top-level constant (it's a protocol-level marker for the directory-based spec format).

Note: `assay-types` has `#![deny(missing_docs)]` so the constant needs a doc comment.

**Confidence:** HIGH

---

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| TTY detection | `std::io::IsTerminal` (stdlib trait, already used in `worktree.rs`) |
| Color library | Keep raw ANSI codes -- existing pattern, no crate needed |
| Help text | clap derive macros -- existing pattern |

---

## Common Pitfalls

1. **NO_COLOR with `--plain` flag:** `context.rs` uses `!plain && colors_enabled()`. After adding TTY detection to `colors_enabled()`, `--plain` remains a separate override. These are orthogonal -- `--plain` also disables Unicode symbols, not just color.

2. **ANSI_COLOR_OVERHEAD correctness:** The constant is 9 (5 for `\x1b[32m` + 4 for `\x1b[0m`). Some ANSI codes are `\x1b[XXm` (5 bytes) but `\x1b[0m` is only 4. The constant is correct for single-color pairs but would be wrong for codes like `\x1b[1;31m` (bold+red = 7 bytes). Current usage only uses single-attribute codes so this is fine.

3. **Enforcement check semantics:** `counters.failed > 0` counts ALL failures (required + advisory-that-errored). But advisory failures go to `counters.warned`, not `counters.failed`. So `failed > 0` already means "required criterion failed." The `gate_blocked()` method name is accurate.

4. **assay-types `deny(missing_docs)`:** Any new public constant in `assay-types` must have a doc comment or the build will fail.

5. **Help text: bare `assay gate` behavior:** Clap automatically shows help for subcommand groups when no subcommand is given. Don't add custom "no subcommand" error handling for `gate` -- it already works correctly.

---

## Code Examples

### TTY-aware `colors_enabled()`
```rust
use std::io::IsTerminal;

pub(crate) fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}
```

### StreamCounters with methods
```rust
/// Accumulated pass/fail/warn/skip counts during streaming gate evaluation.
struct StreamCounters {
    /// Number of criteria that passed.
    passed: usize,
    /// Number of required criteria that failed.
    failed: usize,
    /// Number of advisory criteria that failed (non-blocking).
    warned: usize,
    /// Number of criteria skipped (no executable command).
    skipped: usize,
}

impl StreamCounters {
    /// Total number of criteria processed.
    fn tally(&self) -> usize {
        self.passed + self.failed + self.warned + self.skipped
    }

    /// Whether any required criterion failed (gate is blocked).
    fn gate_blocked(&self) -> bool {
        self.failed > 0
    }
}
```

### Color branch dedup pattern
```rust
// Before (two branches)
if color {
    println!("  {:<tw$}  ...", label, tw = width + ANSI_COLOR_OVERHEAD);
} else {
    println!("  {:<tw$}  ...", label, tw = width);
}

// After (single branch)
let tw = if color { width + ANSI_COLOR_OVERHEAD } else { width };
println!("  {:<tw$}  ...", label, tw = tw);
```

### Magic string constant (assay-types)
```rust
/// Display indicator for directory-based (structured) specs.
///
/// Shown in CLI output to distinguish directory specs from legacy flat-file specs.
pub const DIRECTORY_SPEC_INDICATOR: &str = "[srs]";
```

---

## File Map

| Requirement | File | Lines | Change Type |
|-------------|------|-------|-------------|
| CLI-01 | `crates/assay-cli/src/commands/mod.rs` | 32-34 | Modify `colors_enabled()` to add TTY check |
| CLI-02 | `crates/assay-cli/src/main.rs` | 63-73, 78-91 | Remove/reduce `after_long_help` from `Spec` and `Gate` |
| CLI-03 | `crates/assay-cli/src/commands/gate.rs` | 436, 539 | Replace inline check with `gate_blocked()` |
| CLI-04 | `crates/assay-cli/src/commands/spec.rs` | 185-206 | Collapse color branch to single `println!` |
| CLI-04 | `crates/assay-cli/src/commands/gate.rs` | 687-688 | Collapse status width branch |
| CLI-05 | `crates/assay-cli/src/commands/gate.rs` | 172-177 | Add docs, `tally()`, `gate_blocked()` to `StreamCounters` |
| CLI-06 | `crates/assay-cli/src/commands/gate.rs` | 180-185 | Add field doc comments to `StreamConfig` |
| CLI-07 | `crates/assay-cli/src/commands/mod.rs` | N/A | Add `COLUMN_GAP` constant |
| CLI-08 | `crates/assay-types/src/lib.rs` | N/A | Add `DIRECTORY_SPEC_INDICATOR` constant |
| CLI-08 | `crates/assay-cli/src/commands/init.rs` | 71 | Use constant |
| CLI-08 | `crates/assay-cli/src/commands/spec.rs` | 92, 259 | Use constant |

---

*Phase: 32-cli-polish*
*Research completed: 2026-03-10*
