# M006: TUI as Primary Surface — Context

**Gathered:** 2026-03-19
**Status:** Provisional — detail-planning deferred until M005 is complete

## Project Description

Assay's TUI becomes the preferred primary interface, replacing the current 42-line stub with a real Ratatui application. M006 delivers a project dashboard (milestones, chunk progress, gate status), an interactive spec authoring wizard, a spec browser, and a provider/model configuration UI. The TUI reads from `.assay/` and does not yet spawn agents — that comes in M007.

## Why This Milestone

M005 establishes the data model and CLI/plugin workflow. M006 surfaces that data in a visual, keyboard-navigable terminal application. The TUI is the "install and start" experience for developers who want Assay without configuring Claude Code or Codex first.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Launch `assay` (or `assay-tui`) and see a project dashboard: all milestones listed with status indicators, chunk progress bars, and gate pass/fail counts
- Navigate into a milestone, see its chunks, select a chunk to see its criteria and latest gate run result
- Press a key to launch the authoring wizard from inside the TUI and create a new milestone + chunks without leaving the terminal
- Open a settings screen to configure AI provider and model per phase

### Entry point / environment

- Entry point: `assay` TUI binary (or `assay-tui`)
- Environment: local terminal, any project with `.assay/`
- Live dependencies involved: none (no agent spawning yet)

## Completion Class

- Contract complete means: all TUI screens render without panic, keyboard navigation works, data reads from `.assay/` via assay-core
- Integration complete means: a real `.assay/` project is viewable in the TUI with correct milestone/chunk/gate data
- Operational complete means: `just ready` passes; TUI binary builds and launches without error

## Final Integrated Acceptance

- Launch `assay` on a project with milestones from M005 — dashboard shows correct status, chunks, and gate results
- Create a new milestone via the in-TUI wizard — it appears in the dashboard immediately
- Configure provider settings — persists to `.assay/config.toml` and survives restart

## Risks and Unknowns

- **Ratatui layout complexity** — multi-panel layouts (dashboard + detail + wizard) require careful widget composition. Mitigate by starting with a simple list view and layering complexity.
- **Event loop + async data loading** — TUI event loop must stay responsive while reading `.assay/` data. Use a background thread or tokio task for data loading.
- **Provider config schema extension** — extending the existing `Config` type in assay-types must be backward-compatible.

## Existing Codebase / Prior Art

- `crates/assay-tui/src/main.rs` — current stub (42 lines): replace entirely, preserve binary entry point
- `crates/assay-core/` — all milestone, spec, gate history reads are done through assay-core
- `Cargo.toml` — ratatui already in deps; add crossterm event handling

## Relevant Requirements

- R049 — TUI project dashboard
- R050 — TUI interactive wizard
- R051 — TUI spec browser
- R052 — TUI provider configuration

## Scope

### In Scope

- Real Ratatui TUI with dashboard, detail views, wizard, settings
- Reads from `.assay/` via assay-core (read-only except for wizard + settings)
- Keyboard navigation, help screen

### Out of Scope (M006)

- Agent spawning / live gate execution (M007)
- MCP server management (M007)
- Slash commands (M007)
- PR status display (M008)
