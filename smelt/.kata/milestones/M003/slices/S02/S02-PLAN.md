# S02: Manifest Forge Config + PR Creation

**Goal:** Wire `GitHubForge::create_pr()` into the `smelt run` lifecycle — extend `JobManifest` with an optional `[forge]` section, persist PR info to `RunState`, and insert Phase 9 that creates a real GitHub PR after result collection.
**Demo:** `smelt run manifest.toml` with a `[forge]` block creates a real GitHub PR and prints `PR created: <url>`; `smelt run --no-pr` skips creation even when forge is configured; `smelt run` without `[forge]` is completely unchanged.

## Must-Haves

- `JobManifest.forge: Option<ForgeConfig>` parsed from `[forge]` section; `#[serde(default)]`; `deny_unknown_fields` preserved
- `manifest.validate()` checks forge fields: `repo` must be `owner/repo` format, `token_env` must be non-empty; errors collected into existing `errors` vec (D018)
- `RunState.pr_url: Option<String>` and `RunState.pr_number: Option<u64>` with `#[serde(default)]` on each field; existing state files without these fields round-trip cleanly
- `smelt run --no-pr` flag: bool on `RunArgs`; when set, Phase 9 is skipped even if `[forge]` is present
- Phase 9 inside `exec_future`, after `collect_result` binding: guarded by `!collect_result.no_changes && manifest.forge.is_some() && !args.no_pr`; reads token from env, constructs `GitHubForge`, calls `create_pr()`, writes `pr_url`/`pr_number` to `RunState`, prints `PR created: <url>` to stderr
- Clear error when `token_env` is not set at runtime: `"env var GITHUB_TOKEN not set — required for PR creation (forge.token_env)"`
- `print_execution_plan()` (dry-run) shows `── Forge ──` section with provider, repo, and `token_env` name when `manifest.forge.is_some()`
- `smelt-cli/Cargo.toml` enables `features = ["forge"]` on smelt-core dep
- Roundtrip tests: `[forge]` present → fields populated; `[forge]` absent → `forge: None`; unknown forge field → rejected by `deny_unknown_fields`

## Proof Level

- This slice proves: integration
- Real runtime required: yes — Phase 9 calls `GitHubForge::create_pr()` against the live GitHub API; the `--no-pr` and `--dry-run` paths are proven by automated tests; the full PR-creation path requires a real `GITHUB_TOKEN` and is UAT-level
- Human/UAT required: yes — full `smelt run → PR created → url printed` flow requires real Docker + real GITHUB_TOKEN + real GitHub repo

## Verification

- `cargo test -p smelt-core` — all existing manifest + monitor tests still pass; new forge roundtrip and validation tests pass
- `cargo test -p smelt-cli --test dry_run` — `test_dry_run_with_forge_shows_forge_section` and `test_dry_run_no_pr_flag_accepted` pass
- `cargo test -p smelt-cli` (unit tests in run.rs) — `test_should_create_pr_guard` covers all guard combinations
- `cargo build --workspace` — clean compile with `forge` feature enabled in smelt-cli
- `cargo test -p smelt-core` — `test_run_state_backward_compat_no_pr_fields` passes (existing state TOML without pr_url round-trips)

## Observability / Diagnostics

- Runtime signals: `PR created: <url>` printed to stderr on success; error message includes `token_env` variable name on missing-token failure; `SmeltError::Forge { operation: "create_pr", message }` propagates via anyhow for unexpected API failures
- Inspection surfaces: `RunState.pr_url` and `RunState.pr_number` in `.smelt/run-state.toml` — a future agent can read the state file to find the PR URL after a run; `smelt status` will display it in S03
- Failure visibility: missing token → bail with env var name; PR already exists → octocrab 422 maps to `SmeltError::Forge` with message containing "already exists" — surfaces via anyhow error chain on stderr
- Redaction constraints: `GITHUB_TOKEN` value must never be printed; only the `token_env` name (e.g. `"GITHUB_TOKEN"`) is safe to include in error messages

## Integration Closure

- Upstream surfaces consumed: `smelt_core::forge::{ForgeConfig, GitHubForge, ForgeClient, PrHandle}` (from S01); `manifest.rs` JobManifest struct; `monitor.rs` RunState; `run.rs` exec_future structure
- New wiring introduced in this slice: Phase 9 inside `exec_future` in `run.rs`; `features = ["forge"]` in `smelt-cli/Cargo.toml`; `forge: Option<ForgeConfig>` field on `JobManifest`; `pr_url`/`pr_number` fields on `RunState`
- What remains before the milestone is truly usable end-to-end: S03 (smelt status PR section + smelt watch), S04 (per-job state dirs, smelt init, .assay/.gitignore), S05 (library API), S06 (end-to-end integration proof)

## Tasks

- [x] **T01: Extend JobManifest with forge config and RunState with PR fields** `est:45m`
  - Why: The data model must accept `[forge]` in manifests and persist `pr_url`/`pr_number` in state before Phase 9 can write anything; also establishes `--no-pr` flag and enables the forge feature in smelt-cli
  - Files: `crates/smelt-core/src/manifest.rs`, `crates/smelt-core/src/monitor.rs`, `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/Cargo.toml`
  - Do: Add `use crate::forge::ForgeConfig` import to `manifest.rs`; add `#[serde(default)] pub forge: Option<ForgeConfig>` to `JobManifest` (preserving `deny_unknown_fields` on the struct); add forge validation to `validate()` collecting into existing `errors` vec; add `#[serde(default)] pub pr_url: Option<String>` and `#[serde(default)] pub pr_number: Option<u64>` to `RunState`; add `#[arg(long)] pub no_pr: bool` to `RunArgs`; update smelt-cli dep to `smelt-core = { path = "../smelt-core", features = ["forge"] }`; write roundtrip and validation tests
  - Verify: `cargo test -p smelt-core` passes including new tests; `cargo build --workspace` clean compile
  - Done when: `cargo test -p smelt-core` shows all tests pass; roundtrip test with `[forge]` returns `forge.is_some()`; backward-compat test with old state TOML (no pr_url field) deserializes without error

- [x] **T02: Wire Phase 9 PR creation and dry-run forge display** `est:45m`
  - Why: Closes the slice — inserts the actual PR-creation call into the run lifecycle and makes the dry-run execution plan show forge config, completing R001 and R002
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/dry_run.rs`, `examples/job-manifest-forge.toml`
  - Do: Add `use smelt_core::forge::GitHubForge` import to `run.rs`; extract `pub(crate) fn should_create_pr(no_pr: bool, no_changes: bool, forge: Option<&ForgeConfig>) -> bool` helper; insert Phase 9 inside `exec_future` after `collect_result`: check `should_create_pr`, read token via `std::env::var`, construct `GitHubForge::new(token)`, build PR title/body, call `create_pr()`, write `pr_url`/`pr_number` to `monitor.state`, call `monitor.write()`, `eprintln!("PR created: {url}")`; update `print_execution_plan()` to show `── Forge ──` section when `manifest.forge.is_some()`; add `examples/job-manifest-forge.toml` fixture; add dry_run integration tests; add unit tests for `should_create_pr` guard
  - Verify: `cargo test -p smelt-cli --test dry_run` passes new forge tests; `cargo test -p smelt-cli` passes `test_should_create_pr_guard`; dry-run with forge manifest prints forge section
  - Done when: `cargo test --workspace` clean; dry-run with `examples/job-manifest-forge.toml` prints `── Forge ──` section; `should_create_pr` unit tests cover all 8 combinations of (no_pr, no_changes, forge_some)

## Files Likely Touched

- `crates/smelt-core/src/manifest.rs`
- `crates/smelt-core/src/monitor.rs`
- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-cli/Cargo.toml`
- `crates/smelt-cli/tests/dry_run.rs`
- `examples/job-manifest-forge.toml`
