---
estimated_steps: 6
estimated_files: 3
---

# T02: Create `assay-tui::agent` module and wire `r` key handler

**Slice:** S02 — Provider Dispatch and Harness Wiring
**Milestone:** M007

## Description

Create `crates/assay-tui/src/agent.rs` with the `provider_harness_writer` dispatch function
and TUI-local `OllamaConfig` / `OpenAiConfig` structs. Declare the module in `lib.rs`. Replace
the hardcoded Claude adapter block in the `r` key handler (`app.rs` Dashboard arm) with a call
to `provider_harness_writer`. Makes the three `provider_dispatch` tests green.

## Steps

1. Create `crates/assay-tui/src/agent.rs`:
   - Add imports: `assay_core::pipeline::HarnessWriter`, `assay_types::{Config, HarnessProfile, ProviderKind}`
   - Define `pub struct OllamaConfig { pub model: String }` and
     `pub struct OpenAiConfig { pub model: String, pub api_key_env: String }` — TUI-local, not
     persisted to `assay-types`
   - Implement `pub fn provider_harness_writer(config: Option<&Config>) -> Box<HarnessWriter>`:
     - `Config` does NOT derive `Default` (it has `deny_unknown_fields` and a required `project_name`)
       so the function takes `Option<&Config>` to cleanly handle the no-config case
     - Determine provider: `config.and_then(|c| c.provider.as_ref()).map(|p| p.provider).unwrap_or(ProviderKind::Anthropic)`
     - Extract execution model string: `config.and_then(|c| c.provider.as_ref()).and_then(|p| p.execution_model.clone())`
     - Match on provider kind and return a `'static` closure:
       - `Anthropic`: `Box::new(move |profile: &HarnessProfile, path: &std::path::Path| { let cfg = assay_harness::claude::generate_config(profile); assay_harness::claude::write_config(&cfg, path).map_err(|e| e.to_string())?; Ok(assay_harness::claude::build_cli_args(&cfg)) })`
       - `Ollama`: capture `let model = model_opt.unwrap_or_else(|| "llama3".into()); Box::new(move |_profile, _path| Ok(vec!["ollama".into(), "run".into(), model.clone()]))`
       - `OpenAi`: capture `let model = model_opt.unwrap_or_else(|| "gpt-4o".into()); Box::new(move |_profile, _path| Ok(vec!["openai".into(), "api".into(), "chat.completions.create".into(), "--model".into(), model.clone()]))`
   - Use `_path: &std::path::Path` (prefixed underscore) for Ollama/OpenAI to suppress unused-variable warnings

2. Declare `pub mod agent;` in `crates/assay-tui/src/lib.rs` (add after `pub mod wizard;`).

3. In `crates/assay-tui/src/app.rs`, import `provider_harness_writer`:
   - Add `use crate::agent::provider_harness_writer;` at the top with other `use crate::` imports

4. In `app.rs` Dashboard arm `r` key handler, locate the comment `// S02 replaces this with real provider dispatch from app.config.` The block to replace runs from `let profile = HarnessProfile {` through `let cli_args = assay_harness::claude::build_cli_args(&claude_config);`. Replace this entire block with:
   ```rust
   // Build a minimal HarnessProfile for provider dispatch.
   let profile = HarnessProfile {
       name: chunk_slug.clone(),
       prompt_layers: vec![],
       settings: SettingsOverride {
           model: None,
           permissions: vec![],
           tools: vec![],
           max_turns: None,
       },
       hooks: vec![],
       working_dir: None,
   };
   let writer = provider_harness_writer(self.config.as_ref());
   let cli_args = match writer(&profile, &run_dir) {
       Ok(args) => args,
       Err(_) => return false,
   };
   ```
   `self.config.as_ref()` yields `Option<&Config>` which matches the updated signature —
   no `Config::default()` needed (`Config` does not derive `Default`).

5. Remove the now-unused import `use assay_harness::claude;` if it was used only in the replaced block. Run `cargo build -p assay-tui` — resolve any remaining unused-import warnings by removing or qualifying them.

6. Run the full test suite to confirm green: `cargo test -p assay-tui`.

## Must-Haves

- [ ] `src/agent.rs` exists with `provider_harness_writer`, `OllamaConfig`, `OpenAiConfig`
- [ ] `lib.rs` declares `pub mod agent;`
- [ ] Anthropic closure calls `assay_harness::claude::generate_config`, `write_config`, `build_cli_args` in sequence
- [ ] Ollama and OpenAI closures capture an owned `String` via `move` — no reference capture
- [ ] `r` key handler no longer has a hardcoded `claude` call; uses `provider_harness_writer` instead
- [ ] `cargo test -p assay-tui --test provider_dispatch` → 3/3 pass
- [ ] All pre-existing 35+ assay-tui tests still pass
- [ ] `cargo build -p assay-tui` has no warnings (all unused imports removed)

## Verification

- `cargo test -p assay-tui --test provider_dispatch` — 3/3: `provider_dispatch_anthropic_uses_claude_binary`, `provider_dispatch_ollama_uses_ollama_binary`, `provider_dispatch_openai_uses_openai_binary`
- `cargo test -p assay-tui` — all tests pass (provider_dispatch + pre-existing 35)
- `grep "S02 replaces" crates/assay-tui/src/app.rs` — exits 1 (comment removed; replaced with real dispatch)
- `cargo build -p assay-tui 2>&1 | grep "^warning:"` — zero warnings

## Observability Impact

- Signals added/changed: `r` key handler now routes through `provider_harness_writer`; the `cli_args[0]` in the relay-wrapper thread reflects the configured provider's binary name; this is already visible in `Screen::AgentRun` output
- How a future agent inspects this: `app.config.provider.provider` shows the configured provider; `provider_harness_writer` in `src/agent.rs` is the single dispatch point to inspect
- Failure state exposed: `writer(&profile, &run_dir)` returning `Err(String)` causes `r` key to return `false` silently (same behavior as S01 harness-write failure); the error string is available but not currently surfaced to the UI — acceptable for S02 scope

## Inputs

- `crates/assay-tui/tests/provider_dispatch.rs` from T01 — the exact function signature and test assertions to satisfy
- `S02-RESEARCH.md` — closure ownership patterns, `'static` bound, `move` capture, `_path` convention for unused params
- `origin/main:crates/assay-tui/src/app.rs` lines 381–404 — the exact hardcoded block to replace (confirmed by `// S02 replaces this` comment)
- `origin/main:crates/assay-harness/src/claude.rs` — `generate_config`, `write_config`, `build_cli_args` signatures

## Expected Output

- `crates/assay-tui/src/agent.rs` — fully implemented provider dispatch module
- `crates/assay-tui/src/lib.rs` — `pub mod agent` added
- `crates/assay-tui/src/app.rs` — `r` key handler uses `provider_harness_writer`; no hardcoded claude calls remaining
- `cargo test -p assay-tui --test provider_dispatch` → 3/3 green
