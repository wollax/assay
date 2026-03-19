# M008: PR Workflow + Plugin Parity — Context

**Gathered:** 2026-03-19
**Status:** Provisional — detail-planning deferred until M007 is complete

## Project Description

M008 completes the platform vision: advanced PR workflow (labels, reviewers, templates, PR status in TUI), full OpenCode plugin parity, and history analytics (gate failure trends, milestone velocity). This milestone polishes the delivery workflow and ensures all three major agent platforms have a fully-featured Assay integration.

## Why This Milestone

M005 ships basic gate-gated PRs. M008 makes PRs usable in real team workflows — with labels, reviewer assignment, and body templates, Assay PRs integrate naturally with project conventions. The OpenCode plugin closes the three-platform parity gap. History analytics give developers and teams feedback on spec quality and delivery velocity.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Configure PR labels, default reviewers, and a PR body template in `.assay/milestones/<slug>.toml` — `assay pr create` uses them automatically
- See PR status (open/merged/closed, CI check status, review status) in the TUI dashboard for any milestone with an open PR
- Install the Assay OpenCode plugin and get the same gate-check, spec-show, cycle-status, and plan skills as Claude Code and Codex
- View a gate failure heatmap and milestone completion velocity chart in the TUI analytics panel, or via `assay history --analytics`

### Entry point / environment

- Entry point: TUI analytics panel, `assay pr create`, `plugins/opencode/` installation
- Environment: local dev + GitHub (for PR status)
- Live dependencies involved: `gh` CLI (PR status), OpenCode (plugin)

## Completion Class

- Contract complete means: PR config fields round-trip through TOML; analytics data aggregated from existing history records
- Integration complete means: PR created with labels+reviewer via `gh`; PR status shown correctly in TUI from `gh pr view --json`; OpenCode plugin produces correct skills output
- Operational complete means: `just ready` passes

## Final Integrated Acceptance

- Create a milestone with `pr_labels: ["ready-for-review"]` and `pr_reviewers: ["teammate"]` in TOML — `assay pr create` creates PR with those labels and reviewer
- Open TUI on that project — PR status panel shows the PR as open with correct CI status
- Install OpenCode plugin — `/plan` skill calls `milestone_create` + `spec_create` MCP tools correctly

## Risks and Unknowns

- **GitHub API rate limits for PR status polling** — TUI must not hammer `gh pr view` on every render tick. Mitigate with a background polling interval (e.g., every 60s).
- **OpenCode plugin format stability** — opencode.json format may change. Monitor OpenCode releases.
- **Analytics query performance** — aggregating across many history records should stay fast. Limit analytics to last 90 days or last 100 runs by default.

## Existing Codebase / Prior Art

- `crates/assay-core/src/history/` — existing gate run history: analytics aggregate from here
- `plugins/opencode/` — scaffold already exists (package.json, opencode.json, tsconfig.json): fill in skills + AGENTS.md
- M005 `pr_create` implementation: M008 extends with labels/reviewers/templates
- M006/M007 TUI framework: M008 adds PR status panel + analytics panel

## Relevant Requirements

- R057 — OpenCode plugin
- R058 — Advanced PR workflow
- R059 — Gate history analytics

## Scope

### In Scope

- PR config: labels, reviewers, body template in milestone TOML
- PR status panel in TUI (via `gh pr view --json` polling)
- OpenCode plugin: AGENTS.md + 4 skills
- History analytics: gate failure trends, milestone velocity (TUI panel + CLI command)

### Out of Scope (M008)

- Cloud sync or team-shared analytics
- Billing or marketplace features
- CI/CD integration beyond `gh`
