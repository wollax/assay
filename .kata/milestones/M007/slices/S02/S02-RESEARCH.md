# S02: Provider Dispatch and Harness Wiring — Research

**Date:** 2026-03-21
**Domain:** TUI provider abstraction + harness adapter dispatch + Settings screen extension
**Confidence:** HIGH

## Summary

S02 is a medium-complexity wiring slice that builds the `assay-tui::agent` module for provider dispatch, replaces the S01 hardcoded Claude invocation in the `r` key handler, and extends the Settings screen with editable model input fields. The core abstraction is `provider_harness_writer(config: &Config) -> Box<HarnessWriter>` — a free function that matches on `ProviderKind` and returns a closure compatible with the existing `HarnessWriter` type alias in `assay-core::pipeline`.

All the machinery this slice needs is already present: `HarnessWriter` type alias, `ProviderKind`/`ProviderConfig` in `assay-types`, the existing `assay-harness::claude` adapter, and the `App.config` field loaded at startup. S01 planted a `// S02 replaces this` comment in the `r` key handler pointing exactly at the code to replace.

**Critical prerequisite**: The current branch (`kata/root/M007/S02`) was cut before S01's PR #168 merged to `origin/main`. The branch must be rebased onto `origin/main` before any S02 implementation work begins — all S01 deliverables (TuiEvent, AgentRunStatus, Screen::AgentRun, App.event_tx, App.agent_thread, launch_agent_streaming, channel-based run()) live in that commit.

## Recommendation

1. **Rebase from origin/main first.** `git rebase origin/main` from this branch — this is a required prerequisite, not optional.
2. **Create `crates/assay-tui/src/agent.rs`** with `provider_harness_writer` + `OllamaConfig` + `OpenAiConfig`.
3. **Replace the r-key handler** in `app.rs` (Dashboard arm) — swap the hardcoded claude block for `provider_harness_writer`.
4. **Extend Screen::Settings** with model text-input state (3 String buffers + a `model_focus: Option<usize>` that controls which field is active for char input).
5. **Write unit tests** in `tests/provider_dispatch.rs` proving correct binary per provider kind.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Claude harness config generation + CLI args | `assay_harness::claude::{generate_config, write_config, build_cli_args}` | Already tested, snapshot-locked; just call from inside the provider_harness_writer closure for Anthropic path |
| HarnessWriter type contract | `assay_core::pipeline::HarnessWriter` type alias — `dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>` | Both the r-key handler and run_session() consume this exact type; Box<HarnessWriter> is the unit of composition |
| Provider enum + config struct | `assay_types::{ProviderKind, ProviderConfig}` already in Cargo.toml deps | D092 established these; D109 says use them directly |
| Model single-line text input pattern | `assay_tui::wizard::WizardState` cursor/buffer pattern | Wizard shows how to handle char input, backspace, and cursor in the existing event model |
| Config atomic write | `assay_core::config::save` (D093) | Already called by the Settings `w` handler; new model fields use the same save path |

## Existing Code and Patterns

- `crates/assay-tui/src/app.rs:355–430` (on origin/main) — The `r` key handler block with the comment `// S02 replaces this with real provider dispatch from app.config`. This is the exact code to replace. The surrounding guard logic (agent_thread, event_tx, project_root, cycle_status) must be preserved verbatim.

- `crates/assay-harness/src/claude.rs` — `generate_config(profile) -> ClaudeConfig`, `write_config(cfg, path) -> io::Result<()>`, `build_cli_args(cfg) -> Vec<String>`. The Anthropic path in provider_harness_writer wraps these three calls: generate → write → build_args, returning the args.

- `crates/assay-core/src/pipeline.rs:376` — `pub type HarnessWriter = dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>`. The Ollama and OpenAI closures must match this signature: take `&HarnessProfile` + `&Path`, return `Result<Vec<String>, String>`. For providers without config files, the path parameter is unused and the write step is a no-op.

- `crates/assay-types/src/lib.rs` — `ProviderKind { Anthropic, OpenAi, Ollama }`, `ProviderConfig { provider, planning_model, execution_model, review_model }`, `Config { provider: Option<ProviderConfig>, ... }`. The dispatch function reads `config.provider.as_ref().map(|p| p.provider).unwrap_or(ProviderKind::Anthropic)` and `config.provider.as_ref().and_then(|p| p.execution_model.clone())` for the model arg.

- `crates/assay-tui/src/app.rs Screen::Settings` (origin/main) — Currently `Settings { selected: usize, error: Option<String> }`. Draw function `draw_settings(frame, area, config, selected, error)` renders a full-screen block with 3 provider ListItems. The `w` save handler constructs `ProviderConfig` preserving existing `planning_model/execution_model/review_model` from `self.config`. S02 adds `model_focus: Option<usize>`, `planning_model: String`, `execution_model: String`, `review_model: String` to the variant, and the `w` handler uses the in-screen buffers instead of reading from `self.config`.

- `crates/assay-tui/tests/settings.rs` — 5 existing tests that check `selected` provider cycling and `w`-key saves. These must pass after the Screen::Settings variant is extended; `..` patterns in tests cover the added fields.

- `crates/assay-tui/src/wizard.rs` — Reference for cursor-based char input. `WizardState.cursor` tracks append position; `KeyCode::Char(c)` appends, `Backspace` pops. Same pattern applies to model text fields (simpler: append-only, no mid-line editing needed for S02).

## Constraints

- **D001 (no traits)**: `provider_harness_writer` returns `Box<HarnessWriter>` (a `Box<dyn Fn>`) — no dispatch enum, no trait object beyond the existing alias. All three provider paths are plain closures.
- **D109**: Provider dispatch via free function in `assay-tui::agent`, no new trait hierarchy. `OllamaConfig` and `OpenAiConfig` are TUI-local structs, not persisted to assay-types.
- **D007 (sync core)**: Everything in this slice is synchronous. No tokio, no async.
- **Borrow checker constraint on Screen::Settings match** (D097/D098): When reading `selected` to determine provider in the `w` handler, use the `let (selected, ..) = if let Screen::Settings { selected, .. } = self.screen` pattern already established. Adding more fields to the variant does not change this pattern.
- **Config.deny_unknown_fields**: `ProviderConfig` has `deny_unknown_fields`. `planning_model/execution_model/review_model` fields already exist in the type — S02 just wires the TUI buffers to pass them through on save. No schema changes needed.
- **`Screen::Settings` extension ripple**: Every arm that matches `Screen::Settings { selected, error }` must either add the new fields or use `..`. Pattern `Screen::Settings { selected, error, .. }` is the minimal non-breaking change.
- **`app.config` may be None**: The `r` key handler must handle `config.is_none()` gracefully. Decision: fall back to `ProviderKind::Anthropic` default when config is None (same behavior as S01's hardcoded claude path).

## Common Pitfalls

- **Closure capture of owned String**: `Box<dyn Fn>` closures must capture owned `String` values (model name) via `move`. Pattern: `let model_name = model.unwrap_or_else(|| "llama3".into()); Box::new(move |..| { ... model_name.clone() ... })`. If you capture by reference, the compiler will reject it — close must be `'static`.

- **HarnessProfile.clone() inside the closure**: The Anthropic closure calls `assay_harness::claude::generate_config(profile)` where `profile: &HarnessProfile`. If you need to mutate model, clone the profile inside the closure body: `let mut p = profile.clone(); p.settings.model = Some(model.clone());`. Don't try to pass `mut profile` — the signature is `&HarnessProfile`.

- **Forgetting to rebase before implementing**: Working on S02 without the S01 codebase means `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, and `App.agent_thread` do not exist. Everything compiles on the old code but the additions won't integrate. Rebase is step 0.

- **Screen::Settings variant field count in existing tests**: Tests in `settings.rs` do `if let Screen::Settings { selected, .. } = &app.screen` — the `..` already absorbs unknown fields, so adding new fields to the variant won't break these tests. BUT the `draw_settings` function signature currently takes individual parameters; extending it requires adding the new params everywhere it's called. Changing the draw function signature is simpler than threading optional state — accept `model_focus`, `planning_model`, `execution_model`, `review_model` directly or pass a `SettingsState` reference.

- **Model focus navigation vs provider navigation**: The Settings screen has two interactive sections (provider list and model fields). Using the same arrow keys for both requires a mode or focus tracker. Simplest correct approach: arrow keys always cycle providers (unchanged behavior); pressing `Tab` or `Enter` on the provider list section activates the model fields section (sets `model_focus = Some(0)`); further `Tab`/`↓` cycles focus between Planning/Execution/Review; `Enter` or `Tab` on last model field returns focus to provider list. This keeps arrow navigation unambiguous.

- **`write_config` error type**: `assay_harness::claude::write_config` returns `io::Result<()>`. The HarnessWriter closure must return `Result<Vec<String>, String>`, so map the io error: `.map_err(|e| e.to_string())?`.

- **Ollama and OpenAI adapters have no config files to write**: The `path: &Path` parameter is unused. That's fine — Rust allows unused parameters with `_path: &Path` naming. The write step is just omitted. The tests verify only the returned `Vec<String>` CLI args.

## Open Risks

- **OpenAI CLI invocation format**: There is no universal "openai" binary. The test `provider_dispatch_openai_uses_openai_binary` just verifies the first arg is `"openai"`. The actual OpenAI provider path is a stub in S02 — real invocation is UAT-only per the roadmap ("OpenAI uses a new minimal adapter"). Keep it simple: return `["openai", "api", "chat.completions.create", "--model", model]`. The binary name is the test contract; the exact subcommands can be refined later.

- **Model field content during Settings → `w` save**: When the user opens Settings, the model buffers should be pre-populated from `app.config.provider.{planning,execution,review}_model`. This means the Screen::Settings transition code (the `s` key handler) must extract those strings from config. Empty string → `None` on save (same as ProviderConfig's Option<String> semantics).

- **S01 relay-wrapper thread code duplication**: S01's `r` handler is ~40 lines. S02 replaces about 8 of those lines (the harness config block) with a 2-line call to `provider_harness_writer`. The rest of the relay-wrapper thread logic is unchanged. Be surgical — only replace the harness block, not the entire handler.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | (none found) | none found — existing codebase patterns are sufficient |

## Sources

- S01-SUMMARY.md on origin/main — exact API contracts delivered by S01 (what the `r` key handler produces, App field names, test count); used `git show origin/main:...` to read
- `crates/assay-tui/src/app.rs` on origin/main — confirmed current `Screen::Settings` shape, r-key handler structure, and the `// S02 replaces this` comment
- `crates/assay-harness/src/claude.rs` — confirmed function signatures for generate_config, write_config, build_cli_args
- `crates/assay-core/src/pipeline.rs:376` — confirmed HarnessWriter type alias signature
- `crates/assay-types/src/lib.rs` — confirmed ProviderKind, ProviderConfig field names and Option<String> semantics
- `crates/assay-tui/tests/settings.rs` — confirmed existing test patterns use `..` absorbing new fields
- `crates/assay-tui/src/wizard.rs` — reference for cursor/char-input pattern
- `.kata/DECISIONS.md` — D001 (no traits), D007 (sync), D092 (ProviderConfig), D093 (config_save), D109 (provider_harness_writer)
- M007 roadmap boundary map S01→S02 and S02→S03 sections
