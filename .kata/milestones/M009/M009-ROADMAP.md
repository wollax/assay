# M009: Documentation, Examples & Code Cleanup

**Vision:** Smelt's codebase gets a full documentation, examples, and code quality pass — zero-warning `cargo doc`, `deny(missing_docs)` on both crates, a comprehensive README, documented example manifests, and large files decomposed along natural seams. No behavior changes; all existing tests pass.

## Success Criteria

- `cargo doc --workspace --no-deps` exits 0 with zero warnings
- `cargo test --workspace` passes with zero regressions (286+ tests)
- `cargo clippy --workspace` is clean
- Workspace-level `README.md` exists with project overview, install, usage for all subcommands, and example walkthrough
- `#![deny(missing_docs)]` enforced on smelt-cli (already enforced on smelt-core)
- No stale `#[allow(dead_code)]` annotations in production code
- `run.rs`, `ssh.rs`, and `tests.rs` decomposed into smaller focused modules
- Example manifests have inline documentation explaining every field

## Key Risks / Unknowns

- **`deny(missing_docs)` gap on smelt-cli** — unknown how many public items lack doc comments. Could be large. Mitigate by scouting first, then adding docs systematically.
- **Refactoring regressions** — module moves can break imports and visibility. Mitigate by running full test suite after each refactor step.

## Proof Strategy

- **missing_docs gap** → retire in S01 by enabling the lint and adding all required doc comments; `cargo doc --workspace --no-deps` must exit 0 with zero warnings
- **Refactoring safety** → retire in S03 by running `cargo test --workspace` after each file decomposition; zero regressions

## Verification Classes

- Contract verification: `cargo doc --workspace --no-deps` zero warnings; `cargo test --workspace` all green; `cargo clippy --workspace` clean; no stale `#[allow(dead_code)]`
- Integration verification: none (no behavior changes)
- Operational verification: none
- UAT / human verification: README readability; example manifest clarity

## Milestone Definition of Done

This milestone is complete only when all are true:

- `cargo doc --workspace --no-deps` exits 0 with zero warnings
- `cargo test --workspace` passes (286+ tests, 0 failures)
- `cargo clippy --workspace` is clean
- `README.md` at workspace root covers all subcommands
- `#![deny(missing_docs)]` on smelt-cli compiles clean
- No stale `#[allow(dead_code)]` in production `.rs` files
- `run.rs` < 300 lines, `ssh.rs` < 400 lines, serve `tests.rs` < 500 lines (remainder in focused modules)
- Example manifests have field-level documentation

## Requirement Coverage

- Covers: R040, R041, R042, R043, R044, R045
- Partially covers: none
- Leaves for later: R022 (budget/cost tracking), R026 (tracker integration)
- Orphan risks: none

## Slices

- [x] **S01: cargo doc zero-warning + deny(missing_docs) on smelt-cli** `risk:high` `depends:[]`
  > After this: `cargo doc --workspace --no-deps` exits 0 with zero warnings; `#![deny(missing_docs)]` enabled on smelt-cli and compiles clean; all stale `#[allow(dead_code)]` annotations removed or justified.

- [x] **S02: README + example manifest documentation** `risk:low` `depends:[]`
  > After this: workspace-level `README.md` with project overview, install, all subcommand usage, and example walkthrough; example manifests have inline field-level comments.

- [ ] **S03: Large file decomposition** `risk:medium` `depends:[S01]`
  > After this: `run.rs` decomposed (phases extracted to modules); `ssh.rs` decomposed (trait, impl, free functions, mock in separate modules); serve `tests.rs` decomposed by feature area; all 286+ tests still pass.

## Boundary Map

### S01 → S03

Produces:
- Clean `cargo doc` baseline — no warnings, all public items documented
- Stale annotation cleanup — accurate `#[allow]` state for S03 to inherit
- `deny(missing_docs)` on smelt-cli — S03 module splits must maintain doc coverage

Consumes:
- nothing (first slice, parallel with S02)

### S02 (independent — no downstream consumers)

Produces:
- `README.md` at workspace root
- Updated example manifests with field-level comments

Consumes:
- nothing (independent of S01 and S03)

### S03 (final)

Produces:
- `run.rs` split into `run/mod.rs` + `run/phases.rs` (or similar)
- `ssh.rs` split into `ssh/mod.rs` + `ssh/client.rs` + `ssh/mock.rs` + `ssh/operations.rs`
- `tests.rs` split into `tests/mod.rs` + `tests/dispatch.rs` + `tests/ssh.rs` + `tests/queue.rs`
- All tests still passing

Consumes from S01:
- Clean doc/lint baseline — refactored modules must compile under `deny(missing_docs)`
