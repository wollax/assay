---
id: T04
parent: S01
milestone: M001
provides:
  - "`smelt run --dry-run` CLI command that loads, validates, and prints a structured execution plan"
  - "10 integration tests covering happy path, validation errors, credential resolution, and secret redaction"
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
key_decisions:
  - "Validation failures print to stderr and exit 1 via Ok(1) return — not via anyhow error propagation — so the error message is clean without the 'Error:' prefix"
  - "Execution plan output goes to stdout, tracing/errors go to stderr — enables piping plan output"
patterns_established:
  - "CLI dry-run pattern: load → validate → resolve credentials → print plan; each phase has structured tracing at info/error level"
  - "Integration tests use workspace_root() helper via CARGO_MANIFEST_DIR to locate example manifests regardless of test working directory"
observability_surfaces:
  - "`smelt run --dry-run` prints structured execution plan showing all manifest sections, credential resolution status, and session dependency graph"
  - "Structured tracing at info level for load/validate/plan-print phases; error level for validation failures with field-level detail"
  - "Credential values are never printed — only source (env:VARNAME) and status (resolved/MISSING)"
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T04: `smelt run --dry-run` CLI and execution plan printer

**Implemented the dry-run CLI command that loads a manifest, validates it, resolves credential sources, and prints a structured execution plan with section headers and credential redaction.**

## What Happened

Replaced the stub `run` subcommand with a full dry-run implementation. The command follows a four-phase pipeline: load manifest → validate → resolve credentials → print execution plan. The execution plan printer shows all manifest sections (job metadata, environment with resources, credentials with resolution status, sessions with dependencies/timeouts, merge config) in a human-readable format with aligned fields and section headers.

Without `--dry-run`, the command prints a placeholder message and exits 1 (for S02 Docker implementation).

Wrote 10 integration tests using `assert_cmd` covering: valid manifest plan output (job info, sessions, resources, merge config, credential status both resolved and missing), bad manifest validation errors, nonexistent manifest errors, unimplemented non-dry-run mode, and explicit verification that credential secret values never appear in output.

## Verification

- `cargo test -p smelt-core` — 58 tests pass
- `cargo test -p smelt-cli` — 13 tests pass (3 unit + 10 integration)
- `cargo build --workspace` — zero errors, zero warnings
- `cargo run -- run examples/job-manifest.toml --dry-run` — prints structured execution plan with all sections
- `cargo run -- run examples/bad-manifest.toml --dry-run` — exits 1 with 7 validation errors (empty name, empty image, zero timeout, duplicate session, unknown dependency, empty target, unknown merge order entry)

All slice-level verification checks pass. This is the final task of S01.

## Diagnostics

- `smelt run <manifest> --dry-run` is the primary diagnostic surface — shows exactly what Smelt would do
- Set `SMELT_LOG=info` to see structured tracing for each pipeline phase (load, validate, resolve, print)
- Validation errors list every field violation, not just the first one
- Credential resolution shows `env:VARNAME → resolved` or `env:VARNAME → MISSING` without revealing values

## Deviations

None.

## Known Issues

- `assert_cmd::Command::cargo_bin` emits a deprecation warning — upstream issue, no functional impact

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — rewrote stub into full dry-run implementation with load/validate/resolve/print pipeline
- `crates/smelt-cli/tests/dry_run.rs` — new: 10 integration tests for the dry-run CLI
