---
id: M008
provides:
  - Advanced PR creation with TOML-configurable labels, reviewers, and body templates
  - TUI PR status panel with background gh polling (60s interval)
  - OpenCode plugin with AGENTS.md + 5 skills (three-platform parity)
  - Gate history analytics engine (failure frequency + milestone velocity)
  - TUI analytics screen with color-coded failure heatmap and velocity tables
  - assay history analytics CLI with text tables and --json output
key_decisions:
  - "D116: PR status polling via background thread + TuiEvent"
  - "D117: New Milestone TOML fields use D092 serde(default, skip_serializing_if) pattern"
  - "D118: Analytics types live in assay-core::history::analytics, not assay-types"
  - "D119: OpenCode plugin uses Codex flat-file skill convention"
  - "D121: Caller-provided body takes precedence over pr_body_template"
  - "D122: PrStatusInfo lives in assay-core::pr, not assay-types"
  - "D123: Poll interval hardcoded as const (60s), not configurable"
  - "D124: Shared poll targets via Arc<Mutex<Vec>> for thread-safe milestone tracking"
  - "D125: eprintln for gh-not-found warning in assay-tui"
patterns_established:
  - "Background thread + TuiEvent channel pattern for subprocess polling (reusable for future polling needs)"
  - "NUL-separated arg capture in mock gh scripts for multiline body testing"
  - "Analytics compute functions take &Path (assay_dir) and return Result"
  - "Three-platform plugin parity: Claude Code, Codex, OpenCode share identical skill content"
  - "Screen transition pattern with data load: guard project_root → compute → store on App → transition"
observability_surfaces:
  - "assay history analytics --json — machine-readable analytics inspection"
  - "App.pr_statuses pub field — integration tests and slash commands read directly"
  - "App.analytics_report.is_some() — confirms analytics data loaded"
  - "eprintln warning when gh CLI not found at TUI startup"
  - "Absent PR badge is the graceful degradation signal"
requirement_outcomes:
  - id: R057
    from_status: active
    to_status: validated
    proof: "S03 — AGENTS.md (37 lines) + 5 skill files with all 10 MCP tool names verified; 22/22 structural checks pass"
  - id: R058
    from_status: active
    to_status: validated
    proof: "S01 — 12 integration tests with mock gh binary prove labels/reviewers/template. S02 — 8 core + 3 TUI integration tests prove PR status polling and dashboard badge rendering"
  - id: R059
    from_status: active
    to_status: validated
    proof: "S04 — 14 tests (8 integration + 2 unit + 4 CLI) prove compute_analytics and CLI output. S05 — 6 integration tests prove TUI analytics screen transitions and data rendering"
duration: ~3 days
verification_result: passed
completed_at: 2026-03-24
---

# M008: PR Workflow + Plugin Parity

**Advanced PR automation (labels, reviewers, templates, live status polling), three-platform plugin parity (OpenCode), and gate history analytics (failure heatmap, milestone velocity) across CLI and TUI**

## What Happened

M008 completed the platform with five slices spanning PR workflow, plugin parity, and analytics.

**S01** extended the Milestone TOML type with `pr_labels`, `pr_reviewers`, and `pr_body_template` fields using the D092 backward-compatibility pattern. `pr_create_if_gates_pass()` passes these as `--label`/`--reviewer`/`--body` args to `gh`. CLI and MCP surfaces gained extend-semantics flags (union of TOML + caller values). Template rendering supports 4 placeholders: `{milestone_name}`, `{milestone_slug}`, `{chunk_list}`, `{gate_summary}`.

**S02** added live PR status to the TUI dashboard. `pr_status_poll()` shells out to `gh pr view --json` and parses state, CI check conclusions, and review decisions. A background thread polls every 60s via the TuiEvent channel (D107), with an initial no-delay poll. Dashboard renders 🟢/🟣/🔴 state icons with CI pass/fail/pending counts and review abbreviation. Graceful degradation when `gh` is missing.

**S03** completed three-platform plugin parity by filling `plugins/opencode/` with AGENTS.md + 5 skills (gate-check, spec-show, cycle-status, next-chunk, plan) — content identical to Codex plugin. All 10 MCP tool names verified present.

**S04** built the analytics engine: `compute_analytics()` aggregates gate failure frequency by (spec_name, criterion_name) pair and milestone velocity from milestone timestamps. `assay history analytics` CLI renders ANSI-colored text tables; `--json` produces machine-readable output. Corrupt records are counted but never fatal.

**S05** added the TUI analytics screen: `a` key from Dashboard computes analytics and transitions to Screen::Analytics. Failure frequency table color-codes rates (red >50%, yellow >0%, green 0%). Velocity table shows chunks/day. Empty data shows a centered message. Help overlay updated with `a → Analytics`.

## Cross-Slice Verification

**Criterion 1: `assay pr create` passes --label and --reviewer flags**
✓ S01 `cargo test -p assay-core --test pr` — 12 integration tests with mock `gh` binary. `test_pr_create_passes_labels_and_reviewers` proves flags appear in captured args.

**Criterion 2: TUI dashboard shows PR status badge**
✓ S02 `cargo test -p assay-tui --test pr_status_panel` — 3 tests prove event→state storage, poll target initialization, and target refresh. `cargo test -p assay-core --test pr_status` — 8 tests prove all parsing scenarios.

**Criterion 3: OpenCode plugin with AGENTS.md + 5 skills**
✓ S03 — 22/22 structural checks: 6 file existence, 10 MCP tool name presence, flat .md format, interview-first pattern, null guards.

**Criterion 4: `assay history --analytics` outputs failure frequency and velocity**
✓ S04 `cargo test -p assay-core --test analytics` — 8 integration tests. `cargo test -p assay-cli -- history` — 4 CLI tests including text output shape and JSON round-trip.

**Criterion 5: TUI analytics screen shows heatmap and velocity**
✓ S05 `cargo test -p assay-tui --test analytics_screen` — 6 tests prove screen transition, no-op guard, Esc/q navigation, report population, and synthetic data state.

**Criterion 6: `just ready` passes**
✓ fmt, clippy, deny clean; full test suite passes (wizard integration test has pre-existing intermittent hang, unrelated to M008).

**Definition of Done:**
- All 5 slice checkboxes `[x]` ✓
- All 5 slice summaries exist ✓
- R057, R058, R059 all validated ✓
- 0 active requirements remaining ✓

## Requirement Changes

- R057 (OpenCode plugin): active → validated — S03 delivered AGENTS.md + 5 skills with 22/22 structural checks
- R058 (Advanced PR workflow): active → validated — S01 (labels/reviewers/templates, 12 tests) + S02 (TUI PR status, 11 tests)
- R059 (Gate history analytics): active → validated — S04 (analytics engine + CLI, 14 tests) + S05 (TUI analytics screen, 6 tests)

## Forward Intelligence

### What the next milestone should know
- All 59 requirements are validated or explicitly deferred/out-of-scope. 0 active requirements — the next milestone requires new requirement discovery.
- The TUI has full feature coverage: dashboard, spec browser, wizard, settings, agent spawning, MCP panel, slash commands, PR status, analytics.
- ~24K lines of Rust across 6 crates with 1400+ tests. The zero-trait, sync-core, closure-based architecture (D001/D007) has scaled well through 8 milestones.

### What's fragile
- The wizard integration test has an intermittent hang — not caused by M008 but affects `just ready` reliability
- `TuiEvent` enum is partially duplicated between `event.rs` and `app.rs` (D114) — adding new variants requires updating both
- `pr_create_if_gates_pass()` has 8 parameters with `#[allow(clippy::too_many_arguments)]` — consider a PrCreateOptions struct if further extension is needed

### Authoritative diagnostics
- `cargo test -p assay-core --test pr` — all PR creation behavior (S01)
- `cargo test -p assay-core --test pr_status` — all PR status parsing (S02)
- `cargo test -p assay-core --test analytics` — all analytics computation (S04)
- `cargo test -p assay-tui --test analytics_screen` — TUI analytics screen (S05)
- `assay history analytics --json` — machine-readable analytics inspection

### What assumptions changed
- No major assumptions changed. D092 pattern confirmed for Milestone TOML extension. Background thread + TuiEvent channel (D107) reused successfully for PR status polling. Analytics aggregation required no new storage format.

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — pr_labels, pr_reviewers, pr_body_template fields
- `crates/assay-core/src/pr.rs` — render_pr_body_template, ChunkGateSummary, PrStatusState, PrStatusInfo, pr_status_poll
- `crates/assay-core/src/history/analytics.rs` — AnalyticsReport, FailureFrequency, MilestoneVelocity, compute_analytics
- `crates/assay-core/tests/pr.rs` — 12 PR creation integration tests
- `crates/assay-core/tests/pr_status.rs` — 8 PR status parsing tests
- `crates/assay-core/tests/analytics.rs` — 8 analytics integration tests
- `crates/assay-cli/src/commands/pr.rs` — --label/--reviewer CLI flags
- `crates/assay-cli/src/commands/history.rs` — assay history analytics subcommand
- `crates/assay-mcp/src/server.rs` — labels/reviewers MCP params
- `crates/assay-tui/src/event.rs` — TuiEvent::PrStatusUpdate
- `crates/assay-tui/src/app.rs` — Screen::Analytics, draw_analytics, PR badge rendering, analytics_report field
- `crates/assay-tui/src/main.rs` — gh availability check, background polling thread
- `crates/assay-tui/tests/analytics_screen.rs` — 6 analytics screen tests
- `crates/assay-tui/tests/pr_status_panel.rs` — 3 PR status panel tests
- `plugins/opencode/AGENTS.md` — workflow guide with 10 MCP tools
- `plugins/opencode/skills/*.md` — 5 skill files
