---
estimated_steps: 5
estimated_files: 0
---

# T03: Final verification pass and cleanup

**Slice:** S01 — Decompose manifest.rs and git/cli.rs
**Milestone:** M011

## Description

Run all quality gates in one clean sweep to confirm the decomposition is complete and correct. Fix any clippy or doc warnings that emerged from the restructuring. This task produces no new files — it verifies and polishes the outputs of T01 and T02.

## Steps

1. Run `cargo clippy --workspace` — fix any new warnings from the restructuring (e.g., unused imports, redundant pub qualifiers).
2. Run `cargo doc --workspace --no-deps` — fix any doc warnings (e.g., broken intra-doc links from module path changes).
3. Run `find crates/smelt-core/src/manifest/ crates/smelt-core/src/git/cli/ -name '*.rs' -exec wc -l {} +` — confirm all files under 500L.
4. Run `cargo test --workspace` — confirm 290+ tests, 0 failures.
5. Verify flat files are gone: `test ! -f crates/smelt-core/src/manifest.rs && test ! -f crates/smelt-core/src/git/cli.rs`.

## Must-Haves

- [ ] `cargo clippy --workspace` clean (zero warnings)
- [ ] `cargo doc --workspace --no-deps` clean (zero warnings)
- [ ] No file in `manifest/` or `git/cli/` exceeds 500 lines
- [ ] `cargo test --workspace` 290+ tests, 0 failures
- [ ] `manifest.rs` and `git/cli.rs` flat files no longer exist

## Verification

- `cargo clippy --workspace 2>&1 | grep -c 'warning'` — 0
- `cargo doc --workspace --no-deps 2>&1 | grep -c 'warning'` — 0
- `cargo test --workspace` — 290+ pass
- Line count script — all under 500L
- File existence check — flat files gone

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Standard Rust toolchain commands
- Failure state exposed: None

## Inputs

- T01 output: decomposed `manifest/` directory
- T02 output: decomposed `git/cli/` directory

## Expected Output

- Clean verification pass — no new files, only potential minor fixes to files from T01/T02
- All S01 success criteria verified: files under 500L, all tests pass, clippy/doc clean, flat files gone
