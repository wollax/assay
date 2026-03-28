---
estimated_steps: 2
estimated_files: 0
---

# T03: Full workspace verification

**Slice:** S01 — GitHubBackend correctness fixes (Q001–Q004)
**Milestone:** M013

## Description

Run `just ready` to confirm the full workspace compiles, passes clippy, formatting, and all 1529+ tests with zero regressions. Fix any issues introduced by the T01/T02 changes.

## Steps

1. Run `just ready` (equivalent to `just fmt && just lint && just test && just deny`). If any step fails, diagnose and fix the issue in the relevant file.
2. Confirm test count is ≥1529 and all pass.

## Must-Haves

- [ ] `just ready` exits 0
- [ ] No new clippy warnings
- [ ] Test count ≥1529

## Verification

- `just ready` exits 0
- `cargo test -p assay-backends --features github 2>&1 | tail -5` shows all tests passing

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: None
- Failure state exposed: None

## Inputs

- All changes from T01 and T02 committed to working tree
- `justfile` — defines the `ready` recipe

## Expected Output

- No file changes (verification-only task) — or minor formatting/clippy fixes if needed
