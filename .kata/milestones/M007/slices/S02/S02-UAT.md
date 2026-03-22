# S02: Provider Dispatch and Harness Wiring — UAT

**Milestone:** M007
**Written:** 2026-03-21

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Automated tests prove correct CLI args per provider. The only thing requiring human verification is that the TUI actually invokes `ollama` (or `openai`) when the provider is changed in Settings — this requires a live binary installed and the TUI running against a real project.

## Preconditions

1. `just ready` exits 0 (already verified).
2. `cargo build -p assay-tui` produces `target/debug/assay-tui`.
3. An Assay project exists with at least one InProgress chunk (needed for the `r` key to activate).
4. Ollama is installed and a model (e.g. `llama3`) is available locally (`ollama list` shows it).
5. TUI is launched from the project root: `assay-tui`.

## Smoke Test

Open Settings (`s`), confirm three model input rows appear below the provider list (Planning model / Execution model / Review model). Tab into the Planning model field, type a value, press `w`. Re-launch the TUI; confirm the value is pre-populated.

## Test Cases

### 1. Ollama provider routes `r` to `ollama` binary

1. Open Settings with `s`.
2. Navigate to Ollama with `↓` arrow.
3. Tab into the Execution model field; type `llama3` (or the installed model name).
4. Press `w` to save.
5. Return to Dashboard with `Esc`.
6. Navigate to an InProgress chunk.
7. Press `r`.
8. **Expected:** `Screen::AgentRun` opens; the first output lines (or the status line at the bottom) show activity from `ollama run llama3`, not from `claude`. The invocation binary is `ollama`.

### 2. Model fields pre-populated on Settings open

1. Save Anthropic provider with `planning_model = "claude-3-haiku"` via Settings `w`.
2. Press `Esc` to leave Settings.
3. Press `s` to re-open Settings.
4. **Expected:** The Planning model field displays `"claude-3-haiku"` without any typing.

### 3. Backspace and Tab navigation in model fields

1. Open Settings; Tab into the Planning model field (should focus with cyan highlight).
2. Type `abc`.
3. Press `Backspace` once.
4. **Expected:** Field shows `"ab"`.
5. Press `Tab`.
6. **Expected:** Focus moves to Execution model field.
7. Press `Tab`.
8. **Expected:** Focus moves to Review model field.
9. Press `Tab`.
10. **Expected:** Focus returns to the provider list (`model_focus = None`); arrows navigate providers again.

### 4. Anthropic still works after provider switch

1. Switch back to Anthropic in Settings; save with `w`.
2. Press `r` from Dashboard on an InProgress chunk.
3. **Expected:** `Screen::AgentRun` opens and `claude` binary is invoked (same as S01 UAT behavior).

## Edge Cases

### Empty model field saves as None

1. Open Settings; Tab to the Execution model field.
2. Clear any existing text with Backspace until empty.
3. Press `w`.
4. Inspect `.assay/config.toml` — `execution_model` key should be absent (not `execution_model = ""`).
5. **Expected:** Field saved as `None`; absent from TOML.

### Esc in model section does not leave Settings

1. Open Settings; Tab into the Planning model field.
2. Press `Esc`.
3. **Expected:** Focus returns to the provider list (model_focus = None); Settings screen remains open. A second `Esc` exits Settings.

## Failure Signals

- `Screen::AgentRun` shows `claude` output despite Ollama being configured — dispatch not routing correctly.
- Model fields are blank on Settings re-open despite having been saved — `s` key handler not reading from config.
- Pressing `Esc` in a model field exits Settings entirely — focus guard incorrectly triggering screen exit.
- `w` while a model field is focused appends `'w'` to the buffer instead of saving — fall-through logic broken.

## Requirements Proved By This UAT

- R054 (Provider abstraction) — live Ollama invocation from `r` key proves the dispatch path reaches the correct binary end-to-end, not just the arg-generation unit tests.
- R052 (TUI provider configuration) — model fields editable in TUI and persisted to `.assay/config.toml` confirms the full settings persistence loop works for model values.

## Not Proven By This UAT

- OpenAI provider invocation — requires `openai` CLI installed; not commonly available. The dispatch logic is identical to Ollama; CLI arg tests cover the contract.
- Real agent output quality for Ollama — Assay sends the correct harness config and args; what Ollama produces is outside Assay's scope.
- Worktree-based `run_dir` for Ollama — S01 D114 still applies; `temp_dir` is used. Ollama will not have project file access unless this is resolved in a future slice.

## Notes for Tester

- If Ollama is not installed, skip test case 1 and verify dispatch only via `cargo test -p assay-tui --test provider_dispatch`.
- The `openai` CLI path (`["openai", "api", "chat.completions.create", "--model", model]`) is a placeholder. Real OpenAI invocation may need a different binary; this is a known limitation (see S02-SUMMARY Known Limitations).
- Model field Tab navigation is linear: Planning → Execution → Review → provider list. There is no wrap-around back to Planning.
