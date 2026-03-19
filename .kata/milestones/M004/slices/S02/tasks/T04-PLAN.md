---
estimated_steps: 3
estimated_files: 2
---

# T04: just ready — lint, test, snapshot lockdown

**Slice:** S02 — Mesh Mode
**Milestone:** M004

## Description

Run `just ready` and achieve a green result (fmt ✓, lint ✓, test ✓, deny ✓) with zero clippy warnings. Fix any minor lint issues introduced by T03's implementation. Verify all snapshot files are committed and stable. This is the final gate for S02.

## Steps

1. Run `just ready` and capture the output. If it fails, diagnose the failure:
   - **fmt**: run `just fmt` to auto-fix formatting issues
   - **lint**: fix each clippy warning individually — common issues are unused imports, dead_code, unused variables in routing thread, overly complex expressions. Do not suppress warnings with `#[allow(...)]` unless the pattern is intentional and established elsewhere in the codebase.
   - **test**: if a test fails, investigate — do not move on until all tests pass
   - **deny**: if a new dependency was introduced (unlikely since mesh.rs uses only std), add it to `deny.toml` if needed

2. Verify schema snapshots are stable: run `cargo test -p assay-types --features orchestrate -- schema_snapshots 2>&1` with no `INSTA_UPDATE` flag. All tests must pass without regeneration — if any fail, investigate whether T01 or T03 introduced a schema change that wasn't captured.

3. Run `git diff --name-only crates/assay-types/tests/snapshots/` — must show no unstaged changes. If there are unstaged snapshot files, they need to be accepted via `cargo insta review` or `INSTA_UPDATE=always` + commit.

## Must-Haves

- [ ] `just ready` exits 0 with `fmt ✓ lint ✓ test ✓ deny ✓`
- [ ] 0 clippy warnings across all crates
- [ ] All 4 new/updated snapshot files committed (no pending `*.snap.new` files)
- [ ] All integration tests pass (including both mesh tests from T02/T03)

## Verification

- `just ready` — must exit 0
- `find crates/assay-types/tests/snapshots -name "*.snap.new"` — must return empty (no pending snapshots)
- `cargo test --workspace --features "assay-core/orchestrate,assay-types/orchestrate,assay-mcp/orchestrate"` — final full test suite confirmation

## Observability Impact

- Signals added/changed: None — this task only fixes quality issues
- How a future agent inspects this: `just ready` output is the canonical verification; snapshot files in `crates/assay-types/tests/snapshots/` are the locked schema contract
- Failure state exposed: `just lint` output will show specific warning locations if any exist

## Inputs

- T01, T02, T03 outputs — all code and snapshot files from previous tasks
- `Cargo.toml` (workspace root) — for deny.toml consultation if a new dependency is flagged

## Expected Output

- Minor edits to `mesh.rs` if lint warnings exist (unused imports, etc.)
- `just ready` green — this is the observable slice completion signal
