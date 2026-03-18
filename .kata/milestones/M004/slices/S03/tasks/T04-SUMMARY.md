---
id: T04
parent: S03
milestone: M004
provides:
  - just ready exits 0 — fmt, lint (0 warnings), all tests passing, deny clean
key_files: []
key_decisions: []
patterns_established: []
observability_surfaces:
  - none (verification-only task)
duration: ~5 minutes
verification_result: passed
completed_at: 2026-03-18
blocker_discovered: false
---

# T04: just ready verification pass

**`just ready` exits 0 with fmt ✓, lint ✓ (0 warnings), test ✓ (1264 tests, ≥1222 threshold), deny ✓ — S03 Gossip Mode is complete.**

## What Happened

Ran the full verification suite in sequence:

1. `cargo fmt --all -- --check` — no output, formatting is clean.
2. `cargo clippy --workspace --all-targets --features orchestrate -- -D warnings` — finished with 0 warnings, exit 0.
3. `cargo test --workspace --features orchestrate` — 1264 tests passed across all crates. Both gossip_integration tests passed:
   - `test_gossip_mode_knowledge_manifest` ✓
   - `test_gossip_mode_manifest_path_in_prompt_layer` ✓
   All 64 schema snapshot tests in `assay-types` passed, including `knowledge_entry_schema_snapshot`, `knowledge_manifest_schema_snapshot`, `gossip_status_schema_snapshot`, and `orchestrator_status_schema_snapshot`.
4. `cargo deny check` — advisories ok, bans ok, licenses ok, sources ok (two pre-existing unmatched-license warnings, not errors).
5. `just ready` — exited 0, all checks passed ("Plugin versions match workspace (0.4.0). All checks passed.").

## Verification

- `cargo fmt --all -- --check`: clean (no output)
- `cargo clippy --workspace --all-targets --features orchestrate -- -D warnings`: 0 warnings, exit 0
- `cargo test --workspace --features orchestrate`: 1264 total tests, 0 failures — exceeds pre-S03 threshold of 1222
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok
- `just ready`: exit 0

## Diagnostics

- `just ready` is the canonical verification command for the workspace
- Individual crate scope: `cargo test -p assay-core --features orchestrate --test gossip_integration`
- Gossip runtime inspection: `cat .assay/orchestrator/<run_id>/state.json | jq .gossip_status`
- Knowledge manifest: `cat .assay/orchestrator/<run_id>/gossip/knowledge.json | jq '.entries | length'`

## Deviations

none

## Known Issues

none

## Files Created/Modified

- None — verification-only task, no source changes required.
