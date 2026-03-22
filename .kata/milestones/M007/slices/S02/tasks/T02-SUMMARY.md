---
id: T02
parent: S02
milestone: M007
provides:
  - "`crates/assay-tui/src/agent.rs` — `provider_harness_writer(Option<&Config>) -> Box<HarnessWriter>` dispatch function"
  - "TUI-local `OllamaConfig` and `OpenAiConfig` structs in `agent.rs`"
  - "`pub mod agent` declared in `lib.rs`"
  - "`r` key handler in `app.rs` Dashboard arm dispatches via `provider_harness_writer` instead of hardcoded claude calls"
  - "3/3 `provider_dispatch` tests green; all 38 assay-tui tests pass"
key_files:
  - crates/assay-tui/src/agent.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/app.rs
key_decisions:
  - "Anthropic closure prepends `\"claude\"` as args[0] before the flags returned by `build_cli_args` — `build_cli_args` returns only flags (e.g. `--print`), not the binary name; the test asserts `args[0].contains(\"claude\")`"
  - "Ollama and OpenAi closures ignore `_path` and `_profile` (prefixed underscores to suppress warnings); no filesystem writes needed"
  - "No `OllamaConfig`/`OpenAiConfig` wiring beyond struct definition — closures capture `model: String` directly via `move`"
patterns_established:
  - "`agent.rs` is the single provider dispatch authority; future providers add a new `ProviderKind` arm here"
  - "HarnessWriter closure for Anthropic: prepend binary name, extend with `build_cli_args` output"
observability_surfaces:
  - "`cli_args[0]` in the relay-wrapper thread shows the configured provider binary (claude/ollama/openai); visible in Screen::AgentRun output"
  - "`cargo test -p assay-tui --test provider_dispatch` is the diagnostic command for dispatch correctness"
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Create `assay-tui::agent` module and wire `r` key handler

**Provider dispatch module created; `r` key now routes through `provider_harness_writer` with Anthropic/Ollama/OpenAI arms; 3/3 provider_dispatch tests green.**

## What Happened

Created `crates/assay-tui/src/agent.rs` with the `provider_harness_writer` dispatch function. The function takes `Option<&Config>`, extracts `ProviderKind` (defaulting to `Anthropic`) and `execution_model`, then returns a `'static` closure matching the provider:

- **Anthropic**: calls `generate_config`, `write_config`, then prepends `"claude"` before `build_cli_args` output. The prepend was required because `build_cli_args` returns only flags (`--print`, `--output-format`, ...) not the binary name, and the test asserts `args[0].contains("claude")`.
- **Ollama**: captures `model` (defaulting to `"llama3"`), returns `["ollama", "run", model]`.
- **OpenAi**: captures `model` (defaulting to `"gpt-4o"`), returns `["openai", "api", "chat.completions.create", "--model", model]`.

Declared `pub mod agent` in `lib.rs`. Added `use crate::agent::provider_harness_writer` import to `app.rs`. Replaced the `S02 replaces this` comment block in the Dashboard `r` key handler with a call to `provider_harness_writer(self.config.as_ref())`.

## Verification

- `cargo test -p assay-tui --test provider_dispatch` → 3/3 pass (`anthropic`, `ollama`, `openai`)
- `cargo test -p assay-tui` → 38 tests pass (3 new + 35 pre-existing)
- `grep "S02 replaces" crates/assay-tui/src/app.rs` → exits 1 (comment removed)
- `cargo build -p assay-tui 2>&1 | grep "^warning:"` → zero output (no warnings)

## Diagnostics

- `cargo test -p assay-tui --test provider_dispatch` is the primary diagnostic: compile error means `agent` module absent; test failures mean wrong dispatch args
- `cli_args[0]` in the relay-wrapper thread exposes the configured provider binary at runtime

## Deviations

`build_cli_args` returns flags only (not the binary name), so the Anthropic closure prepends `"claude"` before extending with `build_cli_args` output. The task plan said `Ok(assay_harness::claude::build_cli_args(&cfg))` directly — this would have failed the test. The prepend is the correct fix.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/agent.rs` — new; `provider_harness_writer` dispatch + `OllamaConfig`/`OpenAiConfig` structs
- `crates/assay-tui/src/lib.rs` — added `pub mod agent;`
- `crates/assay-tui/src/app.rs` — replaced hardcoded claude block with `provider_harness_writer` call; added import
