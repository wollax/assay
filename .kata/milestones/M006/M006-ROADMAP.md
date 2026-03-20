# M006: TUI as Primary Surface

**Vision:** Replace the 42-line `assay-tui` stub with a full Ratatui application. When complete, a developer can launch `assay-tui`, see a real project dashboard with milestone status, chunk progress, and gate results, navigate into any chunk to read its criteria and latest gate run, create a new milestone through an in-TUI wizard, and configure their AI provider — all without touching the CLI or editing TOML files.

## Success Criteria

- `assay-tui` launches on any project with `.assay/` and shows a dashboard: milestones listed with status badges (Draft/InProgress/Verify/Complete), chunk progress fractions (e.g. `2/4`), and gate pass/fail counts per milestone — data loaded from `assay-core`, not hardcoded
- Navigating into a milestone shows its chunks; navigating into a chunk shows its criteria, descriptions, and latest gate run result (pass/fail/pending) per criterion
- Pressing `n` from the dashboard launches the in-TUI authoring wizard; completing it produces a real milestone TOML + chunk specs in `.assay/` via `assay_core::wizard::create_from_inputs`; the new milestone appears in the dashboard immediately without restarting
- Pressing `s` from the dashboard opens a settings screen where the user can select AI provider and model; the choice persists to `.assay/config.toml` and survives a TUI restart
- Launching `assay-tui` on a project with no `.assay/` directory shows a clear "Not an Assay project — run `assay init` first" message and exits cleanly (no panic)
- `just ready` passes; `cargo build -p assay-tui` succeeds; the binary name is `assay-tui` (not `assay`, which is already claimed by assay-cli)

## Key Risks / Unknowns

- **Wizard form state machine** — multi-step text input inside a Ratatui event loop (multi-field, backspace, field validation, step navigation) is 200–400 lines of careful state management and a common underestimated failure mode
- **`deny_unknown_fields` on `Config`** — adding `provider: Option<ProviderConfig>` requires the full D056 pattern (serde default + skip_serializing_if + schema snapshot update + backward-compat test); any omission causes silent breakage of existing `config.toml` files
- **Binary name conflict** — `assay-cli` already declares `[[bin]] name = "assay"`; `assay-tui` has no `[[bin]]` declaration; the default crate name produces no output binary; must add `[[bin]] name = "assay-tui"` before any other TUI work

## Proof Strategy

- **Binary name conflict** → retired in S01: `cargo build -p assay-tui` produces a `target/debug/assay-tui` binary; `cargo build -p assay-cli` still produces `target/debug/assay`
- **Wizard form state machine** → retired in S02: wizard completes a round-trip — user input → `WizardState` steps → `create_from_inputs` → milestone file written to tempdir — proven by an integration test, not just unit tests of individual steps
- **`deny_unknown_fields` on `Config`** → retired in S04: a round-trip test proves `config.toml` without the provider section loads without error; a round-trip test proves a config with provider section serializes and deserializes correctly; schema snapshot locked

## Verification Classes

- Contract verification: unit tests for `App` state transitions; integration test for wizard round-trip; round-trip tests for `ProviderConfig` TOML + schema snapshot; `cargo test --workspace`
- Integration verification: `assay-tui` launched on a real `.assay/` project (e.g. assay's own dev fixtures from M005 test helpers) shows correct milestone/chunk/gate data
- Operational verification: `cargo build -p assay-tui` produces `assay-tui` binary; `just ready` passes; no panic on missing `.assay/`
- UAT / human verification: interactive keyboard navigation through dashboard → milestone → chunk → back; wizard creates milestone visible in dashboard; provider config persists across restart

## Milestone Definition of Done

This milestone is complete only when all are true:

- All five slices are complete with their tests passing
- `just ready` passes (fmt, lint, test, deny)
- `cargo build -p assay-tui` produces `target/debug/assay-tui`; `cargo build -p assay-cli` produces `target/debug/assay`; no binary name collision
- `assay-tui` launched on a project with milestones from M005's test fixtures shows correct dashboard data
- The in-TUI wizard creates a real milestone + chunk spec files that `milestone_scan` returns
- Provider config round-trips through `.assay/config.toml`; existing config.toml without provider section still loads without error
- No panic on missing `.assay/` directory

## Requirement Coverage

- Covers: R049 (TUI project dashboard — S01), R050 (TUI interactive wizard — S02), R051 (TUI spec browser — S03), R052 (TUI provider configuration — S04)
- Partially covers: none
- Leaves for later: R053 (TUI agent spawning — M007), R054 (provider abstraction — M007), R055 (TUI MCP server management — M007), R056 (TUI slash commands — M007)
- Orphan risks: none — all four Active M006 requirements are mapped to slices

## Slices

- [ ] **S01: App Scaffold, Dashboard, and Binary Fix** `risk:high` `depends:[]`
  > After this: `cargo build -p assay-tui` produces a real `assay-tui` binary; launching it on any project shows a live dashboard with milestones (name, status badge, chunk progress fraction) loaded from `assay-core::milestone_scan`; arrow keys navigate the list; `q` quits; no panic on missing `.assay/`

- [ ] **S02: In-TUI Authoring Wizard** `risk:high` `depends:[S01]`
  > After this: pressing `n` from the dashboard opens a multi-step form (milestone name → chunk slugs/names → criteria per chunk); completing it calls `create_from_inputs` and the new milestone appears in the dashboard list immediately; proven by an integration test that writes real files to a tempdir

- [ ] **S03: Chunk Detail View and Spec Browser** `risk:medium` `depends:[S01]`
  > After this: pressing Enter on a milestone shows its chunk list; pressing Enter on a chunk shows a detail pane with criteria descriptions and latest gate run result (pass/fail/pending) per criterion loaded from `assay-core::history`; breadcrumb nav and Esc return to parent screens

- [ ] **S04: Provider Configuration Screen** `risk:medium` `depends:[S01]`
  > After this: pressing `s` opens a settings screen listing AI providers (Anthropic, OpenAI, Ollama) and model-per-phase fields; selecting and saving persists to `.assay/config.toml` via atomic write; existing configs without provider section load without error; schema snapshot locked

- [ ] **S05: Help Overlay, Status Bar, and Integration Polish** `risk:low` `depends:[S01,S02,S03,S04]`
  > After this: `?` shows a full keybindings help overlay; a persistent bottom bar shows project name, active milestone slug, and key hints; `just ready` passes; TUI handles terminal resize without panic; the full flow (dashboard → chunk detail → wizard → provider config) works end-to-end on a real `.assay/` project

## Boundary Map

### S01 → S02, S03, S04, S05

Produces:
- `crates/assay-tui/src/main.rs` — replaced; binary entry point preserved; `[[bin]] name = "assay-tui"` in Cargo.toml
- `App` struct: `screen: Screen`, `milestones: Vec<Milestone>`, `list_state: ListState`, `project_root: Option<PathBuf>`, `config: Option<Config>`
- `Screen` enum: `Dashboard`, `MilestoneDetail`, `ChunkDetail`, `Wizard`, `Settings`, `NoProject`
- `run(terminal)` → `draw(frame)` + `handle_event(event) -> bool` split
- Dashboard screen fully rendered with real data from `milestone_scan` + `config::load`
- Empty-state rendering for no-project and no-milestones cases
- Navigation: `↑↓` moves list selection, `Enter` transitions screen, `Esc` goes back, `q` quits

Consumes: nothing (first slice)

### S02 → S05

Produces:
- `WizardState` struct: `step: usize`, `fields: Vec<Vec<String>>` (one `Vec<String>` per step; steps = milestone-name, chunk-slugs, chunk-names, criteria-per-chunk), `cursor: usize`, `chunk_count: usize`
- `draw_wizard(frame, state)` free function
- `handle_wizard_event(state, event) -> WizardAction` where `WizardAction` is `Continue | Submit(WizardInputs) | Cancel`
- Integration test: `wizard_round_trip` — fills all steps via synthetic `KeyEvent`s → `create_from_inputs` → asserts milestone TOML and chunk spec files written to tempdir

Consumes from S01:
- `Screen::Wizard` variant already registered in `App.screen`
- `App.project_root` to pass as `assay_dir` to `create_from_inputs`

### S03 → S05

Produces:
- `Screen::MilestoneDetail { slug: String, list_state: ListState }` variant (chunks list)
- `Screen::ChunkDetail { milestone_slug: String, chunk_slug: String }` variant (criteria + gate results)
- `draw_milestone_detail(frame, milestone, list_state)` free function
- `draw_chunk_detail(frame, spec, latest_run: Option<GateRunRecord>)` free function
- Data loading on navigation: `milestone_load(assay_dir, slug)` when entering milestone; `spec::load_spec_entry_with_diagnostics` + `history::list` + `history::load` when entering chunk

Consumes from S01:
- `Screen` enum (adds two new variants)
- `App.project_root` for all data reads
- Navigation event dispatch in `handle_event`

### S04 → S05

Produces:
- `ProviderKind` enum in `assay-types`: `Anthropic | OpenAI | Ollama`
- `ProviderConfig` struct in `assay-types`: `provider: ProviderKind`, `planning_model: Option<String>`, `execution_model: Option<String>`, `review_model: Option<String>` — all with `serde(default, skip_serializing_if)`
- `Config.provider: Option<ProviderConfig>` field with `serde(default, skip_serializing_if = "Option::is_none")`
- Schema snapshot for `Config` updated and locked; backward-compat test `config_toml_roundtrip_without_provider` proves existing files still load
- `config_save(root, config) -> Result<()>` free function in `assay-core::config` using NamedTempFile pattern
- `Screen::Settings { provider: ProviderKind, list_state: ListState }` variant
- `draw_settings(frame, config, list_state)` free function

Consumes from S01:
- `App.config: Option<Config>` already loaded in S01
- `App.project_root` for config save path
- `Screen::Settings` variant slot in `Screen` enum

### S05 → (milestone complete)

Produces:
- Help overlay widget: `draw_help_overlay(frame)` rendering a centered `Block` with all keybindings as a `Table`
- `App.show_help: bool` toggle on `?`
- Status bar: bottom 1-line area with project name + active milestone slug (from `cycle_status` on load) + key hints
- Terminal resize: `Event::Resize` forwarded to next `terminal.draw()` call without special handling (ratatui automatic) — explicit clear on resize added if tests show artifacts
- Final `just ready` pass: all fmt/lint/test/deny checks green
