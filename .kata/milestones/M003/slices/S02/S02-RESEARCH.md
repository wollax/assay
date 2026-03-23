# S02: Manifest Forge Config + PR Creation — Research

**Researched:** 2026-03-21
**Domain:** Rust manifest parsing, CLI wiring, octocrab/forge integration
**Confidence:** HIGH — S01 established all forge types; S02 is pure wiring

## Summary

S02 has three distinct seams: (1) extending `JobManifest` with an optional `[forge]` section, (2) adding `pr_url`/`pr_number` to `RunState`, and (3) inserting Phase 9 into `execute_run()`. All three seams touch existing, tested code. S01 already delivered `ForgeConfig`, `GitHubForge`, and `ForgeClient::create_pr()` — S02 only wires them together.

The most important structural fact: `exec_future` in `run.rs` is an async block that borrows `manifest` and `monitor` mutably. Phase 9 inserts cleanly inside this block, after `ResultCollector::collect()` and before the `Ok(assay_exit)` return. No additional ownership gymnastics are needed — the pattern is already established by the existing Phase 8 collect code.

The only non-trivial constraint is feature gating: smelt-cli must explicitly enable `features = ["forge"]` on its smelt-core dependency to access `GitHubForge`. Since smelt-cli is a binary (not a library), always enabling the forge feature is the correct choice — the no-octocrab feature isolation is for library consumers, not for the CLI itself.

## Recommendation

Enable `features = ["forge"]` on smelt-cli's smelt-core dependency unconditionally. Add `forge: Option<ForgeConfig>` to `JobManifest` with `#[serde(default)]`. Add `pr_url`/`pr_number` to `RunState` with `#[serde(default)]`. Wire Phase 9 inside `exec_future` after the collect step, guarded by `manifest.forge.is_some() && !args.no_pr && !collect_result.no_changes`. Print `PR created: <url>` to stderr.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| GitHub PR creation | `GitHubForge::create_pr()` in `crates/smelt-core/src/forge.rs` | S01 delivered this; unit-tested with wiremock happy/401/422 paths |
| owner/repo parsing | Implement inline `str::split_once('/')` in manifest validate | `parse_repo()` in `forge.rs` is `#[cfg(feature = "forge")]` + private; safe to duplicate the trivial check |
| Token reading | `std::env::var(&forge_cfg.token_env)` | One-liner; no extra dep |
| Forge error type | `SmeltError::forge(op, msg)` | Already in `error.rs`; maps directly from `anyhow` error surface |

## Existing Code and Patterns

- `crates/smelt-core/src/manifest.rs` — `JobManifest` struct with `#[serde(deny_unknown_fields)]` on the struct itself (not on sub-structs' fields). Adding `forge: Option<ForgeConfig>` with `#[serde(default)]` is the correct pattern; the TOML parser will accept manifests without `[forge]` and parse it as `None`.
- `crates/smelt-core/src/forge.rs` — `ForgeConfig { provider, repo, token_env }` is already `#[derive(Deserialize, Clone)]` with `deny_unknown_fields`; import it in `manifest.rs` with `use crate::forge::ForgeConfig` (no feature needed, per D055).
- `crates/smelt-core/src/monitor.rs` — `RunState` derives `Serialize, Deserialize`. New optional fields need `#[serde(default)]` on each field individually (not on the struct) to maintain backward compat with existing state files written without those fields.
- `crates/smelt-cli/src/commands/run.rs` — `exec_future` is an async block borrowing `manifest` (for `manifest.job.repo`, `manifest.merge.target`, etc.) and `monitor` mutably (for `set_phase`, `set_container`). Phase 9 sits naturally after the `collect_result` binding, before `Ok::<i32, anyhow::Error>(assay_exit)`.
- `crates/smelt-cli/Cargo.toml` — `smelt-core.path = "../smelt-core"` with no features currently. Must become `smelt-core = { path = "../smelt-core", features = ["forge"] }`.

## Constraints

- **D055 (firm)**: `ForgeConfig` is unconditional; `GitHubForge` is gated. `manifest.rs` imports `ForgeConfig` without enabling forge. The forge feature is only needed in smelt-cli's Cargo.toml (to call `GitHubForge::new()` in Phase 9).
- **D017 (firm)**: `deny_unknown_fields` on manifest structs. `ForgeConfig` already has it; `JobManifest` already has it. No relaxation.
- **D018 (firm)**: Validation collects all errors before returning. Forge validation errors must be pushed into the same `errors` Vec, not returned early.
- **D014 (firm)**: `GITHUB_TOKEN` is read from the host environment by name (`token_env`). The value never enters the container. Phase 9 runs entirely on the host after `ResultCollector::collect()` exits.
- **RPITIT (D019)**: No `async_trait`. `ForgeClient::create_pr()` uses RPITIT — already handled in S01.
- **`no_changes` guard**: `collect_result.no_changes == true` means no branch was created. Phase 9 must be skipped in this case. Attempting to create a PR against a nonexistent head branch produces a confusing GitHub 422 error.
- **State cleanup timing**: `monitor.cleanup()` removes the state file at the end of a run. For S02, writing `pr_url`/`pr_number` to `RunState` during Phase 9 is sufficient — the fields persist in the state file until cleanup. S04 will change the file path; S03 will adjust cleanup behavior for post-run status reading. S02 does not need to change cleanup logic.
- **Serde default on RunState fields**: TOML deserialization of existing state files (without `pr_url`/`pr_number`) will fail if the fields lack `#[serde(default)]`. This must be applied per-field, not on the struct.
- **`exec_future` ownership**: `exec_future` is polled in `tokio::select!` alongside a timeout and cancel future. After select!, exec_future is done (or dropped), so its borrows are released. `monitor.set_phase()` calls after select! are valid. Inserting `monitor`-mutating code inside `exec_future` is safe — the borrow ends when exec_future completes.

## Common Pitfalls

- **Missing `#[serde(default)]` on new RunState fields** — Without it, any existing `.smelt/run-state.toml` written by M001/M002 runs will fail to deserialize in `smelt status`, breaking backward compat. Add `#[serde(default)]` on each new field individually.
- **Calling Phase 9 when no_changes** — If Assay produced no commits, `collect_result.no_changes` is true and `collect_result.branch` is the target branch name but no branch was created on the remote. Creating a PR against it returns a GitHub 422. Guard: `if !collect_result.no_changes && manifest.forge.is_some() && !args.no_pr`.
- **Duplicate `parse_repo` validation vs forge runtime check** — `parse_repo()` in `forge.rs` is private and feature-gated. The manifest `validate()` method must implement its own check: `forge.repo.split_once('/').map(|(a,b)| a.is_empty() || b.is_empty()).unwrap_or(true)` → push error. Do NOT call into `forge.rs` from `manifest.rs` for validation.
- **`token_env` readable-at-runtime check in validate()** — `validate()` does NOT check if the env var is actually set (that would make dry_run behavior environment-dependent). It only validates that `token_env` is non-empty (structural validation). The actual `std::env::var` call is in Phase 9 at runtime.
- **PR title and body** — Not specified in the boundary map. Keep simple: title `format!("[smelt] {} — {} → {}", job_name, base_ref, target_branch)`, body `format!("Automated results from smelt job '{}'.\n\nBase: `{}`", job_name, base_ref)`. The exact format is an implementation detail; do not over-engineer.
- **`smelt run --no-pr` position in RunArgs** — This flag should be a bool field on `RunArgs`, not a subcommand. Pattern: `#[arg(long)] pub no_pr: bool`. It must be threaded from `RunArgs` into `run_with_cancellation()` (which takes `&RunArgs` — already has access).
- **Roundtrip test for `[forge]` with TOML** — The existing test constant `VALID_MANIFEST` doesn't include `[forge]`. Add two separate test TOML strings: one with `[forge]`, one without. For the "with forge" case, verify `manifest.forge.is_some()` and field values. For "without forge", verify `manifest.forge.is_none()` (this is implicitly tested by existing parse tests, but an explicit test documents the intent).
- **`smelt-cli` feature gating in Phase 9** — Since smelt-cli will always enable the forge feature, there is NO need for `#[cfg(feature = "forge")]` guards in `run.rs`. The code compiles unconditionally from smelt-cli's perspective.

## Open Risks

- **422 "A pull request already exists" on re-run** — If `smelt run` is called twice against the same head/base, GitHub returns 422 on the second PR creation attempt. The forge client maps this to `SmeltError::Forge`. For S02, surface the error clearly (let it propagate to the anyhow error surface). A retry/detect-existing-PR story is deferred.
- **`token_env` not set at runtime** — `std::env::var` will return `Err`. Phase 9 should convert this to a clear `anyhow::bail!` with the variable name in the message: `"env var {} not set — required for PR creation (forge.token_env)"`. Do not panic.
- **`collect_result.branch` vs `manifest.merge.target`** — These are the same value (target branch name) in the current implementation. `collect_result.branch` is the authoritative value to use as the PR head, since it's what `ResultCollector::collect()` actually created.
- **`smelt run --dry-run` + `[forge]`** — `execute_dry_run()` currently prints credentials and sessions. It should also print the forge config section (provider, repo, `token_env` name) when `manifest.forge.is_some()`. This is a usability addition but not in the strict boundary map scope — include it as part of `print_execution_plan()` if time permits; it's low-risk.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust (serde/TOML) | — | no skill needed — patterns established in codebase |
| octocrab | — | S01 established all patterns; no additional research needed |

## Sources

- S01 summary (preloaded) — all forge module patterns, forward intelligence on what S02 needs
- `crates/smelt-core/src/forge.rs` — confirmed `ForgeConfig` is unconditional, `GitHubForge` is feature-gated, `parse_repo` is private + feature-gated
- `crates/smelt-core/src/manifest.rs` — confirmed `JobManifest` uses `deny_unknown_fields`, existing validation pattern (push to `errors` Vec)
- `crates/smelt-core/src/monitor.rs` — confirmed `RunState` fields, `JobMonitor` API, cleanup behavior
- `crates/smelt-cli/src/commands/run.rs` — confirmed `exec_future` async block structure, `monitor` borrow pattern, collect result binding location for Phase 9 insertion
- `crates/smelt-cli/Cargo.toml` — confirmed no `forge` feature currently enabled on smelt-core dep
