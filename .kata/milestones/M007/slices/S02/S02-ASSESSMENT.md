# S02 Post-Slice Roadmap Assessment

**Date:** 2026-03-23
**Verdict:** Roadmap unchanged — remaining slices S03 and S04 are still valid as written.

## Risk Retirement

All three key risks in M007 are now retired:
- **Unified TUI event loop** — retired in S01 (channel-based `TuiEvent` dispatch)
- **Streaming vs completion** — retired in S01 (`launch_agent_streaming` free function)
- **Provider CLI invocation differences** — retired in S02 (`provider_harness_writer` with Anthropic/Ollama/OpenAI dispatch, unit-tested)

S03 (slash commands, risk:low) and S04 (MCP panel, risk:medium) carry no remaining architectural unknowns.

## Success Criteria Coverage

| Criterion | Owner | Status |
|-----------|-------|--------|
| `r` key spawns agent, streams output, shows Done/Failed | S01 | ✓ complete |
| Gate results refresh after agent exits | S01 | ✓ complete |
| Provider dispatch routes to correct CLI binary; model from ProviderConfig passed | S02 | ✓ complete |
| `/` opens slash overlay; `/gate-check`, `/status`, `/pr-create` work | S03 | remaining ✓ |
| `m` opens MCP panel; add/delete servers; persist to `.assay/mcp.json` | S04 | remaining ✓ |
| `just ready` passes; no deadlock or panic | S03 + S04 | remaining ✓ |

All criteria have at least one remaining owning slice. No blocking gaps.

## Boundary Map Accuracy

S02 delivered exactly what the boundary map specified: `provider_harness_writer` dispatch function, `OllamaConfig`/`OpenAiConfig` structs, and Settings model-field state machine. The S03 boundary entry (consumes only `TuiEvent` loop from S01) is confirmed accurate — S02 forward intelligence explicitly states S03 needs nothing from S02. S04 is fully independent. No boundary changes required.

## Requirement Coverage

- R054 (Provider abstraction) — validated by S02; unit tests prove correct CLI args per provider
- R055 (TUI MCP server management) — owned by S04; coverage unchanged
- R056 (TUI slash commands) — owned by S03; coverage unchanged

Coverage remains sound across all active M007 requirements.

## Notable Forward Intelligence for S03/S04

- The Settings event handler now has a **model-focus guard block** at the top of the Settings arm. Any future Settings key handling added in S03 or S04 must account for this guard — keys are silently swallowed when `model_focus` is `Some`.
- `draw_settings` has 9 parameters with `#[allow(clippy::too_many_arguments)]`. If S03 adds Settings interaction, prefer extracting a settings-struct param rather than adding more arguments.
- S04 is independent of S01/S02/S03 and can be developed in parallel if desired.
