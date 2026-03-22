---
id: T01
parent: S02
milestone: M007
provides:
  - Branch `kata/root/M007/S02` rebased onto `origin/main`; S01 squash commit `d97a1be` visible in log
  - `crates/assay-tui/tests/provider_dispatch.rs` — 3 failing-compile tests anchoring the `provider_harness_writer` contract
  - Test contract: Anthropic → binary contains "claude"; Ollama → "ollama"; OpenAI → "openai"
  - Helper `config_with_provider(ProviderKind)` constructs full `Config` struct literal (no Default due to deny_unknown_fields)
  - Helper `run_writer(&Config)` creates TempDir, constructs minimal `HarnessProfile`, invokes the writer closure
key_files:
  - crates/assay-tui/tests/provider_dispatch.rs
key_decisions:
  - Compile-error test anchor: importing `assay_tui::agent::provider_harness_writer` before the module exists produces a single clear E0432 error; this is the intended T01 state
  - HarnessProfile requires explicit construction (no Default impl due to deny_unknown_fields); SettingsOverride similarly explicit
  - args[0].contains("claude") used for Anthropic (binary may be absolute path on some systems); == used for Ollama and OpenAI
patterns_established:
  - Test file at `tests/provider_dispatch.rs` is the verification command: `cargo test -p assay-tui --test provider_dispatch`
  - Pre-existing tests run with explicit `--test <name>` flags to avoid compile errors from the new failing test file during T01 verification
observability_surfaces:
  - cargo test -p assay-tui --test provider_dispatch — single command to observe contract compliance or its absence
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Rebase onto origin/main and write failing provider dispatch tests

**Branch rebased onto S01 squash commit; three compile-failing tests written that precisely describe the `provider_harness_writer(Option<&Config>) -> Box<HarnessWriter>` dispatch contract.**

## What Happened

The `kata/root/M007/S02` branch was created before S01's PR #168 merged to `origin/main`. Rebasing replayed four branch-local commits: the M006 completion kata state, the M007 planning doc, the S02 research auto-commit, and the S02 slice plan. Three conflicts arose, all in kata artifact files (`.kata/STATE.md`, `.kata/DECISIONS.md`, `crates/assay-tui/src/app.rs`). All were resolved by keeping HEAD (origin/main with S01 deliverables) over the older branch-local versions — the branch commits predated S01 and the HEAD had all S01 additions.

After rebase, `cargo build -p assay-tui` passed immediately (no S01-introduced breakage).

`crates/assay-tui/tests/provider_dispatch.rs` was written with:
- `use assay_tui::agent::provider_harness_writer` — import that produces the intentional compile error
- `use assay_types::{Config, HarnessProfile, ProviderConfig, ProviderKind, SettingsOverride}` 
- `config_with_provider(ProviderKind) -> Config` — full struct literal required (Config has no Default due to `deny_unknown_fields` on ProviderConfig)
- `run_writer(&Config) -> Vec<String>` — creates TempDir, builds minimal HarnessProfile (all optional fields empty), calls `provider_harness_writer(Some(config))`, invokes the returned closure
- Three `#[test]` functions checking `args[0]` per provider kind

## Verification

- `git log --oneline -5`: S01 commit `d97a1be [kata/root/M007/S01] Channel Event Loop and Agent Run Panel (#168)` confirmed in history ✓
- `cargo build -p assay-tui`: exits 0 ✓  
- `cargo test -p assay-tui --test provider_dispatch`: fails with `error[E0432]: unresolved import assay_tui::agent` — exactly one compile error, import wired correctly ✓
- Pre-existing tests (35 total across 6 test files): all pass individually (8+1+6+5+6+9=35) ✓

## Diagnostics

The test file at `crates/assay-tui/tests/provider_dispatch.rs` is both the contract and the diagnostic surface. Running `cargo test -p assay-tui --test provider_dispatch` tells a future agent whether T02's `agent::provider_harness_writer` is correctly wired. A compile error means the module is absent; test failures mean the dispatch returns wrong args.

Note: After T01, running `cargo test -p assay-tui --tests` (all integration tests at once) will fail due to the provider_dispatch compile error. Run test files individually until T02 creates the `agent` module.

## Deviations

None.

## Known Issues

None. The compile failure in `provider_dispatch.rs` is the intended state for T01 — it is the test-first anchor, not a defect.

## Files Created/Modified

- `crates/assay-tui/tests/provider_dispatch.rs` — 3 failing-compile tests for provider dispatch contract
