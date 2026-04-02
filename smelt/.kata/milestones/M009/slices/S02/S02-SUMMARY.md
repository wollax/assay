---
id: S02
parent: M009
milestone: M009
provides:
  - Comprehensive workspace README.md (335 lines, all 6 subcommands documented)
  - Field-level inline comments on all 7 example TOML files
  - Fixed broken agent-manifest.toml (invalid [manifest] key and session fields)
  - bad-manifest.toml documents all 7 intentional validation violations
  - Example file comment style pattern (header block + inline field comments)
requires:
  - slice: none
    provides: independent slice — no upstream dependencies
affects: []
key_files:
  - README.md
  - examples/agent-manifest.toml
  - examples/bad-manifest.toml
  - examples/job-manifest.toml
  - examples/job-manifest-compose.toml
  - examples/job-manifest-forge.toml
  - examples/job-manifest-k8s.toml
  - examples/server.toml
key_decisions: []
patterns_established:
  - "Example file comment style: header block explaining purpose + run command, then inline comments above or beside each field"
observability_surfaces:
  - none (pure documentation slice)
drill_down_paths:
  - .kata/milestones/M009/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M009/slices/S02/tasks/T02-SUMMARY.md
duration: 23min
verification_result: passed
completed_at: 2026-03-24T20:00:00Z
---

# S02: README + example manifest documentation

**335-line workspace README with install/quickstart/all 6 subcommands + field-level comments on all 7 example manifests, including fix for broken agent-manifest.toml**

## What Happened

T01 wrote a comprehensive `README.md` at workspace root from scratch. Every subcommand flag and option was cross-referenced against actual `--help` output. Sections cover: project overview, install, quickstart (init → edit → dry-run → run), all 6 subcommands (init, list, run, serve, status, watch) with usage blocks and flag tables, server mode configuration, examples directory, runtimes overview, and the Smelt/Assay/Cupel ecosystem.

T02 annotated all 7 example TOML files with field-level inline comments. `agent-manifest.toml` required a full rewrite — beyond the known `[manifest]→[job]` fix, it also used invalid session fields (`task`, `file_scope`, `timeout_secs` instead of `spec`, `harness`, `timeout`) and was missing required sections (environment, credentials, merge). All field names were verified against the serde structs in `manifest.rs`, `forge.rs`, and `config.rs`. `bad-manifest.toml` gained `# VIOLATION:` comments documenting each of its 7 intentional errors. All 6 valid examples verified with `--dry-run`; bad-manifest confirmed to exit non-zero with all 7 validation errors detected.

## Verification

- `README.md` exists: 335 lines (must-have: 200+) ✓
- All 6 subcommands documented with accurate flags ✓
- `cargo run -- run examples/job-manifest.toml --dry-run` → exit 0 ✓
- `cargo run -- run examples/job-manifest-forge.toml --dry-run` → exit 0 ✓
- `cargo run -- run examples/job-manifest-compose.toml --dry-run` → exit 0 ✓
- `cargo run -- run examples/job-manifest-k8s.toml --dry-run` → exit 0 ✓
- `cargo run -- run examples/agent-manifest.toml --dry-run` → exit 0 ✓
- `cargo run -- run examples/bad-manifest.toml --dry-run` → exit 1 (7 validation errors) ✓
- `grep -c '^#' examples/agent-manifest.toml` → 28 (must-have: ≥5) ✓
- `grep -c '^#' examples/job-manifest-k8s.toml` → 47 (must-have: ≥10) ✓
- `cargo test --workspace` → all tests pass, 0 failures ✓

## Requirements Advanced

- R041 — README.md now exists with project overview, install, quickstart, and all 6 subcommand documentation
- R045 — All 7 example manifests have inline field-level comments; agent-manifest.toml fixed from broken to parseable

## Requirements Validated

- R041 — README exists, covers all subcommands with accurate flags verified against --help; 335 lines
- R045 — Every field in every example has an inline comment; all parseable examples verified with --dry-run; bad-manifest errors documented

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

agent-manifest.toml required more extensive rewriting than planned. The original had three problems, not one: (1) `[manifest]` instead of `[job]`, (2) invalid session fields (`task`/`file_scope`/`timeout_secs`), and (3) missing required sections (`[environment]`, `[credentials]`, `[merge]`). The file was fully rewritten as a minimal valid manifest.

## Known Limitations

- README subcommand documentation is a point-in-time snapshot of `--help` output — future flag changes require manual README updates
- No automated check that README stays in sync with CLI help text

## Follow-ups

- none

## Files Created/Modified

- `README.md` — New file: comprehensive workspace-level documentation (335 lines)
- `examples/agent-manifest.toml` — Rewritten: fixed invalid keys and fields, added all required sections, full comments
- `examples/bad-manifest.toml` — Added VIOLATION comments documenting all 7 validation errors
- `examples/job-manifest.toml` — Expanded to full field-level comments on every field
- `examples/job-manifest-compose.toml` — Expanded comments including [[services]] passthrough explanation
- `examples/job-manifest-forge.toml` — Expanded comments including [forge] section documentation
- `examples/job-manifest-k8s.toml` — Added header and full field-level comments on all [kubernetes] fields
- `examples/server.toml` — Expanded comments with [[workers]] field documentation

## Forward Intelligence

### What the next slice should know
- README and examples are now complete — S03 (large file decomposition) should not need to touch these files unless module moves change public API signatures that are documented in the README
- The example comment style uses a header block (purpose + run command) followed by inline field comments — maintain this pattern if new examples are added

### What's fragile
- README flag tables are hand-written from --help output — if S03 refactoring changes any public CLI flags (unlikely but possible), the README would drift silently

### Authoritative diagnostics
- `cargo run -- run examples/<file>.toml --dry-run` is the canonical way to verify example correctness — tests every field against the real parser

### What assumptions changed
- agent-manifest.toml was more broken than expected (3 problems, not 1) — fixed completely
