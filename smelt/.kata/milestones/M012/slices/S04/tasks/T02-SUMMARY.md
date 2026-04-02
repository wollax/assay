---
id: T02
parent: S04
milestone: M012
provides:
  - LinearTrackerSource<L: LinearClient> implementing TrackerSource trait
  - ensure_labels() with find-or-create logic and HashMap label UUID cache
  - poll_ready_issues() mapping LinearIssue → TrackerIssue (UUID as id)
  - transition_state() with remove-old + add-new label two-mutation flow
  - cached_label_uuid() helper with descriptive error when cache miss
key_files:
  - crates/smelt-cli/src/serve/linear/source.rs
  - crates/smelt-cli/src/serve/linear/mod.rs
key_decisions:
  - "TrackerIssue.id = Linear UUID (not identifier) — mutations need UUIDs, consumers treat id as opaque"
patterns_established:
  - "LinearTrackerSource mirrors GithubTrackerSource pattern exactly: new(), ensure_labels(), TrackerSource impl"
  - "Label UUID caching via HashMap<String, String> populated by ensure_labels() — required before transition_state()"
observability_surfaces:
  - "tracing::info! on each label ensured (team_id, label_name, action: found/created)"
  - "tracing::info! on state transition (issue_id, from, to)"
  - "SmeltError::tracker('transition', 'label X not in cache — was ensure_labels() called?') on cache miss"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T02: LinearTrackerSource bridging LinearClient to TrackerSource

**Async Linear tracker source with label UUID caching, two-mutation state transitions, and 10 unit tests covering all paths**

## What Happened

Implemented `LinearTrackerSource<L: LinearClient>` in `source.rs` that bridges the Linear-specific `LinearClient` trait to the platform-agnostic `TrackerSource` trait. The struct holds a `HashMap<String, String>` label cache mapping label names to UUIDs.

`ensure_labels()` iterates `TrackerState::ALL` (6 variants), calls `find_label()` first, and only `create_label()` if not found — caching each UUID. `poll_ready_issues()` calls `list_issues()` with the ready label and maps `LinearIssue` to `TrackerIssue` using the Linear UUID as `TrackerIssue.id` (since mutations require UUIDs). `transition_state()` looks up both from/to label UUIDs in the cache and calls `remove_label()` then `add_label()` — failing with a clear error if the cache is missing an entry.

Wrote 10 unit tests using `MockLinearClient`: 3 for poll (mapped issues, empty, failure), 4 for transition (success, cache miss, remove failure, add failure), and 3 for ensure_labels (all created, all found, mixed + cache verification).

## Verification

- `cargo test -p smelt-cli --lib -- serve::linear::source` — 10 tests passed
- `cargo test --workspace` — 175 tests passed, zero regressions
- `cargo clippy --workspace -- -D warnings` — clean

## Diagnostics

- `tracing::info!` logs on each label ensured (team_id, label_name, action)
- `tracing::info!` logs on successful state transition (issue_id, from, to)
- Cache miss produces `SmeltError::tracker("transition", "label 'smelt:queued' not in cache — was ensure_labels() called?")`

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/linear/source.rs` — New: LinearTrackerSource struct, ensure_labels(), TrackerSource impl, 10 unit tests
- `crates/smelt-cli/src/serve/linear/mod.rs` — Registered source module, added re-export of LinearTrackerSource
