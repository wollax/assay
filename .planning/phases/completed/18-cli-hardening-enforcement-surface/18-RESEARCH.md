# Phase 18 Research: CLI Hardening & Enforcement Surface

## Standard Stack

| Concern | Solution | Confidence |
|---------|----------|------------|
| Binary error handling | `anyhow` (new workspace dep) | HIGH |
| Argument parsing | `clap` 4 with derive (already present) | HIGH |
| Async runtime | `tokio` (already present) | HIGH |
| ANSI coloring | Hand-rolled escape sequences (already used) | HIGH |
| Exit codes | `std::process::exit()` in `main()` only | HIGH |

**anyhow version:** Use `anyhow = "1"` as a workspace dependency. Add it to root `Cargo.toml` `[workspace.dependencies]` and to `assay-cli/Cargo.toml` `[dependencies]`. Do NOT add to library crates — they use `thiserror` (already present).

No new crates beyond `anyhow` are needed.

## Architecture Patterns

### 1. Catch-at-Top with `#[tokio::main]`

`#[tokio::main]` expands `async fn main()` into a sync `fn main()` that builds the runtime and blocks on the future. It supports return types — `async fn main() -> T` works for any `T` that sync `fn main()` can return. However, `fn main() -> anyhow::Result<i32>` does not work as a program entry point because Rust's `Termination` trait is not implemented for `Result<i32, anyhow::Error>`.

**The correct pattern:** Keep `main()` return type as `()` and call `process::exit()` explicitly.

```rust
#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn run() -> anyhow::Result<i32> {
    let cli = Cli::try_parse()?;
    // ... dispatch to handlers ...
    Ok(0)
}
```

Key points:
- `main()` has exactly ONE `process::exit()` call — the catch-at-top.
- `run()` returns `anyhow::Result<i32>` where the `i32` is the exit code.
- All 41 existing `process::exit(1)` calls in handler functions become `?` or `bail!()` or `return Ok(1)`.
- `eprintln!("Error: {e:#}")` uses anyhow's alternate display which shows the full cause chain.

**Confidence:** HIGH — this is the documented anyhow pattern (see anyhow docs, "Custom Error Handling" example with `try_main()`). The `#[tokio::main]` expansion is transparent for `()` return.

### 2. Clap `try_parse()` for Exit Code Control

`Cli::parse()` calls `process::exit()` internally on `--help`, `--version`, and parse errors. To keep `main()` as the single exit point, use `Cli::try_parse()` instead.

```rust
async fn run() -> anyhow::Result<i32> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // e.exit() prints to stdout (--help/--version) or stderr (errors)
            // and calls process::exit with 0 or 2 respectively.
            // This is acceptable — clap errors are a special case.
            e.exit();
        }
    };
    // ...
}
```

Alternatively, let clap errors propagate through anyhow:
```rust
let cli = Cli::try_parse()?;
```
This converts clap errors into anyhow errors, but the output loses clap's colored formatting. The `e.exit()` approach preserves clap's native behavior and is conventional.

**Recommendation:** Use `Cli::try_parse()` with `.unwrap_or_else(|e| e.exit())`. This preserves clap's native error formatting (colored, exit code 2 for parse errors, exit code 0 for --help/--version) while keeping `run()` clean. The `e.exit()` call is acceptable because clap parse failures are pre-business-logic.

**Confidence:** HIGH — `try_parse()` is the documented non-panicking variant. Exit codes: 0 for `--help`/`--version`, 2 for parse errors.

### 3. Exit Code Convention

| Code | Meaning | Source |
|------|---------|--------|
| 0 | Success / all required pass | `Ok(0)` |
| 1 | Gate failure / runtime error | `Ok(1)` or `Err(e)` |
| 2 | CLI parse error | clap's `e.exit()` |

This matches Unix convention (0 = success, 1 = application error, 2 = usage error). The `Err(e)` path always maps to exit 1 in the `main()` match.

### 4. Handler Return Type Migration

Current handlers are `fn handle_gate_run(...) -> ()` with scattered `process::exit(1)`. After migration:

```rust
fn handle_gate_run(name: &str, ...) -> anyhow::Result<i32> {
    let (root, config, working_dir, config_timeout) = load_gate_context()?;
    let specs_dir = assay_dir(&root).join(&config.specs_dir);
    // ...
    if has_required_failure { Ok(1) } else { Ok(0) }
}
```

- `load_gate_context()` itself returns `anyhow::Result<(...)>` instead of calling `process::exit`.
- File I/O, config loading, spec parsing all use `?`.
- Gate failure (required criterion failed) returns `Ok(1)` — it is a business logic result, not an error.

### 5. Constant Extraction for `.assay` Path

Rust `const` cannot contain heap-allocated or `Path` types. The idiomatic approach:

```rust
/// Directory name for Assay project metadata.
const ASSAY_DIR_NAME: &str = ".assay";

fn assay_dir(root: &Path) -> PathBuf {
    root.join(ASSAY_DIR_NAME)
}
```

Replace all 12 instances of `root.join(".assay")` with `assay_dir(&root)`. The helper returns `PathBuf` which chains naturally: `assay_dir(&root).join(&config.specs_dir)`.

**Alternative considered:** `static ASSAY_DIR: &str = ".assay"` — functionally identical for `&str`. Using `const` is preferred for string literals (zero overhead, no lazy init).

**Confidence:** HIGH.

## Don't Hand-Roll

| Problem | Use Instead | Why |
|---------|-------------|-----|
| Error type for binary crate | `anyhow::Error` | Cause chains, context, downcasting |
| Colored terminal output library | Keep existing ANSI escapes | Already working, no new dep needed |
| Exit code management | Single `process::exit` in `main()` | Unix convention, testable |
| Path constant abstraction | `const &str` + helper function | Simple, zero-cost |

Do NOT introduce `owo-colors`, `colored`, `termcolor`, or similar — the codebase already uses raw ANSI sequences consistently. Adding a coloring crate for two new color codes (yellow WARN) is not justified.

## Common Pitfalls

### P1: `#[tokio::main]` + `-> Result` Confusion

`async fn main() -> anyhow::Result<()>` works but prints the Debug representation of errors (ugly). `async fn main() -> anyhow::Result<i32>` does NOT compile because `Result<i32, anyhow::Error>` does not implement `Termination`. The catch-at-top pattern avoids both problems.

**Verification:** Ensure `main()` returns `()`, not `Result`.

### P2: Clap Exit Leaks

`Cli::parse()` calls `process::exit()` on `--help` and parse errors, bypassing the catch-at-top. Use `Cli::try_parse()` to prevent this.

**Verification:** Grep for `Cli::parse()` — should be zero occurrences after migration. Only `Cli::try_parse()` should remain.

### P3: Forgetting Advisory Label on Passing Criteria

Per CONTEXT.md decisions: advisory criteria are ALWAYS labeled `[advisory]`, even when passing. It is easy to only label failures. The label is on the criterion itself, not the result.

**Verification:** Test that a passing advisory criterion shows `[advisory]` prefix.

### P4: `process::exit()` Scattered in Handlers

After migration, any remaining `process::exit()` outside `main()` is a bug. It bypasses error formatting and the single-exit-point contract.

**Verification:** `grep -r "process::exit" src/main.rs` should return exactly 1 hit (in `main()`).

### P5: Summary Line Arithmetic

The summary format is: `passed, failed, warned, skipped (of N total)`. The total must equal `passed + failed + warned + skipped`. "failed" counts only required failures. "warned" counts only advisory failures. It is easy to double-count an advisory failure in both "failed" and "warned".

**Verification:** Add a test that `passed + failed + warned + skipped == total` always holds.

### P6: `--all` Mode Exit Code

In `--all` mode, the exit code must be 0 if all specs' required criteria pass, even if some specs have advisory failures. Currently the code tracks `has_required_failure` per spec — ensure it is not reset between specs.

**Verification:** Test `gate run --all` with one spec having advisory-only failures exits 0.

## Code Examples

### Example 1: Complete `main()` + `run()` Structure

```rust
use anyhow::{bail, Context, Result};
use clap::Parser;

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}

async fn run() -> Result<i32> {
    let cli = Cli::try_parse().unwrap_or_else(|e| e.exit());

    match cli.command {
        Some(Command::Init { name }) => handle_init(name),
        Some(Command::Mcp { command }) => handle_mcp(command).await,
        Some(Command::Spec { command }) => handle_spec(command),
        Some(Command::Gate { command }) => handle_gate(command),
        None => handle_bare_invocation(),
    }
}
```

### Example 2: Handler Returning `Result<i32>`

```rust
fn handle_init(name: Option<String>) -> Result<i32> {
    let root = project_root();
    let options = assay_core::init::InitOptions { name };
    let result = assay_core::init::init(&root, &options)
        .context("failed to initialize project")?;
    println!("  Created assay project `{}`", result.project_name);
    for path in &result.created_files {
        let display = path.strip_prefix(&root).unwrap_or(path);
        println!("    created {}", display.display());
    }
    Ok(0)
}
```

### Example 3: Advisory-Aware Streaming Output

```rust
struct StreamCounters {
    passed: usize,
    failed: usize,   // required failures only
    warned: usize,   // advisory failures only
    skipped: usize,
}

fn stream_criterion(
    criterion: &Criterion,
    working_dir: &Path,
    cfg: &StreamConfig,
    counters: &mut StreamCounters,
    enforcement: Enforcement,
) {
    // ... evaluate ...
    match (result.passed, enforcement) {
        (true, _) => {
            counters.passed += 1;
            let prefix = if enforcement == Enforcement::Advisory {
                "[advisory] "
            } else {
                ""
            };
            eprintln!("  {prefix}{} ... {}", criterion.name, format_pass(cfg.color));
        }
        (false, Enforcement::Required) => {
            counters.failed += 1;
            eprintln!("  {} ... {}", criterion.name, format_fail(cfg.color));
        }
        (false, Enforcement::Advisory) => {
            counters.warned += 1;
            eprintln!("  [advisory] {} ... {}", criterion.name, format_warn(cfg.color));
        }
    }
}

fn format_warn(color: bool) -> &'static str {
    if color { "\x1b[33mWARN\x1b[0m" } else { "WARN" }
}
```

### Example 4: Summary Line with Warn

```rust
fn print_gate_summary(counters: &StreamCounters, color: bool, label: &str) {
    let total = counters.passed + counters.failed + counters.warned + counters.skipped;
    let passed_str = format_count(counters.passed, "\x1b[32m", color);
    let failed_str = format_count(counters.failed, "\x1b[31m", color);
    let warned_str = format_count(counters.warned, "\x1b[33m", color);
    let skipped_str = format_count(counters.skipped, "\x1b[90m", color);

    println!();
    println!(
        "{label}: {passed_str} passed, {failed_str} failed, \
         {warned_str} warned, {skipped_str} skipped (of {total} total)"
    );
}
```

### Example 5: Constant Extraction

```rust
/// Directory name for Assay project metadata.
const ASSAY_DIR_NAME: &str = ".assay";

/// Returns the path to the `.assay` metadata directory within the given root.
fn assay_dir(root: &Path) -> std::path::PathBuf {
    root.join(ASSAY_DIR_NAME)
}
```

### Example 6: Bare Invocation Handler

```rust
fn handle_bare_invocation() -> Result<i32> {
    let root = project_root();
    if assay_dir(&root).is_dir() {
        show_status(&root)?;
        Ok(0)
    } else {
        eprintln!("Not an Assay project. Run `assay init` to get started.");
        Cli::command().print_help()?;
        println!();
        Ok(1)
    }
}
```

## Open Questions

None. All decisions are locked in CONTEXT.md and the research confirms they are implementable with standard patterns.

## Codebase-Specific Notes

1. **41 `process::exit(1)` calls** in `main.rs` — each becomes either `?` (for errors) or `return Ok(1)` (for business logic failures like missing spec names). The MCP `serve` handler is async and already uses `if let Err(e)` — it converts to `assay_mcp::serve().await?; Ok(0)`.

2. **12 `.join(".assay")` instances** — 6 compute `specs_dir` (`assay_dir(&root).join(&config.specs_dir)`), 4 compute `assay_dir` directly, 2 are inline checks. All become `assay_dir(&root)` calls.

3. **`StreamCounters`** currently has `passed`, `failed`, `skipped`. Adding `warned: usize` is additive. The `failed` field semantics change from "all failures" to "required failures only" — the `stream_criterion` function must be updated to route advisory failures to `warned` instead.

4. **`resolve_enforcement()`** already exists in `assay_core::gate` and is called in two places in `main.rs` (lines 890, 1013). It takes `Option<Enforcement>` (criterion-level) and `Option<&GateSection>` (spec-level default) and returns `Enforcement`. This is already the correct API for the CLI to determine enforcement per criterion.

5. **`load_gate_context()`** (line ~800-915) currently calls `process::exit(1)` on failures. Converting it to return `anyhow::Result<(PathBuf, Config, PathBuf, Option<u64>)>` is straightforward — each error path becomes `?` with `.context()`.

6. **Gate parent command doc string** on line 80 says `"Run quality gates for a spec"` — change to `"Manage quality gates"` per CONTEXT.md.

7. **No color detection crate needed.** The current code passes `color: bool` through `StreamConfig`. The existing logic for determining `color` (checking `--no-color` or `NO_COLOR` env var) continues to work unchanged.
