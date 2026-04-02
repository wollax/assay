---
id: S01
parent: M001
milestone: M001
provides:
  - "JobManifest type system with strict TOML parsing (deny_unknown_fields), two-phase load+validate pipeline, credential resolution"
  - "RuntimeProvider trait with async lifecycle methods (provision/exec/collect/teardown) using RPITIT"
  - "SmeltError enum with Manifest, Provider, Credential, Config, Git, Io variants and convenience constructors"
  - "SmeltConfig loader from .smelt/config.toml with sensible defaults"
  - "`smelt run --dry-run` CLI command printing structured execution plan with credential redaction"
  - "Example manifests (valid and invalid) for downstream testing"
requires:
  - slice: none
    provides: first slice
affects:
  - S02
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/provider.rs
  - crates/smelt-core/src/error.rs
  - crates/smelt-core/src/config.rs
  - crates/smelt-core/src/lib.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
  - examples/job-manifest.toml
  - examples/bad-manifest.toml
key_decisions:
  - "deny_unknown_fields on all 6 manifest structs for strict schema enforcement"
  - "RPITIT (return-position impl trait in trait) for RuntimeProvider instead of async_trait macro — Rust 2024 edition supports this natively"
  - "Validation collects all errors before returning (not fail-fast) so users see every issue at once"
  - "SmeltConfig returns defaults when .smelt/config.toml is missing (non-fatal)"
  - "Credential resolution returns status enum (Resolved/Missing) — never exposes actual values"
patterns_established:
  - "Two-phase manifest pipeline: from_str() for TOML deserialization, then validate() for semantic checks"
  - "SmeltError convenience constructors for ergonomic error creation"
  - "CLI dry-run pattern: load → validate → resolve credentials → print plan"
  - "Integration tests use workspace_root() helper via CARGO_MANIFEST_DIR for portable path resolution"
observability_surfaces:
  - "`smelt run --dry-run` prints structured execution plan showing all manifest sections and credential resolution status"
  - "SmeltError variants carry structured context (operation, field, path, provider) for machine-readable error inspection"
  - "Validation errors list every field violation with field path and specific constraint message"
drill_down_paths:
  - .kata/milestones/M001/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M001/slices/S01/tasks/T03-SUMMARY.md
  - .kata/milestones/M001/slices/S01/tasks/T04-SUMMARY.md
duration: 60m
verification_result: passed
completed_at: 2026-03-17
---

# S01: Scaffold, Manifest & Dry-Run CLI

**Gutted ~9400 lines of v0.1.0 code, built manifest type system with strict validation, RuntimeProvider trait, SmeltConfig, and `smelt run --dry-run` CLI with 71 passing tests.**

## What Happened

T01 deleted all v0.1.0 orchestration modules (ai/, merge/, orchestrate/, session/, summary/, worktree/, init.rs) and ~15 unused dependencies. The git/ module was retained with `GitWorktreeEntry` and `parse_porcelain` absorbed from the deleted worktree module. A stub `smelt run` CLI subcommand was created.

T02 built the manifest type system in `manifest.rs`: 6 serde structs (`JobManifest`, `JobMeta`, `Environment`, `CredentialConfig`, `SessionDef`, `MergeConfig`) all using `#[serde(deny_unknown_fields)]`. The two-phase pipeline — `load()`/`from_str()` for deserialization then `validate()` for semantic checks — catches both TOML schema errors and logical errors (duplicate sessions, cycles, invalid references). Credential resolution checks env vars and reports Resolved/Missing status without exposing values. 17 unit tests cover every validation rule.

T03 defined `RuntimeProvider` as an async trait using RPITIT with `Send + Sync` bounds. Opaque types (`ContainerId`, `ExecHandle`, `CollectResult`) keep the provider contract clean. `SmeltError` was expanded from 5 to 8 variants with convenience constructors. `SmeltConfig` loads from `.smelt/config.toml` with sensible defaults when the file is missing. 9 config tests added.

T04 wired everything into the CLI. `smelt run <manifest> --dry-run` runs the full pipeline: load → validate → resolve credentials → print structured execution plan. 10 integration tests verify happy path, validation errors, credential resolution, and secret redaction. Without `--dry-run`, the command exits 1 with a placeholder message for S02.

## Verification

- `cargo build --workspace` — zero errors, zero warnings ✅
- `cargo test -p smelt-core` — 58 tests pass (32 git + 17 manifest + 9 config) ✅
- `cargo test -p smelt-cli` — 13 tests pass (3 unit + 10 integration) ✅
- `cargo test --workspace` — 71 total tests pass ✅
- `cargo run -- run examples/job-manifest.toml --dry-run` — prints structured execution plan with all sections ✅
- `cargo run -- run examples/bad-manifest.toml --dry-run` — exits 1 with 7 specific validation errors ✅

## Deviations

None.

## Known Limitations

- `smelt run` without `--dry-run` exits 1 with placeholder — Docker execution is S02
- `assert_cmd::Command::cargo_bin` emits a deprecation warning — upstream issue, no functional impact
- No `.kata/REQUIREMENTS.md` exists — requirements are tracked informally via milestone roadmap

## Follow-ups

None — all planned work completed as specified.

## Files Created/Modified

- `crates/smelt-core/src/lib.rs` — rewritten to export git, error, manifest, provider, config modules
- `crates/smelt-core/src/manifest.rs` — new: 6 serde structs, load/validate/resolve pipeline, 17 tests
- `crates/smelt-core/src/provider.rs` — new: RuntimeProvider trait, ContainerId, ExecHandle, CollectResult
- `crates/smelt-core/src/error.rs` — rewritten: 8 variants with convenience constructors
- `crates/smelt-core/src/config.rs` — new: SmeltConfig TOML loader with defaults, 9 tests
- `crates/smelt-core/src/git/mod.rs` — absorbed GitWorktreeEntry + parse_porcelain from deleted worktree module
- `crates/smelt-core/src/git/cli.rs` — updated import path
- `crates/smelt-core/Cargo.toml` — cleaned deps, added serde/toml
- `crates/smelt-cli/src/main.rs` — rewritten with single `run` subcommand
- `crates/smelt-cli/src/commands/run.rs` — full dry-run implementation
- `crates/smelt-cli/src/commands/mod.rs` — rewritten to export only run
- `crates/smelt-cli/tests/dry_run.rs` — new: 10 integration tests
- `crates/smelt-cli/Cargo.toml` — cleaned deps, added assert_cmd/predicates
- `Cargo.toml` — removed 13 unused workspace deps
- `examples/job-manifest.toml` — new: valid example manifest
- `examples/bad-manifest.toml` — new: invalid manifest for testing

## Forward Intelligence

### What the next slice should know
- `RuntimeProvider` trait is defined in `provider.rs` with `provision()`, `exec()`, `collect()`, `teardown()` — S02 implements `DockerProvider` against this trait
- `JobManifest` provides `Environment` (image, resources), `CredentialConfig` (env vars to inject), and `SessionDef` (session definitions) — all consumed by DockerProvider
- The manifest pipeline is strict: `deny_unknown_fields` will reject any TOML key not in the struct definitions, so new fields require schema changes
- `ContainerId(String)` is the opaque handle — DockerProvider should store the Docker container ID in it

### What's fragile
- `RuntimeProvider` uses RPITIT instead of `async_trait` — requires Rust 2024 edition. If the workspace edition is downgraded, this will break.
- The `ExecHandle` type is a minimal placeholder (exit_code, output stream fields) — S02 may need to expand it for real streaming

### Authoritative diagnostics
- `cargo run -- run examples/job-manifest.toml --dry-run` — the primary smoke test for the manifest pipeline
- `cargo test -p smelt-core -- manifest` — 17 tests covering every validation rule

### What assumptions changed
- No assumptions changed — S01 executed as planned
