---
estimated_steps: 5
estimated_files: 4
---

# T02: Wire Phase 9 PR creation and dry-run forge display

**Slice:** S02 — Manifest Forge Config + PR Creation
**Milestone:** M003

## Description

Insert Phase 9 into `exec_future` in `run.rs`: after `ResultCollector::collect()` returns, check the should_create_pr guard, read the GitHub token from env, construct `GitHubForge`, call `create_pr()`, write `pr_url`/`pr_number` to `RunState`, and print `PR created: <url>` to stderr. Also update `print_execution_plan()` to show a `── Forge ──` section. Add an example forge manifest fixture. Write unit + integration tests covering the guard logic and dry-run display.

## Steps

1. **`run.rs` — add import and guard helper**: Add `use smelt_core::forge::{ForgeConfig, GitHubForge};` to the imports at the top of `run.rs`. Extract the guard as a testable `pub(crate)` free function:
   ```rust
   /// Returns true when Phase 9 should attempt PR creation.
   pub(crate) fn should_create_pr(no_pr: bool, no_changes: bool, forge: Option<&ForgeConfig>) -> bool {
       !no_pr && !no_changes && forge.is_some()
   }
   ```
   Place above `execute()`.

2. **`run.rs` — insert Phase 9 inside `exec_future`**: After the `collect_result` binding and before `Ok::<i32, anyhow::Error>(assay_exit)`, add:
   ```rust
   // Phase 9: Create GitHub PR if forge is configured
   if should_create_pr(args.no_pr, collect_result.no_changes, manifest.forge.as_ref()) {
       let forge_cfg = manifest.forge.as_ref().unwrap();
       let token = std::env::var(&forge_cfg.token_env).map_err(|_| {
           anyhow::anyhow!(
               "env var {} not set — required for PR creation (forge.token_env)",
               forge_cfg.token_env
           )
       })?;
       let github = GitHubForge::new(token);
       let job_name = &manifest.job.name;
       let head = &collect_result.branch;
       let base = &manifest.job.base_ref;
       let title = format!("[smelt] {} — {} → {}", job_name, head, base);
       let body = format!(
           "Automated results from smelt job '{job_name}'.\n\nBase: `{base}`"
       );
       eprintln!("Creating PR: {} → {}...", head, base);
       let pr = github
           .create_pr(&forge_cfg.repo, head, base, &title, &body)
           .await
           .with_context(|| "Phase 9: failed to create GitHub PR")?;
       monitor.state.pr_url = Some(pr.url.clone());
       monitor.state.pr_number = Some(pr.number);
       monitor.write().map_err(|e| anyhow::anyhow!("{e}"))?;
       eprintln!("PR created: {}", pr.url);
   }
   ```
   The `?` operator in `exec_future` maps to `Err`, which the outer `match outcome` arm catches as `ExecOutcome::Completed(Err(e))`.

3. **`run.rs` — update `print_execution_plan()`**: After the `── Merge ──` section, add a `── Forge ──` section rendered only when `manifest.forge.is_some()`:
   ```rust
   if let Some(ref forge) = manifest.forge {
       println!("── Forge ──");
       println!("  Provider:    {}", forge.provider);
       println!("  Repo:        {}", forge.repo);
       println!("  Token env:   {}", forge.token_env);
       println!("  (use --no-pr to skip PR creation)");
       println!();
   }
   ```

4. **`examples/job-manifest-forge.toml`**: Create an example manifest with `[forge]` section. Use `job.repo = "."`, `[forge]` with `provider = "github"`, `repo = "owner/my-repo"`, `token_env = "GITHUB_TOKEN"`. Base it on the existing `examples/job-manifest.toml` structure. This serves as both documentation and integration test fixture.

5. **Tests**: 
   - In `run.rs` `#[cfg(test)]` block: add `test_should_create_pr_guard` covering all 8 combinations of (no_pr: bool, no_changes: bool, forge: Option). Key cases: forge=None always returns false; no_pr=true always returns false; no_changes=true always returns false; all false returns true.
   - In `crates/smelt-cli/tests/dry_run.rs`: add `test_dry_run_with_forge_shows_forge_section` using `examples/job-manifest-forge.toml`; assert output contains `── Forge ──`, `github`, `owner/my-repo`, `GITHUB_TOKEN`, `--no-pr`. Add `test_dry_run_no_pr_flag_accepted` using the same forge manifest with `--dry-run --no-pr`; assert success + forge section still shown (--no-pr only affects live runs, not dry-run display).

## Must-Haves

- [ ] `should_create_pr()` guard: returns false when `no_pr=true`, returns false when `no_changes=true`, returns false when `forge=None`, returns true only when all three conditions clear
- [ ] Phase 9 skips gracefully (no panic, no error) when `should_create_pr()` returns false — `--no-pr` flag is the primary automated test path
- [ ] Missing token env var produces error message containing the variable name (e.g. "env var GITHUB_TOKEN not set")
- [ ] `pr_url` and `pr_number` written to monitor state and persisted to disk when PR is created
- [ ] `print_execution_plan()` shows `── Forge ──` section exactly when `manifest.forge.is_some()`
- [ ] `cargo test --workspace` passes all tests including `test_should_create_pr_guard` and two new dry_run tests
- [ ] `examples/job-manifest-forge.toml` exists and passes `smelt run examples/job-manifest-forge.toml --dry-run`

## Verification

- `cargo test -p smelt-cli` (unit) — `test_should_create_pr_guard` passes all 8 combinations
- `cargo test -p smelt-cli --test dry_run` — `test_dry_run_with_forge_shows_forge_section` passes; `test_dry_run_no_pr_flag_accepted` passes
- `cargo run --bin smelt -- run examples/job-manifest-forge.toml --dry-run` — prints `── Forge ──` section with github/owner/my-repo/GITHUB_TOKEN
- `cargo test --workspace` — clean; all pre-existing tests pass

## Observability Impact

- Signals added/changed: `eprintln!("Creating PR: {} → {}...", head, base)` before the call; `eprintln!("PR created: {url}")` on success; token-missing error includes variable name; anyhow context "Phase 9: failed to create GitHub PR" tags the call site in error chain
- How a future agent inspects this: `cat .smelt/run-state.toml` after a run shows `pr_url` and `pr_number`; S03 will poll these and render them in `smelt status`
- Failure state exposed: `SmeltError::Forge { operation: "create_pr", message }` surfaces via anyhow as `Error: Phase 9: failed to create GitHub PR: <message>`; token-missing message names the env var so the user knows exactly what to set

## Inputs

- T01 output: `RunState.pr_url`, `RunState.pr_number` fields exist; `args.no_pr` bool exists; `manifest.forge: Option<ForgeConfig>` exists; forge feature enabled in smelt-cli
- `smelt_core::forge::GitHubForge` — `GitHubForge::new(token: String) -> Self`; `create_pr(repo, head, base, title, body) -> Result<PrHandle>`; `PrHandle { url, number }` (from S01)
- `collect_result.no_changes` — bool from `BranchCollectResult`; `collect_result.branch` — the head branch created by ResultCollector
- Research constraint: `args.no_pr` is accessible inside `exec_future` since it borrows from `args: &RunArgs` in the outer function scope

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — `should_create_pr()` free function; Phase 9 block inside `exec_future`; `── Forge ──` section in `print_execution_plan()`; unit test `test_should_create_pr_guard`
- `crates/smelt-cli/tests/dry_run.rs` — `test_dry_run_with_forge_shows_forge_section`, `test_dry_run_no_pr_flag_accepted`
- `examples/job-manifest-forge.toml` — complete manifest with `[forge]` section; passes `smelt run --dry-run`
