---
id: T01
parent: S02
milestone: M003
provides:
  - JobManifest.forge: Option<ForgeConfig> — optional [forge] section in TOML manifests
  - forge validation in JobManifest::validate() — token_env non-empty + repo owner/repo format
  - RunState.pr_url and RunState.pr_number — PR fields with #[serde(default)] for backward compat
  - RunArgs.no_pr: bool — clap --no-pr flag to skip PR creation
  - smelt-cli forge feature enabled — smelt-core compiled with forge feature in smelt-cli
  - 6 new tests — 5 manifest forge tests + 1 monitor backward-compat test
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - Forge validation is structural only (D018): validates token_env non-empty and repo format but does NOT check whether the env var is actually set at manifest-load time
  - #[serde(default)] applied per-field (not on the struct) on pr_url and pr_number to preserve backward compat with existing run-state.toml files
patterns_established:
  - Optional TOML section pattern: #[serde(default)] on Option<T> field where T has #[serde(deny_unknown_fields)] — ForgeConfig's deny_unknown_fields is independent of JobManifest's
observability_surfaces:
  - RunState.pr_url and RunState.pr_number written to .smelt/run-state.toml — inspect via `cat .smelt/run-state.toml | grep pr_url` after a run once T02 writes the values
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Extend JobManifest with forge config and RunState with PR fields

**Added ForgeConfig field to JobManifest, PR fields to RunState with backward-compat serde defaults, --no-pr flag to RunArgs, and forge feature enabled in smelt-cli — all 118 tests pass.**

## What Happened

Added `use crate::forge::ForgeConfig` import and `#[serde(default)] pub forge: Option<ForgeConfig>` to `JobManifest`. The `#[serde(deny_unknown_fields)]` on `JobManifest` coexists cleanly with `ForgeConfig`'s own `deny_unknown_fields` — TOML's section-scoped parsing means each struct validates its own fields independently.

Added forge validation in `JobManifest::validate()` after the merge section: checks `token_env` is non-empty and `repo` is in `owner/name` format. Both use the same push-to-errors pattern as all other validation (D018 — collect all errors, don't return early).

Added `pr_url: Option<String>` and `pr_number: Option<u64>` to `RunState` with `#[serde(default)]` on each individual field (not on the struct). Also updated `JobMonitor::new()` to initialize both fields to `None` to satisfy the struct literal. The per-field default pattern ensures existing state files that lack these keys deserialize successfully.

Added `#[arg(long)] pub no_pr: bool` to `RunArgs` in smelt-cli. Clap defaults bool args to false — no extra annotation needed.

Updated smelt-cli's `Cargo.toml` to `smelt-core = { path = "../smelt-core", features = ["forge"] }` so the octocrab-backed `GitHubForge` is compiled into the CLI binary.

Wrote 5 manifest tests (parse with forge, parse without forge, invalid repo format, empty token_env, deny_unknown_fields) and 1 monitor backward-compat test (old TOML without pr_url/pr_number deserializes to None).

## Verification

- `cargo test -p smelt-core -- manifest forge` → 34 tests pass (5 new forge tests included)
- `cargo test -p smelt-core -- monitor` → 13 tests pass (backward-compat test included)
- `cargo test -p smelt-core` → 118 tests pass, 0 failed
- `cargo build --workspace` → clean compile with forge feature enabled

## Diagnostics

PR fields in RunState: once T02 writes values, inspect via `cat .smelt/run-state.toml | grep pr_` after a run. Forge validation errors surface via `SmeltError::Manifest` path — printed to stderr with all errors together (same path as all other validation failures).

## Deviations

Minor: test assertion for `test_validate_forge_invalid_repo_format` checked `"owner/repo format"` but the actual error message uses backtick quoting (`` `owner/repo` format ``). Changed assertion to `msg.contains("owner/repo")` which correctly captures the intent.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — added ForgeConfig import, forge field on JobManifest, forge validation in validate(), 5 new tests
- `crates/smelt-core/src/monitor.rs` — added pr_url/pr_number fields with #[serde(default)] to RunState, updated JobMonitor::new() initializer, added backward-compat test
- `crates/smelt-cli/src/commands/run.rs` — added --no-pr flag to RunArgs
- `crates/smelt-cli/Cargo.toml` — enabled forge feature for smelt-core dependency
