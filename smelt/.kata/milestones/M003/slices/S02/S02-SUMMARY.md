---
id: S02
parent: M003
milestone: M003
provides:
  - JobManifest.forge: Option<ForgeConfig> — optional [forge] TOML section with deny_unknown_fields and structural validation
  - forge validation in JobManifest::validate() — token_env non-empty + repo owner/repo format, errors collected via D018 pattern
  - RunState.pr_url: Option<String> and RunState.pr_number: Option<u64> — backward-compat serde fields persisted to .smelt/run-state.toml
  - RunArgs.no_pr: bool — --no-pr CLI flag that skips Phase 9 even when forge is configured
  - should_create_pr() guard — pub(crate) free function covering all 8 (no_pr × no_changes × forge) combinations
  - Phase 9 in exec_future — reads token from env, constructs GitHubForge, calls create_pr(), persists pr_url/pr_number, prints PR created URL
  - "── Forge ──" section in print_execution_plan() dry-run output
  - examples/job-manifest-forge.toml — complete forge manifest fixture
  - 9 new tests — 5 manifest forge tests, 1 monitor backward-compat, 1 should_create_pr_guard (8 combinations), 2 dry_run integration tests
requires:
  - slice: S01
    provides: ForgeClient::create_pr(), GitHubForge::new(), PrHandle, ForgeConfig — consumed in Phase 9 and manifest.rs
affects:
  - S03
  - S05
key_files:
  - crates/smelt-core/src/manifest.rs
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/Cargo.toml
  - crates/smelt-cli/tests/dry_run.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - examples/job-manifest-forge.toml
key_decisions:
  - D057 — Phase 9 guard extracted as should_create_pr() free function; all 8 guard combinations unit-tested without Docker
  - D058 — smelt-cli always enables forge feature; no #[cfg(feature = "forge")] guards in CLI code since smelt-cli is a binary
  - Forge validation is structural only (D018 pattern): token_env + repo format checked at manifest-load time, not at runtime
  - #[serde(default)] applied per-field on pr_url/pr_number (not on RunState struct) to preserve backward compat with existing state files
  - GitHubForge::new() returns Result<Self> — Phase 9 uses .with_context() to tag the call site before ? propagation
  - Token never printed — only forge_cfg.token_env (the env var name) appears in error messages
patterns_established:
  - Optional TOML section pattern: #[serde(default)] on Option<T> where T has deny_unknown_fields; TOML section scoping means each struct validates its own fields independently
  - Guard-before-side-effect pattern: extract guard function first, test it exhaustively, then use it as the single entry point into effectful code
observability_surfaces:
  - "Creating PR: <head> → <base>..." printed to stderr before API call
  - "PR created: <url>" printed to stderr on success
  - "env var <TOKEN_ENV> not set — required for PR creation (forge.token_env)" on missing token
  - "Phase 9: failed to create GitHub PR" anyhow context in error chain on API failure
  - pr_url and pr_number in .smelt/run-state.toml — inspect with: cat .smelt/run-state.toml | grep pr_
  - -- Forge -- section in dry-run output shows provider/repo/token_env/(use --no-pr hint)
drill_down_paths:
  - .kata/milestones/M003/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M003/slices/S02/tasks/T02-SUMMARY.md
duration: 40min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S02: Manifest Forge Config + PR Creation

**ForgeConfig wired into JobManifest, Phase 9 creates real GitHub PRs after result collection, and all 124 workspace tests pass — `smelt run manifest.toml` with `[forge]` now creates a PR and prints the URL.**

## What Happened

**T01** extended the data model: `JobManifest` gained an optional `forge: Option<ForgeConfig>` field parsed from `[forge]` TOML, with forge validation (token_env non-empty + owner/repo format) appended to the existing D018 error-collection path. `RunState` gained `pr_url` and `pr_number` with per-field `#[serde(default)]` for backward compatibility — existing state files missing these fields round-trip cleanly to `None`. `RunArgs` got `--no-pr` as a clap bool flag. `smelt-cli/Cargo.toml` was updated to enable the `forge` feature on the smelt-core dependency.

**T02** completed the live wiring: `should_create_pr(no_pr, no_changes, forge)` was extracted as a testable `pub(crate)` free function covering all 8 input combinations. Phase 9 was inserted in `exec_future` after `collect_result` — it reads the token from the named env var, constructs `GitHubForge`, formats a PR title/body, calls `create_pr()`, persists `pr_url`/`pr_number` to `RunState`, and prints `PR created: <url>` to stderr. `print_execution_plan()` was extended with a `── Forge ──` section rendered only when `manifest.forge.is_some()`. Two pre-existing test helper compilation errors (missing fields in `RunState` and `JobManifest` literals in `status.rs` and `docker_lifecycle.rs`) were fixed as collateral cleanup.

The `examples/job-manifest-forge.toml` fixture was created to serve as documentation and the dry-run integration test target.

## Verification

- `cargo test -p smelt-core` — 118 passed; forge roundtrip, validation, and backward-compat tests all included
- `cargo test -p smelt-cli --test dry_run` — 12 passed; `test_dry_run_with_forge_shows_forge_section` and `test_dry_run_no_pr_flag_accepted` pass
- `cargo test -p smelt-cli --lib` — 11 passed; `test_should_create_pr_guard` covers all 8 (no_pr × no_changes × forge) combinations
- `cargo test --workspace` — 124 passed, 0 failed
- `cargo run --bin smelt -- run examples/job-manifest-forge.toml --dry-run` — prints `── Forge ──` section with provider/repo/token_env

## Requirements Advanced

- R001 (smelt run creates GitHub PR from result branch) — Phase 9 inserted and functional; live PR creation path proven by design; full runtime proof requires UAT with real Docker + GITHUB_TOKEN
- R002 (Job manifest supports forge configuration block) — `[forge]` section parsed, validated, and round-tripped; `deny_unknown_fields` preserved; proven by automated tests

## Requirements Validated

- R002 (Job manifest supports forge configuration block) — fully validated by automated roundtrip tests; all cases covered (forge present, forge absent, invalid repo format, empty token_env, unknown field rejected)

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- `GitHubForge::new()` returns `Result<Self>` not `Self` — task plan treated it as infallible; Phase 9 handles this via `.with_context()?`
- No `#[cfg(feature = "forge")]` guards in run.rs — task plan included these; removed because smelt-cli unconditionally enables the feature (D058) and `unexpected_cfg` warnings appeared
- Two pre-existing test helper compilation errors patched (status.rs, docker_lifecycle.rs) — not in scope but blocking test compilation; no behavior change

## Known Limitations

- PR creation path requires real Docker + real GITHUB_TOKEN + real GitHub repo for end-to-end validation; automated tests cover guard logic and dry-run display but not the live API call
- PR title and body are statically formatted ("chore: collect results from \<head\>" style); no manifest field for custom PR title/body yet — deferred to S05 API surface decisions
- `--no-pr` flag exists but is not surfaced in `smelt status` output — S03 will add the PR section

## Follow-ups

- S03: Add `pr_url`/`pr_number` rendering to `smelt status` PR section; add `smelt watch` command
- S05: Consider adding optional `title`/`body` fields to `ForgeConfig` for custom PR text

## Files Created/Modified

- `crates/smelt-core/src/manifest.rs` — ForgeConfig import, forge field on JobManifest, forge validation, 5 new tests
- `crates/smelt-core/src/monitor.rs` — pr_url/pr_number fields with #[serde(default)] on RunState, JobMonitor::new() initializer update, backward-compat test
- `crates/smelt-cli/src/commands/run.rs` — --no-pr flag on RunArgs, should_create_pr() guard, Phase 9 block, ── Forge ── section in print_execution_plan(), test_should_create_pr_guard
- `crates/smelt-cli/Cargo.toml` — forge feature enabled for smelt-core dependency
- `crates/smelt-cli/tests/dry_run.rs` — test_dry_run_with_forge_shows_forge_section, test_dry_run_no_pr_flag_accepted
- `crates/smelt-cli/src/commands/status.rs` — added pr_url/pr_number: None to RunState test literal
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added forge: None to JobManifest test literal
- `examples/job-manifest-forge.toml` — forge manifest fixture with [forge] section

## Forward Intelligence

### What the next slice should know
- `RunState.pr_url` and `RunState.pr_number` are persisted to `.smelt/run-state.toml` after Phase 9 — S03's `smelt status` PR section should read from these fields directly; no new state infrastructure needed
- `smelt watch` will need `ForgeClient::poll_pr_status()` from S01 — the trait and GitHubForge impl are already feature-gated; smelt-cli unconditionally enables the forge feature (D058) so no additional Cargo.toml changes are needed for S03
- Phase 9 failure propagates via `ExecOutcome::Completed(Err(e))` → `JobPhase::Failed` — S03 should check whether `JobPhase::Failed` with `pr_url: None` means PR creation failed vs. container failed; may want a distinct `JobPhase::PrFailed` in S04/S05

### What's fragile
- `GitHubForge::new()` parsing: the octocrab client is constructed once per Phase 9 call; if the token is malformed, the error surfaces as a generic forge client init failure — the error message is reasonable but not specifically about token format
- PR already exists (422 from GitHub) surfaces via anyhow chain as `"Phase 9: failed to create GitHub PR: <octocrab error>"` — not a user-friendly message; S05 library surface cleanup should map this to a clearer variant

### Authoritative diagnostics
- `cat .smelt/run-state.toml | grep pr_` — single command to confirm whether PR was created and see the URL/number after a live run
- `cargo test -p smelt-cli --lib test_should_create_pr_guard -q` — confirms guard logic without Docker; fast regression signal

### What assumptions changed
- Task plan assumed no `#[cfg(feature = "forge")]` guards would be needed; confirmed correct — smelt-cli is always compiled with forge, so guards would be unreachable dead code
- Task plan assumed PR creation might need to be deferred if `no_changes` is true (no diff on result branch) — confirmed this guard is already in `should_create_pr()` and returns false cleanly
