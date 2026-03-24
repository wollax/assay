# S01: Structured tracing foundation and eprintln migration — UAT

**Milestone:** M009
**Written:** 2026-03-24

## UAT Type

- UAT mode: mixed (artifact-driven + live-runtime)
- Why this mode is sufficient: Zero-eprintln is provable by grep (artifact). Structured output behavior requires running a real command (live-runtime). No human-experience testing needed — output format is developer tooling, not end-user UX.

## Preconditions

- Assay project initialized (`assay init` run, `.assay/` exists)
- At least one spec with gates configured (e.g. a passing `gates.toml`)
- Rust toolchain installed (`cargo build` works)

## Smoke Test

Run `RUST_LOG=debug assay gate run <spec>` and confirm stderr shows structured, leveled output (timestamps, level tags like INFO/WARN/DEBUG, module paths) instead of bare unformatted text.

## Test Cases

### 1. Zero eprintln in production code

1. Run: `grep -rn 'eprintln!' crates/assay-cli/src/ crates/assay-core/src/ crates/assay-tui/src/ crates/assay-mcp/src/ --include='*.rs'`
2. **Expected:** Zero matches (exit code 1, no output)

### 2. Interactive eprint! prompts preserved

1. Run: `grep -rn 'eprint!' crates/ --include='*.rs' | grep -v eprintln`
2. **Expected:** Exactly 3 matches: gate.rs (carriage-return progress), worktree.rs (2 y/N prompts)

### 3. Default CLI output is leveled

1. Run: `assay gate run <spec>` (no RUST_LOG set)
2. **Expected:** stderr shows info-level events (phase banners, criterion results). No debug-level detail visible.

### 4. RUST_LOG filtering works

1. Run: `RUST_LOG=error assay gate run <spec>`
2. **Expected:** Only error-level events appear on stderr. Info/warn/debug events suppressed.

### 5. Per-crate filtering works

1. Run: `RUST_LOG=assay_cli::commands::gate=debug,warn assay gate run <spec>`
2. **Expected:** Gate module shows debug-level evidence output. Other modules show only warn+.

### 6. MCP serve uses warn level

1. Run: `RUST_LOG=info assay mcp serve` (briefly, then Ctrl-C)
2. **Expected:** No info-level startup banners on stderr (MCP defaults to warn). Only the JSON-RPC protocol on stdout.

### 7. Telemetry unit tests pass

1. Run: `cargo test -p assay-core telemetry`
2. **Expected:** 3 tests pass (test_default_config, test_mcp_config, test_init_tracing_returns_guard)

## Edge Cases

### Invalid RUST_LOG value

1. Run: `RUST_LOG=not_a_valid_filter assay gate run <spec>`
2. **Expected:** Falls back to default level (info for CLI). No crash, no panic. Application runs normally.

### Double init (programmatic)

1. Call `init_tracing()` twice in the same process (e.g. via a test)
2. **Expected:** Second call is a silent no-op via `try_init()`. No panic.

## Failure Signals

- Any `eprintln!` match in the four production crates
- Bare unformatted text on stderr when running any assay command
- Crash or panic when RUST_LOG is set to an invalid value
- Missing structured fields on tracing events (e.g. gate criterion results without `criterion_name`)

## Requirements Proved By This UAT

- R060 (Structured tracing foundation) — tests 1-6 prove zero eprintln, structured leveled output, RUST_LOG filtering, and per-crate control
- R027 (OTel instrumentation) — partially proved: foundation layer architecture exists. Full proof requires S02-S05.

## Not Proven By This UAT

- Pipeline span instrumentation (R061 — S02 scope)
- Orchestration span nesting (R062 — S03 scope)
- JSON file trace export (R063 — S04 scope)
- OTLP export and context propagation (R064, R065 — S05 scope)
- Trace viewer in TUI (R066 — deferred)

## Notes for Tester

- The assay-mcp crate has 2 pre-existing clippy warnings (unused `description` field) — these are unrelated to this slice
- The `orchestrate_integration` test suite is slow (~300s) — this is pre-existing, not caused by the tracing migration
- Guard daemon no longer writes to `guard.log` — this is intentional (file logging deferred to S04)
