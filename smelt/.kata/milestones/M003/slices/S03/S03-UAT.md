# S03: PR Status Tracking — UAT

**Milestone:** M003
**Written:** 2026-03-21

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All behavior is verified by automated unit tests against MockForge; the live polling loop and real GitHub interaction are deferred to S06 UAT where a real GITHUB_TOKEN, real repo, and real PR are available. This slice closes R003 and R004 at the contract/unit level — the live proof is an S06 responsibility.

## Preconditions

- `smelt run` with `[forge]` config has completed successfully (state file exists at `.smelt/run-state.toml` with `pr_url` and `pr_number` populated)
- `GITHUB_TOKEN` env var (or the env var named in `forge_token_env`) is set with read/write PR access to the target repo
- A shell with `smelt` on PATH

## Smoke Test

Run `smelt status` in a directory where a prior `smelt run` created a PR. Confirm the `── Pull Request ──` section appears with the correct PR URL.

## Test Cases

### 1. smelt status shows PR section when PR exists

1. Run `smelt run manifest.toml` (with `[forge]` config) — wait for Phase 9 completion and PR URL to be printed
2. In the same directory, run `smelt status`
3. **Expected:** Output includes a `── Pull Request ──` section containing the PR URL, a state line (open/merged/closed or "unknown"), a CI line (pending/passing/failing/unknown), and a review count line

### 2. smelt status shows no PR section when no PR was created

1. Run `smelt run manifest.toml` without a `[forge]` config (or with `--no-pr`)
2. Run `smelt status`
3. **Expected:** No `── Pull Request ──` section in the output; all other status sections render normally

### 3. smelt watch exits 0 when PR is merged

1. Ensure state file exists with `pr_url`, `pr_number`, `forge_repo`, `forge_token_env`
2. Run `smelt watch <job-name>` in a separate terminal
3. Merge the PR on GitHub
4. **Expected:** `smelt watch` prints a final `PR merged.` line and exits 0 within one polling interval (default 30s)

### 4. smelt watch exits 1 when PR is closed without merging

1. Ensure state file exists with `pr_url`, `pr_number`, `forge_repo`, `forge_token_env`
2. Run `smelt watch <job-name>`
3. Close (without merging) the PR on GitHub
4. **Expected:** `smelt watch` prints `PR closed without merging.` and exits 1

### 5. smelt watch prints status line each poll

1. Run `smelt watch <job-name> --interval-secs 5` (short interval for manual testing)
2. Wait 10–15 seconds without changing the PR state
3. **Expected:** Stderr shows multiple lines in the format `[HH:MM:SS] PR #N — state: Open | CI: Pending | reviews: 0` (one per interval)

### 6. smelt watch updates state file on each poll

1. Run `smelt watch <job-name> --interval-secs 5` in background
2. After 2–3 polls, `cat .smelt/run-state.toml`
3. **Expected:** `pr_status`, `ci_status`, `review_count` fields are present and reflect the current GitHub PR state

## Edge Cases

### No PR URL in state file

1. Manually clear `pr_url` from `.smelt/run-state.toml` (or use a state file from a run without `[forge]`)
2. Run `smelt watch <job-name>`
3. **Expected:** Clear error printed to stderr ("no PR URL in state for job..."); exits non-zero

### Token env var unset

1. Ensure state file has `forge_token_env = "GITHUB_TOKEN"`
2. Run `GITHUB_TOKEN= smelt watch <job-name>` (empty token)
3. **Expected:** Clear error printed to stderr about the token being empty/unset; exits non-zero

### --help shows correct arguments

1. Run `smelt watch --help`
2. **Expected:** Shows `<JOB_NAME>` positional argument and `--interval-secs` option with default value 30

## Failure Signals

- `── Pull Request ──` section absent from `smelt status` when `pr_url` IS set → `format_pr_section` not wired into `print_status()`
- `smelt watch` exits immediately with exit code 0 even though PR is open → Merged/Closed check is incorrect
- `smelt watch` panics or exits with unhandled error on transient GitHub API failure → non-fatal error handling missing
- `smelt watch --help` does not show `watch` subcommand → wiring in `main.rs` missing

## Requirements Proved By This UAT

- R003 (smelt status shows PR state and CI status) — UAT cases 1 and 2 prove the section renders when present and is absent when not; field values (state, CI, reviews) are shown correctly
- R004 (smelt watch blocks until PR merges or closes) — UAT cases 3 and 4 prove the exit code contract; case 5 proves the polling loop is live; case 6 proves state file is updated

## Not Proven By This UAT

- Live ETag/conditional-request behavior against the real GitHub API (polling with If-None-Match headers) — deferred to S06 where rate-limit behavior can be observed
- `smelt watch` behavior under real GitHub rate limiting — deferred to S06
- Full end-to-end flow (smelt run → PR created → smelt watch → merge → exits 0) in a single session — deferred to S06 UAT
- `smelt status` after per-job state migration (`.smelt/runs/<job-name>/state.toml`) — depends on S04

## Notes for Tester

- The automated unit tests (MockForge-based) cover all exit-code and state-update logic; the UAT cases above are live-runtime verification that complements the unit coverage.
- For cases 3–6, a real `GITHUB_TOKEN` with repo write access is required; a throwaway test repo is sufficient.
- The polling interval default is 30s; use `--interval-secs 5` for faster manual testing.
- `smelt watch` does not merge PRs — it only observes. Merging the PR on GitHub is the human action that triggers the exit-0 path.
