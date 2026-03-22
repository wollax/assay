# S02: Provider Dispatch and Harness Wiring

**Goal:** Replace the hardcoded Claude adapter in the `r` key handler with a `provider_harness_writer` free function that dispatches to the correct harness adapter based on `ProviderKind`; extend `Screen::Settings` with editable model text-input fields; prove all three provider paths with unit tests.

**Demo:** After this slice, changing provider to Ollama in the Settings screen and pressing `r` from the Dashboard invokes `ollama run <model>` instead of `claude`; Settings shows editable Planning/Execution/Review model fields pre-populated from config; pressing `w` saves model values alongside provider; unit tests prove correct CLI args per provider without live execution.

## Must-Haves

- `crates/assay-tui/src/agent.rs` exists with `provider_harness_writer(config: &Config) -> Box<HarnessWriter>`, `OllamaConfig`, and `OpenAiConfig` structs
- `assay-tui::agent` is declared in `lib.rs` as `pub mod agent`
- `r` key handler in Dashboard arm calls `provider_harness_writer` instead of the hardcoded claude block; `ProviderKind::Anthropic` path is behaviourally identical to S01
- `provider_dispatch_anthropic_uses_claude_binary`, `provider_dispatch_ollama_uses_ollama_binary`, `provider_dispatch_openai_uses_openai_binary` unit tests in `tests/provider_dispatch.rs` all pass
- `Screen::Settings` has four new fields: `planning_model: String`, `execution_model: String`, `review_model: String`, `model_focus: Option<usize>`
- `s` key handler pre-populates the three model buffers from `app.config.provider`
- Tab cycles focus through the three model fields; char input appends, Backspace pops; Esc/Enter on the last field returns focus to the provider list
- `w` save constructs `ProviderConfig` from in-screen model buffers (empty string → `None`), identical to prior behaviour for fields not shown
- All 35+ existing `assay-tui` tests continue to pass; `just ready` exits 0

## Proof Level

- This slice proves: integration (provider dispatch correctness + Settings state machine + full `just ready` verification)
- Real runtime required: no (provider CLI invocation is UAT-only per roadmap; closures proven by arg-inspection tests)
- Human/UAT required: yes — configure Ollama in Settings → press `r` → verify `ollama` is invoked (out of scope for automated tests)

## Verification

- `cargo test -p assay-tui --test provider_dispatch` → 3/3 pass
- `cargo test -p assay-tui` → all tests pass (≥35 pre-S02 + 3 provider_dispatch + new settings model tests)
- `cargo build -p assay-tui` → binary produced without warnings
- `just ready` → exits 0 (fmt + clippy + test + deny all green)

## Observability / Diagnostics

- Runtime signals: `Screen::AgentRun` status line already shows the binary invoked via `cli_args[0]`; future agents can inspect `app.config.provider.provider` to know which adapter was selected
- Inspection surfaces: `r` key handler is the composition point — readable in `app.rs` Dashboard arm; `agent.rs` module is the single dispatch authority
- Failure visibility: `provider_harness_writer` returns `Box<HarnessWriter>`; the closure returns `Err(String)` on config-write failure; `r` key handler propagates write failure by returning `false` (no-op, same pattern as S01)
- Redaction constraints: model name strings are not secrets; no API keys handled in this slice

## Integration Closure

- Upstream surfaces consumed:
  - `TuiEvent` loop + `Screen::AgentRun` + `App.agent_thread` + `App.event_tx` from S01 (`origin/main`)
  - `App.config: Option<assay_types::Config>` with `ProviderKind` / `ProviderConfig` available at `r`-key spawn time
  - `assay_core::pipeline::HarnessWriter` type alias (`dyn Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String>`)
  - `assay_harness::claude::{generate_config, write_config, build_cli_args}` (Anthropic path)
- New wiring introduced in this slice:
  - `assay-tui::agent` module declared in `lib.rs`; `app.rs` imports `provider_harness_writer` from it
  - `r` key handler swaps hardcoded claude block for 2-line `provider_harness_writer` call
  - `Screen::Settings` gains 4 fields; `s`/`w`/event handlers updated accordingly
- What remains before the milestone is truly usable end-to-end:
  - S03 (slash command overlay): `/gate-check`, `/status`, `/pr-create` commands
  - S04 (MCP panel): `.assay/mcp.json` management
  - Real Ollama invocation is UAT-only; no automated end-to-end agent subprocess for Ollama in this slice

## Tasks

- [x] **T01: Rebase onto origin/main and write failing provider dispatch tests** `est:30m`
  - Why: Gets all S01 deliverables onto this branch; establishes the unambiguous test contract before writing any implementation; tests should compile and fail (or panic-fail) until T02 provides `agent.rs`
  - Files: `crates/assay-tui/tests/provider_dispatch.rs` (new)
  - Do: (1) `git rebase origin/main` from `kata/root/M007/S02`; verify `cargo build -p assay-tui` passes post-rebase. (2) Create `tests/provider_dispatch.rs` with helper `run_writer(config) -> Vec<String>` that calls `provider_harness_writer`, passes a dummy `HarnessProfile` + temp `Path`, and asserts the returned `Vec<String>` starts with the expected binary name. (3) Write three `#[test]` functions: `provider_dispatch_anthropic_uses_claude_binary` (expects `cli_args[0]` to be `"claude"` or start with the claude binary path), `provider_dispatch_ollama_uses_ollama_binary` (expects `"ollama"`), `provider_dispatch_openai_uses_openai_binary` (expects `"openai"`). (4) The tests reference `assay_tui::agent::provider_harness_writer` — they will fail with a compile error until T02 creates the module; confirm the compile error is the only failure.
  - Verify: `git log --oneline -3` shows origin/main commits; `cargo build -p assay-tui` passes; `cargo test -p assay-tui --test provider_dispatch 2>&1` fails with `unresolved import assay_tui::agent` (expected compile error)
  - Done when: branch is rebased, tests file exists with 3 correctly-structured tests, build (excluding the new test) succeeds

- [x] **T02: Create `assay-tui::agent` module and wire `r` key handler** `est:1h`
  - Why: Delivers the core dispatch logic that makes `provider_dispatch` tests pass and replaces the S01 hardcoded claude block with real provider-aware dispatch
  - Files: `crates/assay-tui/src/agent.rs` (new), `crates/assay-tui/src/lib.rs`, `crates/assay-tui/src/app.rs`
  - Do: (1) Create `src/agent.rs`: define `OllamaConfig { model: String }` and `OpenAiConfig { model: String, api_key_env: String }` as TUI-local structs (not pub to the workspace). (2) Implement `pub fn provider_harness_writer(config: Option<&assay_types::Config>) -> Box<assay_core::pipeline::HarnessWriter>` (takes `Option<&Config>` because `Config` does not implement `Default`): determine provider via `config.and_then(|c| c.provider.as_ref()).map(|p| p.provider).unwrap_or(ProviderKind::Anthropic)`; Anthropic closure: `Box::new(move |profile: &HarnessProfile, path: &Path| { let cfg = assay_harness::claude::generate_config(profile); assay_harness::claude::write_config(&cfg, path).map_err(|e| e.to_string())?; Ok(assay_harness::claude::build_cli_args(&cfg)) })`; Ollama closure: capture `model_name: String` (from `config.and_then(…).and_then(|p| p.execution_model.clone()).unwrap_or_else(|| "llama3".into())`), return `vec!["ollama".into(), "run".into(), model_name.clone()]`; OpenAI closure: capture `model_name`, return `vec!["openai".into(), "api".into(), "chat.completions.create".into(), "--model".into(), model_name.clone()]`. (3) Declare `pub mod agent;` in `src/lib.rs`. (4) In `src/app.rs` Dashboard `r` arm: import `use crate::agent::provider_harness_writer;`; replace the hardcoded claude block with: build `HarnessProfile` (same minimal stub), then `let writer = provider_harness_writer(self.config.as_ref()); let cli_args = match writer(&profile, &run_dir) { Ok(args) => args, Err(_) => return false };` — surrounding guard logic preserved verbatim.
  - Verify: `cargo test -p assay-tui --test provider_dispatch` → 3/3 pass; `cargo test -p assay-tui` → all pre-existing tests still pass; `cargo build -p assay-tui` succeeds
  - Done when: `tests/provider_dispatch.rs` 3/3 green; all pre-existing 35 tests pass; no compile warnings in `agent.rs`

- [ ] **T03: Extend Settings screen with model text-input fields and finalize** `est:1.5h`
  - Why: Delivers the user-visible model configuration capability declared in the boundary map; closes the slice by running `just ready`
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/settings.rs`
  - Do: (1) **Add tests first** — append 2 tests to `tests/settings.rs`: `settings_model_fields_prepopulated_from_config` (writes a `config.toml` with `[provider]` section containing `planning_model = "claude-3-haiku"`, opens Settings, asserts `Screen::Settings { planning_model, .. }` equals `"claude-3-haiku"`) and `settings_w_save_includes_model_fields` (opens Settings, types a model name into the active field via char events, saves with `w`, reloads config, asserts `config.provider.planning_model == Some("...")`) — these tests will fail until the variant is extended. (2) **Extend `Screen::Settings`**: add `planning_model: String`, `execution_model: String`, `review_model: String`, `model_focus: Option<usize>` to the enum variant. Fix all match arms that destructure `Settings { selected, error }` by adding `..` — scan for these in `app.rs` and update. (3) **Update `s` key handler** to extract model strings from `app.config`: `planning_model = config.provider.as_ref().and_then(|p| p.planning_model.clone()).unwrap_or_default()`, same for execution/review. Set `model_focus: None`. (4) **Update `draw_settings`** signature to accept the new fields: add a second content section below the provider list showing three labelled text rows ("Planning model:", "Execution model:", "Review model:") with cursor indicator on the focused field (`model_focus == Some(i)` → bold/cyan, others → dim). (5) **Update Settings event handler**: `Tab` when `model_focus.is_none()` → `model_focus = Some(0)`; `Tab` when `model_focus == Some(i)` → `Some(i+1)` or `None` when `i == 2`; `Esc` when `model_focus.is_some()` → `model_focus = None` (does NOT leave Settings); `Char(c)` when `model_focus == Some(0)` → append to `planning_model`, etc.; `Backspace` when model focused → pop last char from the active buffer. Arrow keys continue to cycle providers (unchanged), active only when `model_focus.is_none()`. (6) **Update `w` save handler** to pass model buffers: `planning_model: Some(buf).filter(|s| !s.is_empty())` for each of the three fields — replaces the current `.and_then(|p| p.planning_model.clone())` fallback. (7) Run `cargo test -p assay-tui` (all tests pass), then `just ready`.
  - Verify: `cargo test -p assay-tui` → all tests pass (≥35 + 3 provider_dispatch + 2 new model tests); `just ready` → exit 0
  - Done when: `just ready` exits 0; no regression in any pre-existing test; new model field tests pass

## Files Likely Touched

- `crates/assay-tui/src/agent.rs` (new)
- `crates/assay-tui/src/lib.rs` (add `pub mod agent`)
- `crates/assay-tui/src/app.rs` (Screen::Settings variant, s-key handler, r-key handler, draw_settings, Settings event arm, w-save)
- `crates/assay-tui/tests/provider_dispatch.rs` (new)
- `crates/assay-tui/tests/settings.rs` (append 2 model-field tests)
