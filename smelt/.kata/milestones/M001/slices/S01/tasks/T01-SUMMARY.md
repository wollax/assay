---
id: T01
parent: S01
milestone: M001
provides:
  - Clean workspace with only git/ and error modules in smelt-core
  - Stub `smelt run` CLI subcommand
key_files:
  - crates/smelt-core/src/lib.rs
  - crates/smelt-core/src/git/mod.rs
  - crates/smelt-core/src/git/cli.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/commands/run.rs
key_decisions:
  - Moved GitWorktreeEntry and parse_porcelain from worktree/ into git/mod.rs to keep git module self-contained after worktree deletion
  - Stripped SmeltError to only git-relevant variants (GitNotFound, NotAGitRepo, GitExecution, Io, MergeConflict); new variants added in T03
  - Removed all unused workspace deps (genai, petgraph, globset, similar, dialoguer, libc, indicatif, comfy-table, console, chrono, serde, toml, serde_json, tokio-util) from workspace Cargo.toml; kept serde/toml for T02
patterns_established:
  - Workspace deps only declared when consumed by at least one crate
observability_surfaces:
  - none (gutting task — no runtime behavior added)
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Gut v0.1.0 modules and stabilize workspace

**Deleted ~9400 lines of v0.1.0 orchestration code, retained git/ module with all 32 tests passing, and created stub `smelt run` CLI.**

## What Happened

Removed all v0.1.0 modules from smelt-core: `ai/`, `merge/`, `orchestrate/`, `session/`, `summary/`, `worktree/`, and `init.rs`. The `git/` module was retained as reusable infrastructure. `GitWorktreeEntry` and `parse_porcelain` (previously in `worktree/state.rs`) were moved into `git/mod.rs` since `git/cli.rs` depends on them for worktree operations.

Rewrote `lib.rs` to export only `git` and `error` modules. Stripped `SmeltError` to the five variants used by the git module. Removed all old CLI subcommands (init, worktree, session, merge, orchestrate, summary) and their test files. Created a minimal `smelt run <manifest> [--dry-run]` stub. Cleaned both crate Cargo.toml files and the workspace Cargo.toml to remove ~15 unused dependencies.

## Verification

- `cargo build --workspace` — zero errors, zero warnings ✅
- `cargo test --workspace` — 32 passed, 0 failed ✅ (all git module tests)
- `cargo run -- run examples/agent-manifest.toml --dry-run` — exits with stub message ✅

## Diagnostics

None — this was a deletion/cleanup task with no new runtime behavior.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/lib.rs` — rewritten to export only git and error modules
- `crates/smelt-core/src/error.rs` — stripped to 5 git-relevant variants
- `crates/smelt-core/src/git/mod.rs` — absorbed GitWorktreeEntry + parse_porcelain from deleted worktree module
- `crates/smelt-core/src/git/cli.rs` — updated import path for GitWorktreeEntry
- `crates/smelt-core/Cargo.toml` — removed 10 unused deps (serde, toml, chrono, dialoguer, libc, globset, genai, serde_json, petgraph, tokio-util)
- `crates/smelt-cli/src/main.rs` — rewritten with single `run` subcommand
- `crates/smelt-cli/src/commands/mod.rs` — rewritten to export only `run`
- `crates/smelt-cli/src/commands/run.rs` — new stub run subcommand
- `crates/smelt-cli/Cargo.toml` — removed 8 unused deps (console, dialoguer, which, comfy-table, serde_json, similar, indicatif, tokio-util, tokio)
- `Cargo.toml` — removed 13 unused workspace deps
- Deleted: `crates/smelt-core/src/{ai,merge,orchestrate,session,summary,worktree}/`, `init.rs`, 6 old CLI command files, 6 old CLI test files
