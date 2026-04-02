---
estimated_steps: 5
estimated_files: 1
---

# T01: Write workspace README.md

**Slice:** S02 — README + example manifest documentation
**Milestone:** M009

## Description

Write a comprehensive `README.md` at the workspace root. Smelt currently has no README — new users and contributors have zero entry point (R041). The README must accurately document all 6 subcommands, install instructions, a quickstart walkthrough, and the Smelt/Assay/Cupel ecosystem. Every claim must be verified against actual `--help` output and manifest schema — no invented features.

## Steps

1. Read `--help` output for all 6 subcommands to capture exact flag names, descriptions, and argument syntax
2. Write the README with these sections in order: title/badge, what Smelt is (from PROJECT.md), install (`cargo install --path .`), quickstart (`smelt init` → edit manifest → `smelt run --dry-run` → `smelt run`), subcommand reference (init, list, run, serve, status, watch — each with synopsis, flags table, and a one-liner example), server mode overview (config file, directory watch, HTTP API, TUI), example walkthrough (link to `examples/` with brief description of each file), ecosystem table (Smelt/Assay/Cupel roles)
3. Cross-reference every flag and option against `--help` output — verify no flag is misnamed, missing, or invented
4. Verify the README is self-consistent: quickstart commands reference real subcommands; example file names match `examples/` directory
5. Run `cargo test --workspace` to confirm no regressions (README is docs-only but verify nothing was accidentally changed)

## Must-Haves

- [ ] `README.md` exists at workspace root with 200+ lines
- [ ] Covers all 6 subcommands: init, list, run, serve, status, watch
- [ ] Every flag/option matches actual `--help` output exactly
- [ ] Install section documents `cargo install --path .`
- [ ] Quickstart section walks through `smelt init` → `smelt run --dry-run`
- [ ] Ecosystem table mentions Assay and Cupel with their roles
- [ ] Links to `examples/` directory for manifest reference

## Verification

- `test -f README.md` — file exists
- `wc -l README.md` — 200+ lines
- Manually cross-check: every flag in each subcommand section matches `cargo run -- <cmd> --help`
- `cargo test --workspace` — all tests pass

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Read `README.md`
- Failure state exposed: None

## Inputs

- `cargo run -- <cmd> --help` for all 6 subcommands — authoritative CLI interface
- `.kata/PROJECT.md` — narrative about what Smelt is and ecosystem context
- `examples/` directory listing — what example files exist

## Expected Output

- `README.md` — comprehensive workspace-level documentation covering project overview, install, quickstart, all subcommands, server mode, examples, and ecosystem
