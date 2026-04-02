---
id: T02
parent: S03
milestone: M012
provides:
  - GithubTrackerSource<G: GhClient> implementing TrackerSource trait
  - poll_ready_issues() with auth check, ready-label filter, GhIssue→TrackerIssue mapping
  - transition_state() with single edit_labels call for atomic label swap (D157)
  - ensure_labels() creating all 6 lifecycle labels idempotently
  - issue_id parsing as u64 for gh CLI compatibility
key_files:
  - crates/smelt-cli/src/serve/github/source.rs
  - crates/smelt-cli/src/serve/github/mod.rs
key_decisions:
  - "Auth failure in poll_ready_issues wraps as SmeltError::Tracker { operation: 'poll' } — distinct from underlying auth_status error for poll-level diagnostics"
patterns_established:
  - "GithubTrackerSource<G: GhClient> generic pattern in serve/github/source.rs — bridges GhClient to TrackerSource with label-prefix-based lifecycle"
observability_surfaces:
  - "tracing::info! on label creation (repo + label fields)"
  - "tracing::info! on successful label transitions (repo + issue + from + to fields)"
  - "SmeltError::Tracker { operation: 'poll' } on auth failures before issue listing"
  - "SmeltError::Tracker { operation: 'transition' } on invalid issue_id parse"
duration: 8min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T02: GithubTrackerSource implementing TrackerSource

**GithubTrackerSource bridges GhClient to TrackerSource with auth-gated polling, GhIssue→TrackerIssue mapping, atomic label transitions, and idempotent lifecycle label creation — 8 unit tests passing**

## What Happened

Created `source.rs` with `GithubTrackerSource<G: GhClient>` struct holding `client`, `repo`, and `label_prefix` fields. Implemented `TrackerSource` trait: `poll_ready_issues()` calls `auth_status()` first (wrapping failure as `SmeltError::Tracker { operation: "poll" }`), then `list_issues()` with the ready label and limit 50, mapping each `GhIssue` to `TrackerIssue` (number→string id, url→source_url). `transition_state()` parses `issue_id` as `u64`, builds from/to label strings via `TrackerState::label_name()`, and calls `edit_labels()` in a single call for D157 atomicity. `ensure_labels()` iterates `TrackerState::ALL` calling `create_label()` per variant. Wired `pub mod source` and `pub use source::GithubTrackerSource` into `mod.rs`.

## Verification

- `cargo test -p smelt-cli --lib -- serve::github::source` — 8 tests passed
- `cargo test -p smelt-cli --lib -- serve::github` — 16 tests passed (8 mock + 8 source)
- `cargo clippy --workspace -- -D warnings` — zero warnings

### Observable Truths
| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | GithubTrackerSource implements TrackerSource | ✓ PASS | Compiles; all trait methods exercised in tests |
| 2 | poll_ready_issues checks auth first | ✓ PASS | test_poll_ready_issues_auth_failure |
| 3 | GhIssue→TrackerIssue mapping correct | ✓ PASS | test_poll_ready_issues_returns_mapped_issues |
| 4 | transition_state uses single edit_labels call | ✓ PASS | test_transition_state_edits_labels |
| 5 | ensure_labels creates all 6 labels | ✓ PASS | test_ensure_labels_creates_all_lifecycle_labels |
| 6 | issue_id parsed as u64 | ✓ PASS | test_transition_state_invalid_issue_id |

## Diagnostics

- Auth failures surface as `SmeltError::Tracker { operation: "poll" }` — check error string for "poll" substring
- Invalid issue IDs surface as `SmeltError::Tracker { operation: "transition" }` with "not a valid u64" message
- Successful transitions logged via `tracing::info!` with repo/issue/from/to fields
- Label creation logged via `tracing::info!` with repo/label fields

## Deviations

- Added `test_transition_state_invalid_issue_id` as a bonus test (not in plan) to verify u64 parse error path
- Total 8 tests instead of 7 — exceeds plan requirement of ≥6

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/github/source.rs` — GithubTrackerSource with TrackerSource impl + 8 unit tests
- `crates/smelt-cli/src/serve/github/mod.rs` — added `pub mod source` and `pub use source::GithubTrackerSource`
