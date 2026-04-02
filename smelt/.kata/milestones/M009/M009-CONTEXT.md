# M009: Documentation, Examples & Code Cleanup — Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. After 8 milestones (M001–M008), the codebase has grown to ~14,500 lines across two crates with Docker, Compose, Kubernetes, and SSH worker pool runtimes, a parallel dispatch daemon, persistent queue, HTTP API, and TUI. The codebase works — 286 tests pass, clippy is clean — but documentation, examples, and internal code organization have accumulated debt.

## Why This Milestone

M001–M008 were feature-delivery focused. The project has no README.md at any level. `cargo doc` fails with an unresolved link warning. `smelt-cli` has no `deny(missing_docs)` enforcement. Several `#[allow(dead_code)]` annotations are stale. Large files (run.rs 755L, ssh.rs 978L, tests.rs 1322L) mix concerns. Example manifests exist but lack a usage guide. This debt makes the codebase harder for contributors (human or agent) to navigate and harder to publish.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Read a comprehensive workspace-level `README.md` that explains what Smelt is, how to install it, and how to use each subcommand
- Run `cargo doc --workspace --no-deps` with zero warnings and zero errors
- Browse well-documented public API surfaces in both `smelt-core` and `smelt-cli`
- Read example manifests with inline commentary explaining every field
- Navigate a cleaner module structure where large files have been decomposed along natural seams

### Entry point / environment

- Entry point: `cargo doc`, `README.md`, example files, code navigation
- Environment: local dev, GitHub repo page, docs.rs (future)
- Live dependencies: none

## Completion Class

- Contract complete means: `cargo doc --workspace --no-deps` zero warnings; `#![deny(missing_docs)]` on smelt-cli compiles; no stale `#[allow(dead_code)]`; README exists
- Integration complete means: not applicable (documentation milestone)
- Operational complete means: not applicable

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `cargo doc --workspace --no-deps` exits 0 with zero warnings
- `cargo test --workspace` still passes with zero regressions
- `cargo clippy --workspace` is clean
- `README.md` exists at workspace root with install, usage, and subcommand documentation
- No stale `#[allow(dead_code)]` annotations remain in production code

## Risks and Unknowns

- **`deny(missing_docs)` on smelt-cli could surface many missing doc comments** — smelt-cli currently has no missing_docs enforcement; the gap could be large. Mitigate by adding docs incrementally in the same slice that enables the lint.
- **Refactoring large files risks regressions** — moving code between modules can break imports, visibility, and test references. Mitigate by running `cargo test --workspace` after each refactor step.
- **`cargo doc` build failure** — the current `cargo doc` fails with a filesystem error (`search.index/path/`), possibly a corrupted doc cache. May need `cargo clean --doc` before building.

## Existing Codebase / Prior Art

- `crates/smelt-core/src/lib.rs` — already has `#![deny(missing_docs)]` and module-level doc comments
- `crates/smelt-cli/src/lib.rs` — no `deny(missing_docs)`, only 4 lines
- `crates/smelt-cli/src/commands/run.rs` — 755 lines, mixes CLI args, Phase 1-9 execution, AnyProvider enum, helper functions
- `crates/smelt-cli/src/serve/ssh.rs` — 978 lines, mixes SshClient trait, SubprocessSshClient, MockSshClient, free functions, tests
- `crates/smelt-cli/src/serve/tests.rs` — 1322 lines, all serve integration tests in one file
- `examples/` — 7 example files exist but no documentation guide
- No `README.md` at workspace root, smelt-core, or smelt-cli level

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R040 — Zero-warning `cargo doc` (new, this milestone)
- R041 — Workspace README with usage documentation (new, this milestone)
- R042 — `deny(missing_docs)` on smelt-cli (new, this milestone)
- R043 — No stale `#[allow]` annotations in production code (new, this milestone)
- R044 — Large file decomposition (new, this milestone)
- R045 — Example manifest documentation (new, this milestone)

## Scope

### In Scope

- Workspace-level `README.md` with project overview, install instructions, subcommand usage, example walkthrough
- Fix `cargo doc` unresolved link warning (`build_ssh_args`)
- Add `#![deny(missing_docs)]` to smelt-cli and add all missing doc comments
- Remove stale `#[allow(dead_code)]` annotations (verify each is actually used or remove the code)
- Decompose `run.rs` (755L), `ssh.rs` (978L), and `tests.rs` (1322L) along natural seams
- Expand example manifests with inline documentation
- Ensure `cargo test --workspace`, `cargo clippy --workspace`, and `cargo doc --workspace --no-deps` are all clean

### Out of Scope / Non-Goals

- New features or behavior changes
- crates.io publishing (separate milestone if desired)
- API reference website or mdBook
- Changelog generation
- CI/CD pipeline setup

## Technical Constraints

- `#![deny(missing_docs)]` must not break the build — add doc comments in the same slice that enables the lint
- Refactoring must preserve all existing public API signatures — no breaking changes
- Module moves must keep `pub(crate)` visibility where it exists today
- All 286+ tests must continue to pass after every slice

## Integration Points

- None — this is a pure documentation and code quality milestone

## Open Questions

- **`cargo doc` filesystem error** — may be a corrupted target/doc cache. `cargo clean --doc` likely fixes it. To be confirmed in S01.
