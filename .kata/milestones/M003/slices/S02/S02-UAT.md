# S02: Manifest Forge Config + PR Creation — UAT

**Milestone:** M003
**Written:** 2026-03-21

## UAT Type

- UAT mode: mixed (artifact-driven + live-runtime)
- Why this mode is sufficient: Guard logic, dry-run output, manifest parsing, and backward compat are fully proven by automated tests (artifact-driven). The live PR creation path (Phase 9 calling the real GitHub API) requires a real GITHUB_TOKEN and repo — that portion is live-runtime and requires human execution.

## Preconditions

For automated checks (no special setup):
- `cargo test --workspace` passes (124 tests)
- `examples/job-manifest-forge.toml` present in repo root

For live PR creation (manual UAT):
- Real GitHub repo available (the tester's own fork or test repo)
- `GITHUB_TOKEN` set with `repo` scope (create PR + push branch permissions)
- Real Docker daemon running
- `smelt` binary built: `cargo build --release --workspace`
- A manifest with `[forge]` pointing to the test repo, a valid `assay` binary available, and a job that produces a non-empty result branch

## Smoke Test

```
cargo run --bin smelt -- run examples/job-manifest-forge.toml --dry-run
```

Should print `── Forge ──` section with provider/repo/token_env and exit 0. If this fails, the forge manifest integration is broken.

## Test Cases

### 1. Dry-run with forge manifest shows forge section

1. Run: `cargo run --bin smelt -- run examples/job-manifest-forge.toml --dry-run`
2. **Expected:** Output contains `── Forge ──`, `Provider:    github`, `Repo:        owner/my-repo`, `Token env:   GITHUB_TOKEN`, `(use --no-pr to skip PR creation)`. Exits 0.

### 2. Dry-run accepts --no-pr flag

1. Run: `cargo run --bin smelt -- run examples/job-manifest-forge.toml --dry-run --no-pr`
2. **Expected:** Exits 0 without error. (--no-pr is accepted as a valid flag.)

### 3. Manifest without [forge] parses cleanly

1. Run: `cargo run --bin smelt -- run examples/job-manifest.toml --dry-run` (or any manifest without `[forge]`)
2. **Expected:** No forge section in output. Exits 0. No forge-related errors.

### 4. Invalid forge config rejected at validation

1. Create a temp manifest with `[forge]` having `repo = "not-valid"` (no slash) or empty `token_env = ""`
2. Run: `cargo run --bin smelt -- run /tmp/bad-forge.toml --dry-run`
3. **Expected:** Exits non-zero with a validation error mentioning `owner/repo format` or `token_env`. Does not crash.

### 5. Existing state file without PR fields round-trips cleanly

1. Run: `cargo test -p smelt-core monitor::tests::test_run_state_backward_compat_no_pr_fields`
2. **Expected:** Test passes. Confirms old `.smelt/run-state.toml` files without `pr_url`/`pr_number` deserialize to `None` without error.

### 6. Live PR creation (manual, requires real GITHUB_TOKEN + Docker)

1. Set `GITHUB_TOKEN` in environment
2. Create/edit a manifest with `[forge]` pointing to your test repo
3. Run: `smelt run <your-manifest.toml>`
4. Wait for full execution (Docker + Assay + result collection)
5. **Expected:** After Phase 8 (result collection), stderr prints `Creating PR: <head> → <base>...` then `PR created: https://github.com/<owner>/<repo>/pull/<number>`. Process exits 0 (assuming gates pass). PR appears in the GitHub repo.

### 7. --no-pr skips creation even with [forge] configured

1. Same setup as test 6 (real token, forge manifest)
2. Run: `smelt run <your-manifest.toml> --no-pr`
3. **Expected:** Run completes. No `Creating PR:` or `PR created:` output. No PR created in GitHub. Exits 0 (assuming gates pass). `cat .smelt/run-state.toml` shows `pr_url` absent or empty.

### 8. Missing token fails with actionable error

1. Unset `GITHUB_TOKEN` (or name it incorrectly in `token_env`)
2. Run a job that reaches Phase 9 (forge configured, result has changes, no --no-pr)
3. **Expected:** Process exits non-zero. Stderr contains `"env var GITHUB_TOKEN not set — required for PR creation (forge.token_env)"` (with the actual var name substituted). No PR created.

## Edge Cases

### no_changes short-circuits PR creation

1. Arrange a run where the result branch has no diff vs base (no changes from Assay)
2. Run: `smelt run <manifest.toml>` (forge configured, no --no-pr)
3. **Expected:** `JobPhase::NoChanges` set. No `Creating PR:` output. No PR created. Exits 0.

### Unknown field in [forge] section rejected

1. Add `unknown_key = "value"` to `[forge]` in a manifest
2. Run: `smelt run <manifest.toml> --dry-run`
3. **Expected:** Parse error mentioning unknown field. Exits non-zero.

## Failure Signals

- `── Forge ──` absent from dry-run output when forge manifest used → Phase 9 display path broken
- `cargo test --workspace` failing any test → regression introduced
- `pr_url` absent from `.smelt/run-state.toml` after a live forge run (without --no-pr and with changes) → Phase 9 not writing state
- Missing token error message doesn't include the env var name → error message formatting regression
- PR created in GitHub but `pr_url` not written to state file → monitor write step in Phase 9 not executing

## Requirements Proved By This UAT

- R002 (Job manifest supports forge configuration block) — fully proved by automated tests (cases 1–5); manifest parsing, validation, round-trip, and deny_unknown_fields all verified
- R001 (smelt run creates GitHub PR from result branch) — live-runtime path proved by cases 6–8 (manual); automated tests prove guard logic and dry-run display but not the actual API call

## Not Proven By This UAT

- S03 (smelt status PR section, smelt watch command) — pr_url is persisted but not yet displayed by `smelt status`
- Full end-to-end round-trip (smelt run → PR → smelt watch → merge → exit 0) — requires S03
- PR creation with non-default base branch (e.g. non-main base) — not tested; Phase 9 uses the manifest's `base_ref` field
- Rate-limit handling on the GitHub API — octocrab default behavior applies; no explicit retry logic in S02

## Notes for Tester

- The `examples/job-manifest-forge.toml` fixture uses `owner/my-repo` as the repo — it is intentionally a placeholder. For live testing, copy and edit it with your real repo.
- `GITHUB_TOKEN` must have `repo` scope (or `pull_requests: write` on fine-grained PAT) to create PRs programmatically.
- If a PR already exists for the head→base pair, Phase 9 will fail with a 422 from GitHub (surfaced via anyhow chain). This is expected behavior — smelt does not detect or reuse existing PRs.
- After a successful live run, inspect state with: `cat .smelt/run-state.toml | grep pr_`
