# S02: TUI PR status panel with background polling — UAT

**Milestone:** M008
**Written:** 2026-03-23

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The integration tests prove parsing and state management mechanically, but the visual TUI experience (badge rendering, polling timing, graceful degradation) can only be verified by a human running `assay-tui` against a real GitHub PR.

## Preconditions

- `gh` CLI installed and authenticated (`gh auth status` succeeds)
- A GitHub repository with at least one open PR
- An `.assay/` project initialized in that repository (`assay init`)
- A milestone TOML file with `pr_number` set to the open PR's number (e.g., `pr_number = 42`)
- `cargo build -p assay-tui` succeeds

## Smoke Test

Run `assay-tui` in the project root. Confirm the dashboard renders milestone names. If a milestone has `pr_number` set and `gh` is available, a PR status badge (e.g., `🟢 OPEN ✓2/2`) should appear within a few seconds.

## Test Cases

### 1. PR badge appears for milestone with open PR

1. Create a milestone TOML with `pr_number = <your-open-PR-number>`
2. Run `cargo run -p assay-tui`
3. Wait ~5 seconds (initial poll has no delay)
4. **Expected:** The milestone's dashboard line shows a badge like `🟢 OPEN ✓N/N` with CI check counts

### 2. Badge updates after PR is merged

1. While `assay-tui` is running with a badge visible, merge the PR on GitHub
2. Wait up to 60 seconds for the next poll cycle
3. **Expected:** Badge changes to `🟣 MERGED` (CI counts may still show)

### 3. Review status shown in badge

1. Create an open PR and request a review (or have a review submitted)
2. Set the milestone's `pr_number` to that PR
3. Run `assay-tui` and wait for the badge
4. **Expected:** Badge includes review abbreviation: `✓rvw` (approved), `△rvw` (changes requested), or `?rvw` (review required)

### 4. No badge when milestone has no pr_number

1. Ensure a milestone TOML has no `pr_number` field
2. Run `assay-tui`
3. **Expected:** That milestone's dashboard line shows only the name and status — no PR badge

## Edge Cases

### gh CLI not installed

1. Temporarily rename/remove `gh` from PATH
2. Run `assay-tui`
3. **Expected:** Dashboard renders normally without any PR badges. An eprintln warning about `gh` not found appears on stderr (visible if running in a terminal that shows stderr before TUI takes over).

### Network failure during polling

1. Start `assay-tui` with a valid `pr_number` milestone — badge appears
2. Disconnect from the network
3. Wait 60 seconds for next poll
4. **Expected:** Badge stays showing the last known state — no crash, no error display. Badge may go stale but won't disappear until the app is restarted.

### Milestone with invalid pr_number

1. Set `pr_number = 999999999` (nonexistent PR)
2. Run `assay-tui`
3. **Expected:** No badge appears for that milestone — `gh pr view` fails silently, no crash

## Failure Signals

- TUI crashes or hangs on startup (polling thread panic)
- Badge never appears despite valid `pr_number` and working `gh`
- Badge shows incorrect state (Open when PR is Merged)
- CI counts don't add up (pass + fail + pending ≠ total checks)
- Dashboard rendering breaks (layout corruption, missing milestone names)

## Requirements Proved By This UAT

- R058 (Advanced PR workflow) — TUI PR status visibility: this UAT proves the badge renders correctly with real `gh` data, polls on interval, and degrades gracefully. Combined with S01's UAT (labels/reviewers in `gh pr create`), R058 is fully validated end-to-end.

## Not Proven By This UAT

- PR creation workflow (covered by S01 UAT)
- Concurrent polling of many milestones under load (only typical scale tested)
- Exact polling interval timing (60s is approximate due to subprocess execution time)

## Notes for Tester

- The initial poll happens immediately on startup (no 60s wait) — badges should appear within a few seconds if `gh` is fast.
- Review decision abbreviations: APPROVED→`✓rvw`, CHANGES_REQUESTED→`△rvw`, REVIEW_REQUIRED→`?rvw`. Empty review decision shows no review indicator.
- The polling thread is spawned only when `gh` is available AND at least one milestone has `pr_number` set.
