# S02: TUI PR Status Panel with Background Polling — Research

**Date:** 2026-03-23

## Summary

S02 adds a PR status badge to the TUI dashboard and a background polling mechanism that fetches PR state from `gh pr view --json`. The key risk is blocking the TUI event loop with subprocess calls. The solution is already well-established in the codebase: D107 introduced the `mpsc` channel-based event loop with `TuiEvent` variants for background thread delivery. S02 extends this with a `TuiEvent::PrStatusUpdate` variant and a polling thread that runs `gh pr view --json state,statusCheckRollup,reviewDecision` on a configurable interval (default 60s).

The `gh pr view` JSON fields needed are: `state` (OPEN/MERGED/CLOSED), `statusCheckRollup` (array of check runs with `conclusion` and `status`), and `reviewDecision` (APPROVED/CHANGES_REQUESTED/REVIEW_REQUIRED/""). These three fields provide the PR badge, CI summary, and review status. The `--json` flag is stable and documented.

The dashboard already renders milestones in a list. Milestones with `pr_number: Some(n)` are the ones that need PR status polling. The polling thread iterates these, calls `gh pr view <n> --json ...` for each, and sends results back via the channel. The dashboard render function adds a status badge (e.g., `🟢 OPEN ✓2/2 checks`) after the milestone name.

## Recommendation

Extend the existing `TuiEvent` enum with `PrStatusUpdate { slug: String, info: PrStatusInfo }`. Add a `PrStatusInfo` struct in `assay-tui` (not `assay-types` — it's a display-only view type). Spawn a background polling thread on app startup that sleeps for 60s between polls. The thread reads `App.milestones` snapshot (cloned slugs + pr_numbers at startup and after each milestone refresh) and calls `gh pr view <pr_number> --json state,statusCheckRollup,reviewDecision` for each. Parse the JSON, send results through the channel.

Store `HashMap<String, PrStatusInfo>` on `App` for the dashboard renderer to read. The renderer shows the badge only when an entry exists for that milestone slug. Polling failures degrade gracefully to no badge (no error display).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Background event delivery to TUI | `mpsc::channel<TuiEvent>` (D107, main.rs) | Already proven; just add a variant |
| Subprocess execution for `gh` | `std::process::Command` (pr.rs pattern) | Consistent with D008 (git/gh CLI-first) |
| JSON response parsing | `serde_json::from_slice` (pr.rs `parse_gh_output`) | Same pattern, different fields |
| Atomic milestone reload | `milestone_scan` (app.rs `handle_agent_done`) | Already used for post-agent refresh |

## Existing Code and Patterns

- `crates/assay-tui/src/main.rs` — Channel-based event loop with `TuiEvent` dispatch. Clone `tx` for the polling thread exactly like the crossterm input thread.
- `crates/assay-tui/src/event.rs` — `TuiEvent` enum. Add `PrStatusUpdate` variant here.
- `crates/assay-tui/src/app.rs:handle_agent_done` — Pattern for refreshing `self.milestones` and `self.cycle_slug` after background events. Reuse for `handle_pr_status_update`.
- `crates/assay-core/src/pr.rs` — `parse_gh_output` parses `gh` JSON. New `pr_status_poll` function follows the same `Command::new("gh")` pattern.
- `crates/assay-core/tests/pr.rs` — `write_fake_gh` + `with_mock_gh_path` pattern for testing `gh` subprocess calls with mock binaries. Reuse for polling tests.
- `crates/assay-tui/src/app.rs:draw_dashboard` (line 1266) — Renders milestone list items. Add PR badge to the `ListItem` line format.
- `crates/assay-tui/tests/agent_run.rs` — Integration test pattern: construct `App` directly, drive events, assert on `Screen` state. Follow for PR status tests.

## Constraints

- **Sync-core convention (D007):** No tokio runtime. Background work uses `std::thread::spawn` + `mpsc::channel`. The polling thread is a plain loop with `std::thread::sleep(Duration::from_secs(interval))`.
- **Zero-trait convention (D001):** `PrStatusInfo` is a plain struct. No `Widget` trait impls. Rendering via free functions.
- **`gh` availability:** Polling must gracefully handle `gh` not being installed (skip polling entirely, no crash). Check once at startup.
- **`pr_number` as poll key:** Only milestones with `pr_number: Some(n)` are polled. The polling thread needs the slug→pr_number mapping, not the full `Milestone` struct.
- **Thread lifetime:** The polling thread runs for the lifetime of the TUI process. On `q` quit, the main loop exits and the OS reclaims the thread (same pattern as the crossterm input thread).
- **D116 confirms the architecture:** "PR status polling via background thread + TuiEvent. Polling interval 60s, configurable."

## Common Pitfalls

- **Blocking the event loop with `gh` calls** — `gh pr view` takes 200-500ms per call. Never call it inside `handle_event()` or `draw()`. Always in the background thread. The channel-based loop (D107) handles this cleanly.
- **Stale milestone list in polling thread** — If the user creates a new milestone with a PR while the TUI is running, the polling thread won't know about it until the milestone list is refreshed. Solution: the polling thread re-reads the milestone list from `App`'s shared state (via a `Mutex<Vec<(String, u64)>>` for slug+pr_number pairs) or accepts updates via a separate channel. Simplest: use `Arc<Mutex<Vec<(String, u64)>>>` shared between `App` and the polling thread, updated on milestone refresh.
- **Rate limiting with many milestones** — If a project has 10 milestones with PRs, polling all 10 every 60s means 10 `gh` calls per minute. Acceptable for local use. If concerned, poll one milestone per tick (round-robin) rather than all at once.
- **`gh` auth failure mid-session** — If `gh` token expires while TUI is running, polls will start failing. Degrade gracefully: log nothing, just don't update the status. The badge shows the last known state or nothing.
- **Thread panic isolation** — If the polling thread panics (unlikely but possible from malformed JSON), the main loop continues running. `mpsc::Sender` dropping just means no more updates. But add `std::panic::catch_unwind` around the poll loop body for defense.
- **`statusCheckRollup` can be empty or null** — When a PR has no CI configured, `statusCheckRollup` is `[]`. When checks are still running, some entries have `status: "IN_PROGRESS"`. Handle both cases: empty = "no checks", in-progress = "pending".

## Open Risks

- **`gh pr view` for merged/closed PRs** — After a PR is merged, `gh pr view <number>` still returns valid JSON with `state: "MERGED"`. This is correct behavior — the badge should show "MERGED" until the milestone is completed and the PR number is cleared or the user navigates away.
- **Configurable poll interval** — D116 says "60s, configurable". Where does the interval live? Options: (a) new field on `Config` in `assay-types`, (b) hardcoded with a plan to make configurable later. Recommend (b) for S02 — a `const POLL_INTERVAL_SECS: u64 = 60` in the polling module. Config extension is low-value scope creep.
- **Shared state between main thread and polling thread** — `App` is not `Send`/`Sync` (it holds `ListState`, mpsc handles, etc.). The polling thread cannot hold a reference to `App`. Solution: `Arc<Mutex<Vec<(String, u64)>>>` for the poll target list, cloned to both `App` and the thread. On milestone refresh, `App` updates the mutex. Polling thread reads it each cycle.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | `blacktop/dotfiles@ratatui-tui` (57 installs) | available — not needed (established patterns in codebase) |
| Ratatui | `padparadscho/skills@rs-ratatui-crate` (22 installs) | available — not needed |

No skills needed — the codebase already has extensive Ratatui patterns (D089, D097, D105, D107) and the TUI architecture is well-established across M006/M007.

## Sources

- `gh pr view --json` field reference from `gh pr view --help` — confirmed `state`, `statusCheckRollup`, `reviewDecision` fields available
- Live `gh pr list --json` output from this repo — confirmed `statusCheckRollup` structure: `{ __typename, conclusion, status, name, workflowName }`
- D107 (Decisions Register) — unified TUI event loop using mpsc channel
- D116 (Decisions Register) — PR status polling via background thread + TuiEvent, 60s interval
- S01 boundary map — `Milestone.pr_number` is the field used to decide which milestones to poll
