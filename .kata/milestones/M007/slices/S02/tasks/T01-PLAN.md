---
estimated_steps: 4
estimated_files: 1
---

# T01: Rebase onto origin/main and write failing provider dispatch tests

**Slice:** S02 — Provider Dispatch and Harness Wiring
**Milestone:** M007

## Description

Rebase the `kata/root/M007/S02` branch onto `origin/main` to acquire all S01 deliverables
(`TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, `App.event_tx`, `App.agent_thread`,
`launch_agent_streaming`, the channel-based `run()`). Then write the test contract in
`tests/provider_dispatch.rs` — three tests that will fail with a compile error until T02
provides `assay_tui::agent::provider_harness_writer`. This is the test-first anchor for T02.

## Steps

1. From the `kata/root/M007/S02` branch, run `git rebase origin/main`. Resolve any conflicts
   (the only files on this branch are the research file and kata artifacts, so conflicts are
   unlikely). Verify the rebase succeeded with `git log --oneline -5` and confirm the S01
   squash commit appears in history.

2. Confirm the baseline compiles: `cargo build -p assay-tui`. Fix any immediate issues from
   the rebase before writing tests.

3. Create `crates/assay-tui/tests/provider_dispatch.rs` with:
   - A `use` import for `assay_tui::agent::provider_harness_writer` (will cause compile error
     until T02)
   - A `use` import for `assay_types::{Config, ProviderConfig, ProviderKind}`
   - A `use` import for `assay_types::HarnessProfile` and `assay_types::SettingsOverride`
   - A helper `fn config_with_provider(kind: ProviderKind) -> Config` that constructs:
     ```rust
     Config {
         project_name: "test".into(),
         specs_dir: "specs/".into(),
         gates: None, guard: None, worktree: None, sessions: None,
         provider: Some(ProviderConfig {
             provider: kind, planning_model: None, execution_model: None, review_model: None
         }),
     }
     ```
     (`Config` has no `Default` impl due to `deny_unknown_fields`; full struct literal is required)
   - A helper `fn run_writer(config: &Config) -> Vec<String>`: creates a `tempfile::TempDir`,
     constructs a minimal `HarnessProfile` (name "test", empty layers/settings/hooks), calls
     `provider_harness_writer(Some(config))` (the function takes `Option<&Config>`), invokes
     the returned closure with `(&profile, tmp.path())`, returns the `Vec<String>` on `Ok`,
     panics on `Err`
   - Three `#[test]` functions:
     - `provider_dispatch_anthropic_uses_claude_binary`: builds `config_with_provider(ProviderKind::Anthropic)`,
       calls `run_writer`, asserts `args[0].contains("claude")` (the binary may be an absolute
       path on some systems, so `contains` is more robust than `==`)
     - `provider_dispatch_ollama_uses_ollama_binary`: builds `config_with_provider(ProviderKind::Ollama)`,
       calls `run_writer`, asserts `args[0] == "ollama"`
     - `provider_dispatch_openai_uses_openai_binary`: builds `config_with_provider(ProviderKind::OpenAi)`,
       calls `run_writer`, asserts `args[0] == "openai"`

4. Verify the expected state: `cargo test -p assay-tui --test provider_dispatch 2>&1 | head -20`
   must show a compile error referencing `assay_tui::agent` (or a missing module). The pre-existing
   35 tests must still pass: `cargo test -p assay-tui --tests 2>&1 | grep -E "test result|FAILED"`.

## Must-Haves

- [ ] `git rebase origin/main` completed successfully; `git log --oneline -3` shows S01 squash commit
- [ ] `cargo build -p assay-tui` passes after rebase
- [ ] `crates/assay-tui/tests/provider_dispatch.rs` exists with 3 properly-structured `#[test]` functions
- [ ] Each test references `provider_harness_writer` and the correct provider enum variant
- [ ] Running `cargo test -p assay-tui --test provider_dispatch` fails with a compile error for missing `agent` module (expected — confirms the import is wired correctly)
- [ ] All pre-existing 35+ assay-tui tests still pass

## Verification

- `git log --oneline -5` — S01 squash commit (`[kata/root/M007/S01] Channel Event Loop…`) visible
- `cargo build -p assay-tui` — exits 0
- `cargo test -p assay-tui --test provider_dispatch 2>&1 | grep "unresolved import\|error\[E"` — at least one compile error referencing the missing agent module
- `cargo test -p assay-tui --lib --tests 2>&1 | tail -5` — all pre-existing tests still pass (no regressions)

## Observability Impact

- Signals added/changed: None (test scaffolding only; no runtime code changed)
- How a future agent inspects this: Test file at `tests/provider_dispatch.rs` is the contract; `cargo test -p assay-tui --test provider_dispatch` is the single verification command
- Failure state exposed: Compile error is the visible failure signal — confirms the import path is correct before T02 creates the module

## Inputs

- `origin/main` with S01 squash commit — `TuiEvent`, `Screen::AgentRun`, `App.event_tx`, etc.
- `S02-RESEARCH.md` — provider dispatch function signature, closure patterns, `Box<dyn Fn>` ownership constraints
- S01-SUMMARY.md `provides` section — exact field and function names to import in tests

## Expected Output

- `crates/assay-tui/tests/provider_dispatch.rs` — 3 failing-compile tests that precisely describe the `provider_harness_writer` contract
- Branch is rebased; baseline build is green
