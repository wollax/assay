---
id: T01
parent: S02
milestone: M009
provides:
  - Comprehensive workspace README.md (335 lines)
  - All 6 subcommands documented with exact flags from --help
  - Install, quickstart, server mode, examples, and ecosystem sections
key_files:
  - README.md
key_decisions: []
patterns_established: []
observability_surfaces: []
duration: 8min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T01: Write workspace README.md

**335-line README.md covering install, quickstart, all 6 subcommands with exact flag tables, server mode, examples directory, and Smelt/Assay/Cupel ecosystem**

## What Happened

Wrote `README.md` at workspace root from scratch. Gathered `--help` output for all 6 subcommands (init, list, run, serve, status, watch) and the top-level binary. Every flag name, description, and argument syntax in the README was copied directly from `--help` output — no invented features. Structured the README with: title, project description (from PROJECT.md), install instructions, quickstart walkthrough (init → edit → dry-run → run), per-subcommand reference sections with usage blocks, flags tables, and examples, server mode configuration details (config file, HTTP API endpoints, queue persistence, SSH worker pools, TUI), examples directory table linking all 7 files, ecosystem table (Smelt/Assay/Cupel roles), runtimes overview, and project state location.

## Verification

- `test -f README.md` — exists ✓
- `wc -l README.md` → 335 lines (must-have: 200+) ✓
- All 6 subcommands covered: init, list, run, serve, status, watch ✓
- Every flag cross-checked against `cargo run -- <cmd> --help` output ✓
- Install section: `cargo install --path .` ✓
- Quickstart: `smelt init` → edit → `smelt run --dry-run` → `smelt run` ✓
- Ecosystem table: Smelt, Assay, Cupel with roles ✓
- Links to `examples/` directory ✓
- `cargo test --workspace` — all tests pass (155 smelt-core + smelt-cli + doctests, 0 failures) ✓

### Slice-level checks (T01 scope):
- `test -f README.md && wc -l README.md` — 335 lines ✓
- `cargo test --workspace` — all pass ✓

## Diagnostics

None — pure documentation artifact. Inspect by reading `README.md`.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `README.md` — Comprehensive workspace-level documentation (new file, 335 lines)
