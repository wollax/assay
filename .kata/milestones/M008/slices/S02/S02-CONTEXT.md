---
id: S02
milestone: M008
status: ready
---

# S02: TUI PR status panel with background polling — Context

## Goal

Show live PR status (open/merged/closed), CI check summary, and review approval state inline on the Dashboard milestone list, polled via a background thread using the existing `TuiEvent` channel.

## Why this Slice

S02 is the high-risk slice in M008 — it introduces background subprocess polling into the TUI event loop, which is the first use of periodic background work (M007's agent streaming was one-shot, not interval-based). Retiring this risk early proves the pattern works. S02 depends on S01's Milestone type extensions (`pr_number`, `pr_labels`, `pr_reviewers`).

## Scope

### In Scope

- Background polling thread that runs `gh pr view --json state,statusCheckRollup,reviews` for each milestone with `pr_number` set
- New `TuiEvent::PrStatusUpdate { slug, info }` variant for delivering poll results to the main event loop
- `PrStatusInfo` type in assay-core::pr with fields: `state` (open/merged/closed), `ci_status` (pass/fail/pending), `review_status` (approved/changes_requested/pending)
- Inline PR badge on Dashboard milestone rows: `PR:#42 ✓CI ✓Rev` or `PR:#42 ✗CI ○Rev` etc.
- Auto-start polling on TUI launch for milestones with `pr_number` set
- Manual refresh via `R` key from Dashboard for instant re-poll
- One-time status bar warning when `gh` is not installed or not authenticated, then silent skip
- Stop polling for milestones in terminal PR states (merged/closed) — cache final state, only re-poll on manual `R`
- Polling interval: 60 seconds (hardcoded for S02; configurable is a future concern)
- CI summary as a single icon: ✓ (all pass), ✗ (any fail), ○ (pending)
- Review status as a single icon: ✓ (approved), ! (changes requested), ○ (pending review)
- Integration tests with mock `gh` binary returning JSON responses

### Out of Scope

- CI check drill-down (listing individual check runs) — future slice
- Configurable polling interval — hardcode 60s for now
- PR detail screen (full PR view with body, labels, reviewers list) — future slice
- Webhook-based real-time updates — polling only
- Polling from non-Dashboard screens (only poll while Dashboard is visible)
- PR creation or update from the status panel — status is read-only

## Constraints

- Must use the existing `TuiEvent` channel (D107) — no tokio runtime, no new event loop
- `gh pr view` is a sync subprocess call (D065/D008) — must run in a background `std::thread`
- Dashboard render function takes individual fields per D097 — PR status data stored on `App` level fields per D099 pattern
- No new Screen variant needed — PR status is rendered inline within `draw_dashboard`
- `gh` missing/unauth produces a one-time warning in the status bar, then polling is disabled for the session

## Integration Points

### Consumes

- `Milestone.pr_number: Option<u64>` — determines which milestones to poll (already exists)
- `Milestone.pr_url: Option<String>` — for potential display (already exists)
- `TuiEvent` enum in `assay-tui::event` — extends with `PrStatusUpdate` variant
- `App.event_tx: Option<Sender<TuiEvent>>` — used to send poll results from background thread
- `draw_dashboard` function — enhanced to render PR badge inline

### Produces

- `pr_status_poll(pr_number, working_dir) -> Result<PrStatusInfo>` free function in assay-core::pr
- `PrStatusInfo { state: PrState, ci_status: CiStatus, review_status: ReviewStatus }` — view type in assay-core::pr (not assay-types, per D078 pattern)
- `TuiEvent::PrStatusUpdate { slug: String, info: PrStatusInfo }` — new event variant
- `App.pr_statuses: HashMap<String, PrStatusInfo>` — cached poll results, keyed by milestone slug
- Background poll thread spawned on TUI launch, sends updates via `event_tx`
- `R` key handler in Dashboard that triggers immediate re-poll

## Open Questions

- None — all behavioral decisions captured during discuss.
