# Phase 18 Context: CLI Hardening & Enforcement Surface

## Scope

Phase 18 requirements from ROADMAP.md:
- CLI-01: `main()` returns `Result` for proper error propagation
- CLI-02: Bare `assay` invocation exits with non-zero code
- CLI-03: `.assay` directory path is extracted to a named constant
- CLI-04: Gate command help duplication is resolved
- ENFC-03: CLI exit code reflects only required criterion failures; advisory failures are warnings

## Decisions

### 1. Error Propagation Model

- **Error type:** Use `anyhow::Result` in the binary crate. `anyhow` is acceptable as a binary-only dependency.
- **Architecture:** Catch-at-top pattern. `main()` calls `run()` which returns `anyhow::Result<i32>`. The `i32` is the exit code.
  - `Ok(0)` — success (all required pass, status view)
  - `Ok(1)` — gate failure (required criteria failed, no runtime error)
  - `Err(e)` — runtime/infrastructure error (always exits 1)
- **`main()` body:** Calls `run()`, matches result, calls `process::exit(code)`. Formats errors with `eprintln!("Error: {e:#}")` for cause chains.
- **Bare invocation (CLI-02):**
  - Inside a project (`.assay` dir exists): show status dashboard, exit 0.
  - Not a project: print help + hint, exit non-zero (via `Err` or `Ok(1)`).

### 2. Enforcement-Aware Exit Codes and Output

- **Exit code rule:** Advisory warnings NEVER affect the exit code. Only required failures cause non-zero exit.
- **Streaming output labels:**
  - Required failures: red **FAILED**
  - Advisory failures: yellow **WARN**
  - All advisory criteria are always labeled (e.g., `[advisory] lint-style ... ok`) even when passing, so the user always sees which track each criterion belongs to.
- **Summary line:** Single line with warn category added: `3 passed, 1 failed, 2 warned, 1 skipped (of 7 total)`
  - "failed" counts only required failures
  - "warned" counts only advisory failures
- **`--all` mode:** Overall exit 0 if all required pass across all specs, regardless of advisory warnings in any spec.

### 3. Help Duplication Resolution

- **Parent command descriptions** follow a consistent terse pattern:
  - `init` — "Initialize a new Assay project in the current directory" (leaf command, left as-is)
  - `mcp` — "MCP server operations"
  - `spec` — "Manage spec files"
  - `gate` — "Manage quality gates"
- **Top-level examples:** Stay minimal — one example per subcommand group. No `gate history` example in top-level help; users discover subcommands via `--help`.

## Deferred Ideas

None surfaced during discussion.

## Current State (for researcher/planner)

- `main.rs` is 1657 lines with `async fn main()` (no return type)
- 41 `process::exit(1)` calls across handler functions
- 12 occurrences of `.join(".assay")` — candidates for the named constant (CLI-03)
- `StreamCounters` tracks `passed/failed/skipped` — needs `warned` field for advisory failures
- Gate parent command doc says "Run quality gates for a spec" — needs update to "Manage quality gates"
