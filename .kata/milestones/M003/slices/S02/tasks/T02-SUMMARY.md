---
id: T02
parent: S02
milestone: M003
provides:
  - should_create_pr() guard function in run.rs — testable pub(crate) fn covering all 8 combinations of (no_pr, no_changes, forge)
  - Phase 9 block in exec_future — reads token from env, constructs GitHubForge, calls create_pr(), persists pr_url/pr_number to RunState, prints "PR created: <url>"
  - "── Forge ──" section in print_execution_plan() — shown only when manifest.forge.is_some()
  - examples/job-manifest-forge.toml — complete forge manifest fixture; passes --dry-run
  - test_should_create_pr_guard — 8-combination unit test in run.rs
  - test_dry_run_with_forge_shows_forge_section — integration test in dry_run.rs
  - test_dry_run_no_pr_flag_accepted — integration test in dry_run.rs
key_files:
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/dry_run.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
  - examples/job-manifest-forge.toml
key_decisions:
  - GitHubForge::new() returns Result<Self> not Self — Phase 9 block calls .with_context() after ::new() to tag the error site
  - forge feature is unconditionally enabled in smelt-cli (via smelt-core dep features = ["forge"]) — no #[cfg(feature = "forge")] guards needed in run.rs; all types are always available
  - Phase 9 is guarded by should_create_pr() before any env-var or API access — ensures --no-pr and no_changes short-circuit cleanly with no side effects
patterns_established:
  - Token never printed — only forge_cfg.token_env (the env var name) appears in error messages; the token value stays in the local binding and is consumed by GitHubForge::new()
observability_surfaces:
  - "Creating PR: <head> → <base>..." printed to stderr before the API call
  - "PR created: <url>" printed to stderr on success
  - "env var <TOKEN_ENV> not set — required for PR creation (forge.token_env)" on missing token
  - "Phase 9: failed to create GitHub PR" anyhow context tags the call site in error chain
  - pr_url and pr_number persisted to .smelt/run-state.toml after PR created — inspect with: cat .smelt/run-state.toml | grep pr_
duration: 25min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Wire Phase 9 PR creation and dry-run forge display

**Phase 9 inserted into exec_future: GitHubForge.create_pr() called after result collection, pr_url/pr_number persisted to RunState, and ── Forge ── section added to dry-run output — all 12 dry_run + 11 unit tests pass**

## What Happened

Added `should_create_pr(no_pr, no_changes, forge)` as a `pub(crate)` free function above `execute()`. This is the single guard controlling Phase 9 — returns false when any of: `no_pr=true`, `no_changes=true`, `forge=None`. All 8 combinations tested.

Phase 9 block inserted in `exec_future` after the `collect_result` binding. Flow: read `token_env` from env (bail with named-var error if missing) → `GitHubForge::new(token)?` → format title/body → `eprintln!("Creating PR: ...")` → `create_pr(...)` → write `pr_url`/`pr_number` to `monitor.state` → `monitor.write()` → `eprintln!("PR created: ...")`. The `?` operator in `exec_future` propagates to `ExecOutcome::Completed(Err(e))` which the outer `match outcome` arm sets `JobPhase::Failed` for — correct failure path.

`print_execution_plan()` updated with a `── Forge ──` section rendered only when `manifest.forge.is_some()`. Shows provider, repo, token_env, and a hint about `--no-pr`.

Two additional fixes discovered during compilation: `status.rs` test helper had a `RunState` literal missing the new `pr_url`/`pr_number` fields (added `None`); `docker_lifecycle.rs` test helper had a `JobManifest` literal missing `forge` (added `forge: None`). Both were pre-existing gaps from T01 that needed patching when tests compiled with dev-dependencies.

`examples/job-manifest-forge.toml` created as documentation and integration test fixture.

## Verification

- `cargo test -p smelt-cli --lib` — 11/11 pass including `test_should_create_pr_guard` covering all 8 guard combinations
- `cargo test -p smelt-cli --test dry_run` — 12/12 pass including `test_dry_run_with_forge_shows_forge_section` and `test_dry_run_no_pr_flag_accepted`
- `cargo run --bin smelt -- run examples/job-manifest-forge.toml --dry-run` — prints `── Forge ──` section with github/owner/my-repo/GITHUB_TOKEN/(use --no-pr to skip PR creation)
- `cargo test --workspace` — 124 passed, 0 failed (all smelt-core + smelt-cli tests)

## Diagnostics

After a live run with `[forge]` configured:
- `cat .smelt/run-state.toml | grep pr_` — shows `pr_url` and `pr_number` once PR is created
- Missing token error: `"env var GITHUB_TOKEN not set — required for PR creation (forge.token_env)"` on stderr
- PR API failure: `"Error: Phase 9: failed to create GitHub PR: <octocrab error>"` on stderr via anyhow chain

## Deviations

- `GitHubForge::new()` returns `Result<Self>` not `Self` (the task plan's code snippet used it as infallible). Handled by adding `.with_context(|| "Phase 9: failed to initialise GitHub forge client")?` after `::new(token)`.
- No `#[cfg(feature = "forge")]` guards needed — the forge feature is always enabled in smelt-cli. The task plan included these guards, which produced `unexpected_cfg` warnings. Removed entirely.
- Fixed two pre-existing compilation errors in test helpers (`status.rs` and `docker_lifecycle.rs`) caused by T01 adding fields to `RunState` and `JobManifest` — not in scope but blocking test compilation.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/run.rs` — should_create_pr() guard, Phase 9 block, ── Forge ── section, test_should_create_pr_guard
- `crates/smelt-cli/tests/dry_run.rs` — test_dry_run_with_forge_shows_forge_section, test_dry_run_no_pr_flag_accepted
- `crates/smelt-cli/src/commands/status.rs` — added pr_url/pr_number: None to RunState test literal
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added forge: None to JobManifest test literal
- `examples/job-manifest-forge.toml` — forge manifest fixture with [forge] section
