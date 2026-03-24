# M008: PR Workflow + Plugin Parity

**Vision:** Complete the platform with advanced PR automation (labels, reviewers, templates, PR status in TUI), three-platform plugin parity (OpenCode), and gate history analytics (failure trends, milestone velocity). After M008, Assay is a complete spec-driven development platform across all three agent surfaces.

## Success Criteria

- `assay pr create <milestone>` passes `--label` and `--reviewer` flags to `gh` from milestone TOML fields; a PR is created with those labels and reviewers
- TUI dashboard shows PR status badge (open/merged/closed) and CI check summary for milestones with an open PR, refreshed on a background polling interval
- OpenCode plugin installed in `plugins/opencode/` with AGENTS.md + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan) that reference correct MCP tool names
- `assay history --analytics` outputs gate failure frequency and milestone completion velocity as structured text
- TUI analytics screen (`a` key from Dashboard) shows gate failure heatmap and milestone velocity summary
- `just ready` passes with all new tests green

## Key Risks / Unknowns

- **PR status polling in sync TUI** — `gh pr view --json` is a subprocess call that blocks the TUI event loop. Must use background thread + channel to avoid freezing.
- **Milestone TOML backward compatibility** — Adding `pr_labels`, `pr_reviewers`, `pr_body_template` to `Milestone` must not break existing TOML files (the `deny_unknown_fields` + `serde(default)` pattern).

## Proof Strategy

- PR status polling risk → retire in S02 by building the real TUI PR status panel with background `gh` subprocess polling via the existing channel event loop (D107)
- TOML backward compat risk → retire in S01 by adding fields with the D092 pattern and verifying existing TOML round-trips unchanged

## Verification Classes

- Contract verification: unit tests (TOML round-trip, analytics aggregation, PR arg construction), integration tests (TUI PR status panel, analytics screen), insta snapshots for new schema fields
- Integration verification: `gh pr create` with labels/reviewers via mock `gh` binary (same pattern as M005/S04)
- Operational verification: `just ready` passes
- UAT / human verification: real `gh pr create` with labels on a real repo; real TUI PR status polling against a live PR; OpenCode plugin installation and skill invocation

## Milestone Definition of Done

This milestone is complete only when all are true:

- All slice checkboxes are `[x]` in this roadmap
- `assay pr create` passes labels and reviewers from milestone TOML to `gh`
- TUI shows live PR status for milestones with open PRs
- OpenCode plugin has AGENTS.md + 5 skills with correct MCP tool references
- `assay history --analytics` produces gate failure frequency and milestone velocity output
- TUI analytics screen renders gate failure heatmap and milestone velocity
- `just ready` passes with zero warnings
- All success criteria re-checked against running code

## Requirement Coverage

- Covers: R057 (OpenCode plugin — S03), R058 (Advanced PR workflow — S01 + S02), R059 (Gate history analytics — S04)
- Partially covers: none
- Leaves for later: none
- Orphan risks: none — all 3 active requirements mapped to slices

## Slices

- [x] **S01: Advanced PR creation with labels, reviewers, and templates** `risk:medium` `depends:[]`
  > After this: user adds `pr_labels = ["ready-for-review"]` and `pr_reviewers = ["teammate"]` to milestone TOML; `assay pr create` creates the PR with those labels and reviewer assigned. Proven by integration tests with mock `gh` binary.

- [x] **S02: TUI PR status panel with background polling** `risk:high` `depends:[S01]`
  > After this: TUI dashboard shows a PR status badge (open/merged/closed) and CI check summary next to milestones with open PRs, polled via background thread every 60s. Proven by integration tests with mock `gh` binary.

- [x] **S03: OpenCode plugin with full skill parity** `risk:low` `depends:[]`
  > After this: `plugins/opencode/` contains AGENTS.md + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan) matching Codex plugin structure. Proven by file existence and structural checks.

- [ ] **S04: Gate history analytics engine and CLI** `risk:medium` `depends:[]`
  > After this: `assay history --analytics` outputs gate failure frequency (which criteria fail most) and milestone completion velocity (chunks per day). Proven by unit tests with synthetic history records.

- [ ] **S05: TUI analytics screen** `risk:low` `depends:[S04]`
  > After this: pressing `a` from Dashboard opens an analytics screen showing gate failure heatmap and milestone velocity. Proven by integration tests driving synthetic key events.

## Boundary Map

### S01 → S02

Produces:
- `Milestone.pr_labels: Option<Vec<String>>` — new field in assay-types with serde(default, skip_serializing_if)
- `Milestone.pr_reviewers: Option<Vec<String>>` — new field in assay-types with serde(default, skip_serializing_if)
- `Milestone.pr_body_template: Option<String>` — new field in assay-types with serde(default, skip_serializing_if)
- Updated `pr_create_if_gates_pass()` in assay-core::pr that passes `--label` and `--reviewer` flags to `gh pr create`
- Updated `pr_create` MCP tool that accepts optional labels/reviewers params
- Schema snapshot updated for Milestone type

Consumes:
- nothing (extends existing Milestone type and pr.rs)

### S01 → S05

Produces:
- `Milestone.pr_number` and `Milestone.pr_url` fields (already exist) — used by TUI to decide which milestones to show PR status for

Consumes:
- nothing (fields already exist)

### S02 (standalone)

Produces:
- `pr_status_poll()` free function in assay-core::pr returning `PrStatusInfo { state, ci_status, review_status }`
- `TuiEvent::PrStatusUpdate { slug, info }` variant for background polling delivery
- Dashboard draw function enhanced with PR status badge rendering

Consumes from S01:
- `Milestone.pr_labels`, `Milestone.pr_reviewers` (for display in PR status panel if desired)
- `Milestone.pr_number` (to know which milestones have PRs to poll)

### S03 (standalone)

Produces:
- `plugins/opencode/AGENTS.md` — workflow guide with skills/MCP tables
- `plugins/opencode/skills/gate-check.md`
- `plugins/opencode/skills/spec-show.md`
- `plugins/opencode/skills/cycle-status.md`
- `plugins/opencode/skills/next-chunk.md`
- `plugins/opencode/skills/plan.md`
- `.gitkeep` files removed from skills/ and agents/ directories

Consumes:
- nothing (pure markdown, references existing MCP tool names from M005)

### S04 → S05

Produces:
- `assay-core::history::analytics` module with `AnalyticsReport { failure_frequency, milestone_velocity }` type
- `compute_analytics(assay_dir, options) -> Result<AnalyticsReport>` free function
- `FailureFrequency { criterion_name, spec_name, fail_count, total_runs }` and `MilestoneVelocity { milestone_slug, chunks_completed, days_elapsed }` structs
- `assay history --analytics` CLI subcommand

Consumes:
- `history::list()` and `history::load()` from assay-core (existing API, no changes)
- `milestone_scan()` from assay-core (existing API, for velocity calculation)

### S05 (standalone)

Produces:
- `Screen::Analytics` variant in assay-tui
- `draw_analytics(frame, area, report)` free function
- `a` key handler from Dashboard transitioning to Analytics screen

Consumes from S04:
- `compute_analytics()` function for data
- `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` types for rendering
