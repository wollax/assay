---
id: S03
milestone: M009
status: ready
---

# S03: Large file decomposition — Context

## Goal

Decompose `run.rs` (755L), `ssh.rs` (978L), and serve `tests.rs` (1322L) into focused submodules along natural seams, fix the 16 pre-existing clippy warnings in smelt-core, with all 286+ tests passing and `cargo clippy --workspace -- -D warnings` clean.

## Why this Slice

These three files have grown past the 500-line threshold (D126) over 8 milestones of feature delivery. Each mixes multiple concerns — `run.rs` combines CLI args, provider dispatch, 9 execution phases, dry-run output, and helper functions; `ssh.rs` combines a trait, two implementations, a mock, free functions, and tests; `tests.rs` puts all serve integration tests in one 1322-line file. Decomposing now establishes a clean baseline for future feature work. S01's `deny(missing_docs)` constraint means all new public items in extracted modules must have doc comments.

## Scope

### In Scope

- **run.rs → run/ directory:**
  - `mod.rs` — RunArgs, execute(), thin routing layer (<300 lines target)
  - Extract `run_with_cancellation` phases, `execute_dry_run`, `print_execution_plan`, `ensure_gitignore_assay`, `truncate_spec`, and `AnyProvider` enum into submodules
  - Existing `#[cfg(test)] mod tests` block moves with its associated functions

- **ssh.rs → ssh/ directory:**
  - `mod.rs` — re-exports, module doc
  - `trait.rs` or `client.rs` — `SshClient` trait + `SshOutput` struct
  - `subprocess.rs` — `SubprocessSshClient` impl + `build_ssh_args`/`build_scp_args`
  - `operations.rs` — free functions (`deliver_manifest`, `sync_state_back`, `run_remote_job`)
  - `mock.rs` — `MockSshClient` (currently `#[cfg(test)] pub(crate) mod tests`)

- **tests.rs → tests/ directory (or multiple test files):**
  - Split by existing `// ──` section markers (~6-8 files):
    - Queue tests (4 tests)
    - Dispatch tests (3 tests)
    - Watcher tests (2 tests)
    - HTTP tests (6 tests)
    - Worker config tests (5 tests)
    - Server config tests (3 tests)
    - Smoke test (1 test)
    - SSH dispatch tests (4 tests)
    - TUI test (1 test)

- **Fix 16 pre-existing clippy warnings** in smelt-core (collapsible-if in compose.rs and k8s.rs)

- **Verification:** `cargo test --workspace` passes (286+ tests, 0 failures); `cargo clippy --workspace -- -D warnings` exits 0; `cargo doc --workspace --no-deps` zero warnings

### Out of Scope

- Renaming public API items or changing signatures
- Changing any runtime behavior
- Decomposing files that are under the 500-line threshold
- Adding new tests (only moving existing ones)
- Refactoring logic within the extracted modules (pure move, no rewrites)

## Constraints

- D125: No behavior changes — pure structural refactoring
- D126: Target <500 lines per file (roadmap specifies run.rs <300, ssh.rs <400, tests.rs <500)
- D127: `#![deny(missing_docs)]` on smelt-cli — any new `pub` items in extracted modules need doc comments
- All `pub(crate)` visibility must be preserved — don't widen access
- Run `cargo test --workspace` after each file decomposition step to catch regressions immediately
- The `MockSshClient` is `pub(crate)` inside a `#[cfg(test)]` module — its new location must preserve this test-only visibility

## Integration Points

### Consumes

- S01 output: clean `cargo doc` baseline, `deny(missing_docs)` enforced on smelt-cli
- Existing module structure and `pub(crate)` visibility contracts

### Produces

- `run/` directory with mod.rs <300 lines + submodules
- `ssh/` directory with mod.rs + focused submodules (<400 lines each)
- Multiple test files replacing monolithic `tests.rs` (<500 lines each)
- Clean `cargo clippy --workspace -- -D warnings` (16 pre-existing warnings fixed)

## Open Questions

- None — all grey areas resolved during discuss phase.
