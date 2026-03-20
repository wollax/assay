# M006: TUI as Primary Surface — Research

**Researched:** 2026-03-20
**Domain:** Ratatui TUI, Rust terminal applications, data-driven UI
**Confidence:** HIGH

## Summary

The current TUI stub (`crates/assay-tui/src/main.rs`, 42 lines) already uses the correct Ratatui 0.30 primitives: `ratatui::init()`, `ratatui::restore()`, `DefaultTerminal`, the `Frame`/`Layout`/`Constraint` layout system, and crossterm event polling. Ratatui 0.30 modularized into `ratatui-core`, `ratatui-crossterm`, and `ratatui-widgets` subcrates, but the `ratatui` umbrella crate re-exports everything needed. The workspace already has `ratatui = "0.30"` and `crossterm = "0.28"` as workspace dependencies. **No new dependencies are needed for S01–S02; the tokio dep is optional for background loading.** The only missing dep for the wizard slice (S02) is none — wizard logic already lives in `assay-core::wizard`.

The recommended architecture is an `App` struct with an explicit `Screen` enum for navigation (`Dashboard`, `ChunkDetail`, `Wizard`, `Settings`). All domain data reads go through existing `assay-core` functions: `milestone_scan()`, `milestone_load()`, `history::list()` + `history::load()`, `spec::scan()`, `config::load()`. These are all sync, so a data-loading background thread (std::thread, not tokio) or polling-on-focus is the right fit with the sync-core convention (D007). The `Config` type in `assay-types` needs a new `provider` section for R052 — this is a type extension, not a schema break, because `deny_unknown_fields` is on `Config` and new optional fields must use `serde(default, skip_serializing_if)`.

The riskiest part of this milestone is the `Config` extension for R052 (TUI provider configuration). `Config` has `deny_unknown_fields` and a locked schema snapshot — the new `provider: Option<ProviderConfig>` field must go through the same schema snapshot update that every prior Config extension has used (see D056 pattern). The wizard form in S02 is the second-riskiest: collecting text input inside a Ratatui event loop requires careful cursor management and is a common source of UX bugs. Using `tui-textarea` crate avoids most of this but adds a dep — weigh that against a simple line-buffer implementation using `crossterm::event::KeyCode`.

## Recommendation

Build `assay-tui` as a classic `App` struct with a `Screen` enum and a `draw(frame)` + `handle_event(event)` split. Keep data reads synchronous — load everything on focus/navigation, not in a background thread for S01. Add a background thread for gate history loading only if latency is observable during S01 integration testing. Reuse `assay-core` functions directly (no new core APIs needed). For the wizard (S02), implement a minimal multi-step form state machine inside the TUI using a `Vec<String>` input buffer per field and crossterm key events — do not pull in `dialoguer` (it's TTY-blocking, not ratatui-compatible). For provider config (S03), extend `Config` in `assay-types` with `Option<ProviderConfig>` using the D056 pattern, and use a `Select`-style list widget in the TUI for enum choices.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Terminal init / panic hook / raw mode | `ratatui::init()` + `ratatui::restore()` | Already used in the stub; handles alternate screen, raw mode, and panic restore in one call |
| Scrollable list with selection | `ratatui::widgets::List` + `ListState` | StatefulWidget built into ratatui; handles scroll offset and selected index automatically |
| Progress bars | `ratatui::widgets::Gauge` or `ratatui::widgets::LineGauge` | Built in; renders `X/N` chunk progress as a fraction bar without math |
| Block borders / titles | `ratatui::widgets::Block` | Wraps any widget with a border and title in one method chain |
| Multi-column layout | `Layout::horizontal/vertical` with `Constraint` | Already imported in the stub; handles proportional/fixed/fill splits |
| Milestone scan / load | `assay_core::milestone::milestone_scan()`, `milestone_load()` | Tested, atomic-read, returns `Vec<Milestone>` sorted by slug — ready to use |
| Gate history load | `assay_core::history::list()` + `history::load()` | Returns sorted run IDs and full `GateRunRecord` with pass/fail per criterion |
| Spec scan | `assay_core::spec::scan()` | Returns `ScanResult` with all specs including GatesSpec data |
| Config load | `assay_core::config::load(root)` | Handles missing file gracefully (defaults), validates, returns `Config` |
| Wizard pure logic | `assay_core::wizard::create_from_inputs()` | Tested pure function: takes `WizardInputs` → writes milestone TOML + gates.toml files atomically |
| Atomic config write | `NamedTempFile` + `sync_all` + `persist` | Pattern established in `milestone.rs` and `history.rs` — reuse verbatim for config saves |

## Common Pitfalls

### Pitfall 1: `deny_unknown_fields` on `Config` breaks when adding provider fields
**What goes wrong:** Adding `provider: Option<ProviderConfig>` to `Config` and forgetting to update the schema snapshot causes `cargo insta` failures. Worse, existing `.assay/config.toml` files without the field fail to load if the field is not `serde(default)`.
**Why it happens:** `Config` has `#[serde(deny_unknown_fields)]` — unknown keys in the TOML cause parse failure. New fields on the Rust side are fine only when marked `serde(default, skip_serializing_if = "Option::is_none")`.
**How to avoid:** Follow D056 exactly: add `#[serde(default, skip_serializing_if = "Option::is_none")]` on the new field; run `cargo insta review` after adding it; confirm backward compat with a `config_toml_roundtrip_without_provider` test. Check `crates/assay-types/src/lib.rs` (the `Config` struct) before touching it.
**Warning signs:** Existing `config.toml` loads fail with "unknown field" in tests.

### Pitfall 2: `dialoguer` is incompatible with Ratatui's raw mode
**What goes wrong:** `dialoguer::Input::new().interact_text()` blocks the thread in its own raw-mode loop. Running it inside a Ratatui event loop corrupts terminal state (double raw mode enter/exit).
**Why it happens:** Both Ratatui and dialoguer take ownership of terminal raw mode. They cannot be nested.
**How to avoid:** Implement the TUI wizard as a Ratatui form: `WizardState { step: usize, fields: Vec<String>, cursor: usize }`. Each step renders a prompt + input line; `KeyCode::Enter` advances to next step; `KeyCode::Char(c)` appends to the current field buffer. Do NOT call `dialoguer` from within the TUI.
**Warning signs:** TUI goes blank or shows garbage after a dialoguer call from within the event loop.

### Pitfall 3: Blocking on data load inside `terminal.draw()`
**What goes wrong:** Calling `milestone_scan()` or `history::load()` inside the `draw` callback blocks the render thread, causing visible lag or missed keypresses.
**Why it happens:** `terminal.draw()` holds a mutable borrow on the terminal; any blocking I/O inside it starves the event loop.
**How to avoid:** Load data in `handle_event()` on navigation (e.g., when user selects a milestone, load its chunks and history synchronously before the next frame). For S01, synchronous loads on navigation transitions are acceptable — gate history load is O(file count) and fast for typical project sizes. If profiling shows latency, move history loads to a background `std::thread` that sends results via `std::sync::mpsc`.
**Warning signs:** Keypress events pile up; UI stutters during list navigation.

### Pitfall 4: `ListState::selected()` returns `None` on empty lists
**What goes wrong:** Rendering a `List` with `render_stateful_widget` when the list is empty and `ListState` has a stale selection index panics or displays nothing where a placeholder is expected.
**Why it happens:** `ListState` is not automatically bounded to the list length.
**How to avoid:** Guard all selection-dependent logic with `if items.is_empty() { render_empty_placeholder }`. Reset `ListState` selection to `None` when the underlying data changes. Add a dedicated "No milestones found — run `assay plan` to create one" empty-state widget.
**Warning signs:** Panic in release builds on fresh projects with no `.assay/milestones/` directory.

### Pitfall 5: Forgetting to create `.assay/milestones/` directory before `milestone_scan()`
**What goes wrong:** `milestone_scan()` returns `Ok(vec![])` when the directory doesn't exist — that's correct behavior. But if the TUI also calls `config::load()` and the user hasn't run `assay init`, `config.toml` is missing and `load()` returns an error.
**Why it happens:** The TUI launches from any directory; the project may not be initialized.
**How to avoid:** In `main.rs`, detect if `.assay/` exists. If not, show a "Not an Assay project — run `assay init`" splash and exit cleanly (or offer init from TUI in S01). Do not panic on missing config.

### Pitfall 6: `assay-tui` missing `tokio` when needed for background threads
**What goes wrong:** If background loading is added, tokio's `spawn_blocking` is the natural tool — but `assay-tui/Cargo.toml` doesn't have tokio. Adding it requires enabling the right features.
**Why it happens:** The current TUI stub is pure sync. Tokio is available in the workspace but not declared as a dep for assay-tui.
**How to avoid:** For S01, use `std::thread::spawn` + `std::sync::mpsc` if background loading is needed. Avoids an async runtime in the TUI binary. Consistent with D007 (sync core). Add tokio only if S02+ wizard async needs it.

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — Current 42-line stub. Replace entirely; preserve the `ratatui::init()` / `ratatui::restore()` / panic hook pattern exactly. The entry point binary name is `assay` (same as CLI) — check binary name conflicts in workspace; they may need separate binary names or feature flags.
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir)`, `milestone_load(assay_dir, slug)`, `milestone_save(assay_dir, milestone)`. Primary data source for the dashboard. Returns `Vec<Milestone>` sorted by slug.
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status(assay_dir)` returns `Option<CycleStatus>` with active milestone slug, phase, active chunk slug, progress counts. Useful for dashboard header badge.
- `crates/assay-core/src/history/mod.rs` — `list(assay_dir, spec_name)` returns sorted run IDs; `load(assay_dir, spec_name, run_id)` returns `GateRunRecord` with full criterion results. Use `list()` to find the latest run ID, then `load()` to get pass/fail details.
- `crates/assay-core/src/spec/mod.rs` — `scan(specs_dir)` returns `ScanResult` with all `SpecEntry` variants. For chunk detail, use `load_spec_entry_with_diagnostics(slug, specs_dir)` to get `GatesSpec` with criteria.
- `crates/assay-core/src/wizard.rs` — `create_from_inputs(WizardInputs, assay_dir, specs_dir)` — the pure function the TUI wizard should call after collecting all user inputs in its own form state machine. Produces `WizardResult { milestone_path, spec_paths }`. `WizardChunkInput { slug, name, criteria: Vec<String> }` and `WizardInputs { slug, name, description, chunks }` are the public input types.
- `crates/assay-core/src/config/mod.rs` — `load(root: &Path) -> Result<Config>` where `root` is the project root (parent of `.assay/`). Returns `Config` with all optional sections defaulted.
- `crates/assay-types/src/lib.rs` — `Config` struct (lines 173+): `project_name`, `specs_dir`, `gates: Option<GatesConfig>`, `guard: Option<GuardConfig>`, `worktree: Option<WorktreeConfig>`, `sessions: Option<SessionsConfig>`. No `provider` field yet — S03 adds it.
- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` types. `MilestoneStatus` is `Draft | InProgress | Verify | Complete`. `Milestone.completed_chunks: Vec<String>` tracks done chunks. `Milestone.chunks: Vec<ChunkRef>` with `{ slug, order }`.
- `crates/assay-types/src/gate_run.rs` — `GateRunRecord` (full run result), `GateRunSummary` (aggregated), `CriterionResult { criterion_name, result: Option<GateResult>, enforcement }`. `GateResult` is pass/fail/skip.
- `crates/assay-cli/src/commands/plan.rs` — Shows how `WizardInputs` is assembled from dialoguer prompts. The TUI wizard needs to replicate this flow in a Ratatui form, not call dialoguer.

### Binary name conflict risk
The workspace has `assay-cli` with `[[bin]] name = "assay"` and `assay-tui` — check if the TUI also declares a `"assay"` binary. Two workspace crates cannot both produce a binary named `assay`. The TUI should use `"assay-tui"` as the binary name, matching the M006 context description ("entry point: `assay` TUI binary or `assay-tui`"). This is a naming decision for S01 planning — the context says "Launch `assay`" which suggests the CLI and TUI may share the `assay` binary via a subcommand dispatch (e.g., `assay` with no subcommand launches TUI), or TUI uses `assay-tui`. **Check this before S01 planning.**

## Constraints

- `assay-core` is sync (D007). All milestone/history/spec/config reads are `std::fs` — no async. TUI must not introduce an async event loop unless it wraps the sync reads in `spawn_blocking` via tokio, or uses `std::thread`.
- `Config` has `deny_unknown_fields`. Any new field for R052 must be `Option<T>` + `serde(default, skip_serializing_if)` + schema snapshot update (D056 pattern).
- Zero traits (D001). `App` struct methods, free render functions, no `Widget` trait impls unless genuinely needed. Use `impl Widget for MyWidget` only for self-contained pure display types with no app state.
- The `assay-tui` crate does not have tokio in deps. Add it only if needed; prefer `std::thread` for background loads.
- `assay-cli/Cargo.toml` declares `[[bin]] name = "assay"`. The TUI must use a different binary name unless the CLI is refactored to launch TUI by default with no subcommand. This is a scope decision for S01.
- All writes (config save, wizard output) must use atomic tempfile-rename pattern. `wizard::create_from_inputs()` already does this. Config save in S03 must replicate the pattern from `milestone_save()`.

## Open Risks

- **Binary name collision**: If the TUI wants to be invoked as `assay`, the CLI's `[[bin]] name = "assay"` conflicts. Resolution options: (a) TUI uses `assay-tui` binary name; (b) The CLI's `main.rs` detects no subcommand and launches TUI. This must be decided in S01 planning before coding starts — it affects how the binary is built and installed.
- **Config schema extension latency**: The `Config` type has `deny_unknown_fields` and at least one `cargo insta` snapshot. Any provider config change requires a schema snapshot update and backward-compat tests. Plan one full task for Config extension in S03.
- **Wizard form UX complexity**: A multi-step form inside a Ratatui event loop (with cursor positioning, backspace, field validation, back-navigation) is easily 200–400 lines of state machine code. Underestimating this is a common S02 failure mode. Size tasks generously.
- **Empty project state**: TUI launched on a project without `.assay/` or with no milestones needs a clean "no data" path for every screen. Plan explicit empty-state rendering for S01.
- **`tui-textarea` crate**: The crate `tui-textarea` provides a production-grade text input widget for Ratatui (>1M downloads, actively maintained). It would substantially reduce S02 wizard form complexity. The tradeoff is one new dependency. Worth evaluating in S02 planning. Install count and license (MIT) are favorable.

## Candidate Requirements (Advisory)

These are behaviors commonly expected in a TUI of this type that are not currently captured in R049–R052. Surface for user consideration — not auto-binding:

- **R-TUI-RESIZE**: The TUI should handle terminal resize events (`Event::Resize`) and redraw without panic. Ratatui handles this automatically on the next frame if the event loop continues, but explicit resize handling (clear + redraw) is the conventional pattern.
- **R-TUI-HELP**: A `?` key shows a help overlay with all keybindings. This is table-stakes for any TUI that introduces non-obvious navigation.
- **R-TUI-STATUS-BAR**: A persistent bottom status bar showing project name, active milestone, and key hints (e.g., `q: quit | ?: help | ↑↓: navigate`) is standard in TUI applications of this class. Not required but expected by users.
- **R-TUI-INIT-GUARD**: If `.assay/` doesn't exist, the TUI shows a helpful "run `assay init` first" message instead of panicking or showing an empty list.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (searched) | No dedicated skill found via `npx skills find` — network unavailable |

## Sources

- Ratatui 0.30 source: `~/.cargo/registry/src/index.crates.io-*/ratatui-0.30.0/src/init.rs` — init/restore API, DefaultTerminal type, App struct pattern (HIGH confidence)
- Ratatui widgets source: `~/.cargo/registry/src/index.crates.io-*/ratatui-widgets-0.3.0/src/list.rs` — List + ListState stateful widget API (HIGH confidence)
- Codebase: `crates/assay-core/src/milestone/mod.rs` — milestone_scan/load/save public API surface (HIGH confidence)
- Codebase: `crates/assay-core/src/history/mod.rs` — history list/load API (HIGH confidence)
- Codebase: `crates/assay-core/src/wizard.rs` — WizardInputs, WizardChunkInput, create_from_inputs (HIGH confidence)
- Codebase: `crates/assay-types/src/lib.rs` — Config struct layout, existing fields (HIGH confidence)
- Codebase: `crates/assay-cli/src/commands/plan.rs` — dialoguer-based wizard flow (reference for TUI wizard state machine) (HIGH confidence)
