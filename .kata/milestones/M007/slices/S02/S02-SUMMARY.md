---
id: S02
parent: M007
milestone: M007
provides:
  - "`crates/assay-tui/src/agent.rs` — `provider_harness_writer(Option<&Config>) -> Box<HarnessWriter>` free function dispatching Anthropic/Ollama/OpenAI provider paths"
  - "TUI-local `OllamaConfig { model: String }` and `OpenAiConfig { model: String, api_key_env: String }` structs in `agent.rs`"
  - "`pub mod agent` declared in `lib.rs`; `r` key handler in `app.rs` routes through `provider_harness_writer` instead of hardcoded claude calls"
  - "`Screen::Settings` extended with `planning_model`, `execution_model`, `review_model: String` and `model_focus: Option<usize>` fields"
  - "`s` key handler pre-populates model buffers from `app.config.provider`; `draw_settings` renders labelled model input rows with cyan/bold focus"
  - "Tab/Char/Backspace/Esc model-focus state machine; `w` always saves model buffers regardless of focus state"
  - "40 assay-tui tests pass (3 new provider_dispatch + 2 new model-field settings + 35 pre-S02); `just ready` exits 0"
requires:
  - slice: S01
    provides: "`TuiEvent` loop + `Screen::AgentRun` + `App.agent_thread` + relay-wrapper thread pattern; `r` key handler stub with `S02 replaces this` comment; `App.config: Option<Config>` with `ProviderKind`"
affects:
  - S03
key_files:
  - crates/assay-tui/src/agent.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/provider_dispatch.rs
  - crates/assay-tui/tests/settings.rs
key_decisions:
  - "D115 — Anthropic closure prepends `\"claude\"` before `build_cli_args` flags; `build_cli_args` is a pure flag builder, not a full invocation builder"
  - "D116 — `w` save in Settings falls through to the save arm even when model_focus is Some; `'w'` is not appended to the model buffer"
  - "D117 — Tab cycle in model section is linear (0→1→2→None), not wrap-around; final Tab returns focus to provider list"
patterns_established:
  - "`agent.rs` is the single provider dispatch authority; future providers add a new `ProviderKind` arm there"
  - "Model-focus guard at top of Settings arm: read `model_focus` first, early-return `false` after model-section key handling to skip provider-list navigation"
  - "Borrow-safe `w` save: extract `(selected, pm_buf, em_buf, rm_buf)` by value from `Screen::Settings` before taking mutable borrows on `self.project_root` / `self.config`"
observability_surfaces:
  - "`cargo test -p assay-tui --test provider_dispatch` — primary diagnostic; compile error = `agent` module absent; test failures = wrong dispatch args"
  - "`cli_args[0]` in the relay-wrapper thread shows configured provider binary (claude/ollama/openai) visible in `Screen::AgentRun` output"
  - "`assay_core::config::load(root)` reads back persisted model values after `w` save"
drill_down_paths:
  - .kata/milestones/M007/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M007/slices/S02/tasks/T03-SUMMARY.md
duration: 1h 30min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S02: Provider Dispatch and Harness Wiring

**`provider_harness_writer` free function dispatches Anthropic/Ollama/OpenAI agent invocations; Settings screen gains editable per-phase model fields pre-populated from config; `just ready` exits 0, all 40 assay-tui tests pass.**

## What Happened

Three tasks executed in order over approximately 90 minutes.

**T01** (rebase + failing tests): The `kata/root/M007/S02` branch was rebased onto `origin/main`, absorbing the S01 squash commit (`d97a1be`). Three conflicts in kata artifact files were resolved by keeping HEAD (S01 deliverables). `crates/assay-tui/tests/provider_dispatch.rs` was written with three tests referencing `assay_tui::agent::provider_harness_writer` — producing the intended single compile error `E0432: unresolved import assay_tui::agent`.

**T02** (agent module): `src/agent.rs` was created with `provider_harness_writer(Option<&Config>) -> Box<HarnessWriter>` dispatching on `ProviderKind`. The Anthropic arm calls the claude harness functions and prepends `"claude"` as `args[0]` before the flag output from `build_cli_args` (deviation from plan: `build_cli_args` returns only flags, not the binary name). Ollama captures `model` (default `"llama3"`) and returns `["ollama", "run", model]`. OpenAI captures `model` (default `"gpt-4o"`) and returns `["openai", "api", "chat.completions.create", "--model", model]`. `pub mod agent` declared in `lib.rs`; `app.rs` `r` key handler swapped from hardcoded claude block to `provider_harness_writer` call. 3/3 provider_dispatch tests passed; all 38 tests passed.

**T03** (model fields + finalize): Two failing tests written first anchoring the model-field contract. `Screen::Settings` extended with four new fields. `s` key handler updated to extract model strings from config. `draw_settings` gained three new parameters with a model section below the provider list. Settings event handler restructured with a model-focus guard block intercepting Tab/Esc/Char/Backspace when `model_focus.is_some()`. One notable implementation choice: `Char('w')` in the model section falls through to the global save arm (not appended to the buffer) — required by the test contract. `w` save handler extracts model buffers by value before taking mutable borrows. `#[allow(clippy::too_many_arguments)]` applied to `draw_settings` (9 args). `just ready` exited 0.

## Verification

- `cargo test -p assay-tui --test provider_dispatch` → 3/3 pass (anthropic, ollama, openai) ✓
- `cargo test -p assay-tui` → 40/40 pass (8 agent_run + 1 app_wizard + 6 help_status + 3 provider_dispatch + 7 settings + 6 spec_browser + 9 wizard_round_trip) ✓
- `cargo build -p assay-tui` → binary produced, zero warnings ✓
- `just ready` → exit 0 (fmt + clippy + test + deny all green) ✓

## Requirements Advanced

- R054 (Provider abstraction) — S02 adds Ollama and OpenAI dispatch paths via `provider_harness_writer`; all three provider kinds tested with correct CLI args

## Requirements Validated

- R054 (Provider abstraction) — unit tests prove correct CLI args per provider (`provider_dispatch_anthropic_uses_claude_binary`, `provider_dispatch_ollama_uses_ollama_binary`, `provider_dispatch_openai_uses_openai_binary` all pass); Settings model fields persist correctly per `settings_model_fields_prepopulated_from_config` and `settings_w_save_includes_model_fields`; real Ollama/OpenAI invocation remains UAT-only per roadmap

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- **Anthropic closure prepends binary name**: `build_cli_args` returns only flags (`--print`, `--output-format`, ...) not the binary name. The task plan described `Ok(assay_harness::claude::build_cli_args(&cfg))` directly — this would have failed the `args[0].contains("claude")` assertion. The closure prepends `"claude"` before extending with `build_cli_args` output. Correct fix; captured as D115.
- **`Char('w')` in model-focus handler falls through to save**: The plan did not explicitly specify this behavior. Required by the `settings_w_save_includes_model_fields` test; the correct UX. Captured as D116.

## Known Limitations

- Ollama and OpenAI provider paths are tested for correct CLI args only. Real subprocess invocation (live `ollama run` or live `openai` CLI) is UAT-only — the closures return the correct args vec but no automated test exercises a real subprocess for these providers.
- `run_dir` in the `r` key handler still uses `temp_dir/assay-agent-{chunk_slug}` (D114 from S01). The harness config is written there for the Anthropic path; Ollama/OpenAI paths ignore `_path` entirely (no filesystem writes). Real worktree-based path is a future improvement.
- The OpenAI closure returns `["openai", "api", "chat.completions.create", "--model", model]` which assumes the `openai` CLI tool is installed. This is a placeholder — the actual OpenAI invocation path may need a different binary or API approach in a real deployment.

## Follow-ups

- S03 (slash command overlay) is next; it consumes the `TuiEvent` loop from S01 — no S02 surfaces are required by S03.
- Future: wire real worktree path into `run_dir` for all providers (supersedes D114).
- Future: validate OpenAI invocation path against actual `openai` CLI binary or SDK.

## Files Created/Modified

- `crates/assay-tui/src/agent.rs` — new; `provider_harness_writer` dispatch + `OllamaConfig`/`OpenAiConfig` structs
- `crates/assay-tui/src/lib.rs` — added `pub mod agent;`
- `crates/assay-tui/src/app.rs` — `Screen::Settings` extended; `s`/`w`/Settings event handlers updated; `r` key uses `provider_harness_writer`; added `use crate::agent::provider_harness_writer`
- `crates/assay-tui/tests/provider_dispatch.rs` — new; 3 provider dispatch contract tests
- `crates/assay-tui/tests/settings.rs` — 2 new model-field tests appended

## Forward Intelligence

### What the next slice should know
- S03 only needs the `TuiEvent` loop (from S01). No S02 surfaces (agent dispatch, Settings model fields) are consumed by S03.
- The Settings event handler now has a model-focus guard block at the top — any future Settings key handling must account for this guard or keys will be silently swallowed when model_focus is Some.

### What's fragile
- `draw_settings` has 9 parameters and an `#[allow(clippy::too_many_arguments)]` suppression — if more settings are added, refactoring to a settings-struct param is the correct fix.
- Ollama/OpenAI closures ignore `_path` and `_profile` entirely. If future work needs these providers to write harness config files (e.g. for context injection), the closures need to be updated.

### Authoritative diagnostics
- `cargo test -p assay-tui --test provider_dispatch` — single command for dispatch correctness; compile error means `agent` module absent.
- `cargo test -p assay-tui --test settings` — Settings state machine regression check (7 tests).

### What assumptions changed
- `build_cli_args` was assumed to return a full invocation including the binary — it returns flags only. Any future harness adapter writing a `HarnessWriter` closure must explicitly set `args[0]` to the binary name.
