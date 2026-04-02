---
estimated_steps: 7
estimated_files: 5
---

# T01: Extend RunState with forge context fields and add smelt status PR section

**Slice:** S03 — PR Status Tracking
**Milestone:** M003

## Description

RunState needs five new backward-compatible fields so that `smelt watch` can reconstruct the GitHub client and display cached PR status without re-reading the original manifest. Phase 9 in `run.rs` must persist `forge_repo` and `forge_token_env` at PR creation time. The `smelt status` command gains a `── Pull Request ──` section rendered from cached RunState fields. A new test file validates both the display logic and the backward-compat guarantee.

This task closes R003 at the contract level (display logic proven by unit tests against in-memory RunState values; no live GitHub call required).

## Steps

1. **Extend RunState in `monitor.rs`:** add five fields with `#[serde(default)]`:
   - `pub pr_status: Option<PrState>` — last-known PR state from a poll
   - `pub ci_status: Option<CiStatus>` — last-known CI state
   - `pub review_count: Option<u32>` — last-known review count
   - `pub forge_repo: Option<String>` — `owner/repo` from ForgeConfig, saved at Phase 9
   - `pub forge_token_env: Option<String>` — env var name for token, saved at Phase 9
   Add `use crate::forge::{PrState, CiStatus};` import. Update `JobMonitor::new()` to initialize all five to `None`.

2. **Update Phase 9 in `run.rs`:** after `monitor.state.pr_number = Some(pr.number)`, add:
   ```rust
   monitor.state.forge_repo = Some(forge_cfg.repo.clone());
   monitor.state.forge_token_env = Some(forge_cfg.token_env.clone());
   monitor.write().map_err(|e| anyhow::anyhow!("{e}"))?;
   ```
   (The existing `monitor.write()` call can be consolidated — ensure one write includes all five fields.)

3. **Create `tests/status_pr.rs` with failing tests first:** write the test file before implementing `format_pr_section`. Tests must compile but can fail. Tests to include:
   - `test_format_pr_section_absent_when_no_url`: `make_state_no_pr()` → `format_pr_section` returns None
   - `test_format_pr_section_shows_url`: state with `pr_url = Some("https://github.com/o/r/pull/42")` → Some(text) containing the URL
   - `test_format_pr_section_shows_state_ci_reviews`: state with pr_url + pr_status=Open + ci_status=Pending + review_count=3 → section contains "Open", "Pending", "3"
   - `test_format_pr_section_shows_unknown_when_no_cached_status`: pr_url set, all status fields None → section contains "unknown" for state and CI
   - `test_run_state_new_fields_backward_compat`: deserialize TOML string without any new fields → all five new fields are None

4. **Implement `format_pr_section` in `status.rs`:** `pub(crate) fn format_pr_section(state: &RunState) -> Option<String>`. Returns `None` when `state.pr_url.is_none()`. When `pr_url` is set, builds:
   ```
   ── Pull Request ──
     URL:     https://github.com/owner/repo/pull/42
     State:   Open          (or "unknown" if pr_status is None)
     CI:      Pending       (or "unknown" if ci_status is None)
     Reviews: 3             (or "0" if review_count is None)
   ```
   Import `smelt_core::forge::{PrState, CiStatus}`. Add `use` in `status.rs`.

5. **Wire `format_pr_section` into `print_status()`:** after the existing `println!` calls in `print_status()`, add:
   ```rust
   if let Some(section) = format_pr_section(state) {
       println!("{section}");
   }
   ```

6. **Fix test helpers:** scan `crates/smelt-cli/src/commands/status.rs` and `tests/docker_lifecycle.rs` for `RunState { ... }` struct literals. Add `pr_status: None, ci_status: None, review_count: None, forge_repo: None, forge_token_env: None` to each (they'll fail to compile otherwise). The S02 patch already fixed `pr_url`/`pr_number` — only the new five fields need adding.

7. **Run tests to confirm:** `cargo test -p smelt-cli --test status_pr` must pass; `cargo test -p smelt-core` must pass (monitor backward-compat); `cargo test --workspace` must pass.

## Must-Haves

- [ ] `RunState` has `pr_status`, `ci_status`, `review_count`, `forge_repo`, `forge_token_env` — all `#[serde(default)]`
- [ ] Old TOML without these fields deserializes to `None` for all five (backward-compat test passes)
- [ ] Phase 9 in `run.rs` writes `forge_repo` and `forge_token_env` into RunState when PR is created
- [ ] `format_pr_section` returns `None` when `pr_url` is `None`; returns `Some(text)` containing URL, state, CI, and review count when `pr_url` is set
- [ ] `smelt status` output includes `── Pull Request ──` section when `pr_url` is set; section absent otherwise
- [ ] All 5 tests in `tests/status_pr.rs` pass
- [ ] `cargo test --workspace` passes (no regressions)

## Verification

- `cargo test -p smelt-cli --test status_pr -q` — 5 tests pass
- `cargo test -p smelt-core -q` — monitor tests including backward-compat pass
- `cargo test --workspace -q` — all tests pass, 0 failed
- Inspect `tests/status_pr.rs`: `test_run_state_new_fields_backward_compat` uses a raw TOML string without the new fields and asserts all five are None

## Observability Impact

- Signals added/changed: `smelt status` now emits a `── Pull Request ──` section when pr_url is set — new human-readable signal for PR state after `smelt run`
- How a future agent inspects this: `cat .smelt/run-state.toml` shows pr_status, ci_status, review_count, forge_repo, forge_token_env after Phase 9; `smelt status` renders these fields
- Failure state exposed: if forge_repo/forge_token_env are None in RunState, `smelt watch` (T02) will surface a clear "no forge context in state" error — this is detectable from the state file

## Inputs

- `crates/smelt-core/src/monitor.rs` — RunState struct, JobMonitor::new(), existing pr_url/pr_number fields from S02
- `crates/smelt-cli/src/commands/run.rs` — Phase 9 block (after `create_pr` call); `forge_cfg` variable is in scope
- `crates/smelt-cli/src/commands/status.rs` — `print_status()` function, existing status display structure
- `crates/smelt-core/src/forge.rs` — `PrState`, `CiStatus` enums for use in format_pr_section
- S02 summary: `#[serde(default)]` on per-field basis is the established backward-compat pattern (not on the struct)

## Expected Output

- `crates/smelt-core/src/monitor.rs` — RunState with 5 new serde-default fields; JobMonitor::new() initializes them to None
- `crates/smelt-cli/src/commands/run.rs` — Phase 9 writes forge_repo and forge_token_env into RunState
- `crates/smelt-cli/src/commands/status.rs` — `format_pr_section(state) -> Option<String>` function (pub(crate)); `print_status()` calls it and prints when Some
- `crates/smelt-cli/tests/status_pr.rs` — 5 unit tests, all passing
