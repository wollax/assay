---
id: S02
milestone: M009
status: ready
---

# S02: README + example manifest documentation — Context

## Goal

Workspace-level `README.md` with project overview, install instructions, quickstart walkthrough, full subcommand reference (including deep `smelt serve` coverage), and all 7 example manifests documented with field-level inline comments.

## Why this Slice

No README exists. New users and contributors have no entry point. Example manifests vary in comment coverage (0 comments on k8s, some on compose/base). This is the primary launchability gap — the project works but is undiscoverable. Independent of S01 and S03; can ship in any order.

## Scope

### In Scope

- Workspace-level `README.md` with:
  - Project overview (what Smelt is, its role in the smelt/assay/cupel ecosystem)
  - Install via `cargo install --path .` (no binary releases or homebrew yet)
  - Quickstart walkthrough: `smelt init` → edit manifest → `smelt run --dry-run` → `smelt run` → `smelt status` → `smelt watch`
  - Per-subcommand reference section covering all 6 subcommands (init, list, run, serve, status, watch)
  - Full `smelt serve` documentation: HTTP API endpoints (POST/GET/DELETE), TUI overview, SSH worker config, server.toml structure, queue persistence behavior
- Field-level inline comments on all 7 example files:
  - `job-manifest.toml` — base Docker runtime example
  - `job-manifest-compose.toml` — Compose runtime with services
  - `job-manifest-k8s.toml` — Kubernetes runtime (currently has 0 comments)
  - `job-manifest-forge.toml` — forge/PR integration
  - `server.toml` — serve daemon config
  - `agent-manifest.toml` — agent manifest example
  - `bad-manifest.toml` — invalid manifest for testing/validation demo

### Out of Scope

- Per-crate README files (smelt-core/README.md, smelt-cli/README.md)
- Library embedding guide (cargo doc is sufficient for smelt-core consumers)
- Binary release / homebrew install instructions (no releases exist yet)
- mdBook, docs.rs setup, or API reference website
- Changelog generation
- New features or behavior changes

## Constraints

- README audience is CLI users, not library embedders — library API docs live in `cargo doc`
- D125: M009 is documentation/cleanup only — no behavior changes
- All existing tests must continue to pass (no code changes expected, but verify)
- Example manifests must remain valid TOML — `#` comments only, no structural changes

## Integration Points

### Consumes

- `cargo run -- --help` and subcommand `--help` output — source of truth for CLI flags and descriptions
- `examples/*.toml` — existing example files to annotate
- `examples/server.toml` — existing serve config to document
- `.kata/PROJECT.md` — project description for README overview

### Produces

- `README.md` at workspace root — primary user-facing documentation
- Updated `examples/*.toml` files — field-level inline comments added to all 7 files

## Open Questions

- None — all grey areas resolved during discuss phase.
