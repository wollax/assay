---
estimated_steps: 5
estimated_files: 2
---

# T04: just ready verification pass

**Slice:** S03 — Gossip Mode
**Milestone:** M004

## Description

Run `just ready` to confirm the full workspace is green — fmt, lint (0 warnings), all tests passing, deny clean — before the slice is declared complete. Fix any issues found. This task also confirms the integration test count is at least as large as the pre-S03 count, proving no regressions.

## Steps

1. Run `cargo fmt --all -- --check` to detect any formatting issues. If issues exist, run `cargo fmt --all` and review changes.

2. Run `cargo clippy --workspace --all-targets --features orchestrate -- -D warnings` to catch all lint warnings. Fix any clippy warnings before proceeding (0 tolerance).

3. Run `cargo test --workspace --features orchestrate` and check total test count is ≥ pre-S03 count (1222+). Confirm `gossip_integration` tests appear in output and pass.

4. Run `cargo deny check` to verify no new dependency issues.

5. Run `just ready` for the final confirmation — must exit 0.

## Must-Haves

- [ ] `just ready` exits 0
- [ ] 0 clippy warnings
- [ ] Total test count ≥ 1222 (no regressions)
- [ ] `gossip_integration::test_gossip_mode_knowledge_manifest` passes
- [ ] `gossip_integration::test_gossip_mode_manifest_path_in_prompt_layer` passes
- [ ] All snapshot tests in `assay-types` pass

## Verification

- `just ready` exits 0 with output containing `fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓`
- `cargo test --workspace --features orchestrate 2>&1 | grep "test result"` shows all crates passing

## Observability Impact

- Signals added/changed: None (verification only)
- How a future agent inspects this: `just ready` is the canonical verification command; individual crate checks can be scoped with `cargo test -p <crate> --features orchestrate`
- Failure state exposed: Any regression surfaces immediately as a failed test in the workspace test run

## Inputs

- All files modified in T01–T03 — the workspace must compile and test cleanly with all changes integrated
- `justfile` — defines the `ready` target sequence

## Expected Output

- No file changes expected (lint/fmt fixes only if needed)
- `just ready` output confirming: fmt ✓, lint ✓ (0 warnings), test ✓, deny ✓
