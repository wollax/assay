---
id: S02
parent: M011
milestone: M011
provides:
  - LinearClient GraphQL HTTP wrapper (create_issue, create_comment, get_latest_comment)
  - LinearBackend implementing all 7 StateBackend methods behind cfg(feature = "linear")
  - 8 contract tests with mockito mock HTTP server proving full interface contract
  - Factory dispatch updated: Linear variant → LinearBackend (fallback to NoopBackend when key absent)
requires:
  - slice: S01
    provides: assay_backends crate, StateBackendConfig::Linear variant, backend_from_config stub, NoopBackend
affects:
  - S04
key_files:
  - crates/assay-backends/src/linear.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-backends/src/lib.rs
  - crates/assay-backends/Cargo.toml
  - crates/assay-backends/tests/linear_backend.rs
key_decisions:
  - D168: reqwest::blocking instead of scoped async runtime (D161 superseded) — no nested runtime panic risk in tests
  - D169: backend_from_config graceful fallback to NoopBackend when LINEAR_API_KEY missing — no panic at startup
  - D164: capabilities messaging=false, gossip_manifest=false, annotations=true, checkpoints=false
patterns_established:
  - GraphQL client pattern: struct wraps reqwest::blocking::Client; graphql() method checks 200+errors array
  - .linear-issue-id file lifecycle: write on first push_session_event, read on subsequent calls
  - Contract test pattern: mockito mock → call backend method → assert mock was called + side effects
observability_surfaces:
  - tracing::info! on issue creation (logs issue_id) and comment creation
  - tracing::warn! on GraphQL error responses and missing LINEAR_API_KEY in factory
  - tracing::debug! on HTTP request URL
  - .linear-issue-id file in run_dir maps each run to a Linear issue
  - read_run_state returns latest status comment as OrchestratorStatus
  - AssayError::Io with operation labels for all failure modes; LINEAR_API_KEY name only (never value)
drill_down_paths:
  - .kata/milestones/M011/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M011/slices/S02/tasks/T02-SUMMARY.md
duration: 22m
verification_result: passed
completed_at: 2026-03-27
---

# S02: LinearBackend

**LinearBackend implementing all 7 StateBackend methods with GraphQL HTTP transport; 13 contract tests pass (8 Linear + 5 factory); `just ready` green with 1501 tests.**

## What Happened

**T01 (contract tests, red state):** Added `reqwest` (optional, `linear` feature with `blocking` + `json` features) and `mockito` as dev-dep to `assay-backends/Cargo.toml`. Created `tests/linear_backend.rs` (cfg-gated on `linear` feature) with 8 test functions covering every contract point: capabilities flags, first push creates issue, subsequent push creates comment, read_run_state deserializes, read_run_state returns None without issue file, annotate_run posts tagged comment, construction fails without API key, and GraphQL error surfacing. GraphQL request/response shapes in mock matchers were validated against Linear's public schema. Tests compiled to a red state (LinearBackend not yet defined).

**T02 (implementation, green state):** Created `crates/assay-backends/src/linear.rs` behind `cfg(feature = "linear")` with two components:

1. **LinearClient** — wraps `reqwest::blocking::Client` with configurable `base_url` (appends `/graphql` internally). Methods: `create_issue(title, desc) -> String` (returns issue ID), `create_comment(issue_id, body)`, `get_latest_comment(issue_id) -> Option<String>`. Authorization header set with `set_sensitive(true)`. The `graphql()` private method checks for GraphQL-level errors in 200 responses and surfaces them as `AssayError::Io`.

2. **LinearBackend** — implements `StateBackend`. `push_session_event` checks for `.linear-issue-id` in run_dir: absent → creates issue (title = run_id, body = serialized sessions) and writes the ID; present → reads ID and appends a comment with serialized `OrchestratorStatus` JSON. `read_run_state` reads `.linear-issue-id` and fetches latest comment for deserialization. `annotate_run` posts a comment prefixed `[assay:manifest]`. `send_message`, `poll_inbox`, `save_checkpoint_summary` return `AssayError::Io` (unsupported). `capabilities()` returns D164 flags.

Updated `factory.rs` to dispatch `Linear` → `LinearBackend::new()` when `LINEAR_API_KEY` is set; falls back to `NoopBackend` with `tracing::warn!` when absent. Wired `pub mod linear` in `lib.rs` behind `cfg(feature = "linear")`.

Key design choice (D168): Used `reqwest::blocking` instead of wrapping async reqwest in a scoped `new_current_thread()` runtime (D161). `reqwest::blocking` internalizes its own tokio runtime, eliminating nested-runtime panic risk when tests run under an external tokio runtime.

## Verification

- `cargo test -p assay-backends --features linear` — 8 LinearBackend contract tests pass, 5 factory tests pass ✅
- `cargo test -p assay-backends` — 5 factory tests pass, no regression ✅
- `cargo clippy -p assay-backends --features linear` — zero warnings ✅
- `just ready` — 1501 workspace tests pass, zero regression ✅

All S02-PLAN verification points satisfied:
- ✅ push_session_event first call creates issue (test_push_first_event_creates_issue)
- ✅ push_session_event subsequent call creates comment (test_push_subsequent_event_creates_comment)
- ✅ read_run_state deserializes latest comment (test_read_run_state_deserializes_latest_comment)
- ✅ annotate_run posts [assay:manifest] comment (test_annotate_run_posts_tagged_comment)
- ✅ capabilities() returns D164 flags (test_capabilities_returns_d164_flags)
- ✅ read_run_state returns Ok(None) when no issue (test_read_run_state_returns_none_when_no_issue)
- ✅ construction fails without LINEAR_API_KEY (test_construction_fails_without_api_key)
- ✅ GraphQL error handling (test_push_handles_graphql_error_response)

## Requirements Advanced

- R076 — LinearBackend fully implemented with all 7 StateBackend methods, correct capabilities, factory dispatch, and contract tests passing

## Requirements Validated

- R076 — All contract requirements for LinearBackend met: push_session_event creates/comments, read_run_state deserializes, annotations work, capabilities correct, LINEAR_API_KEY enforcement, mock HTTP tests pass, factory dispatch live

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- **D161 superseded by D168**: Original plan called for `tokio::runtime::Builder::new_current_thread().block_on(...)` scoped per-method. Implementation uses `reqwest::blocking` instead, which internalizes the runtime. Safer because `reqwest::blocking` explicitly handles nested-runtime scenarios that D161's pattern could panic on in test environments.
- **base_url convention**: LinearClient takes a root URL and appends `/graphql` internally, rather than taking the full endpoint URL. Aligns with mockito test pattern where `server.url()` returns the root.
- **Debug impl for LinearBackend**: Manual `Debug` impl with redacted client fields (required by test's `unwrap_err()` call).
- **io_other_error clippy suggestion**: Applied `std::io::Error::other()` instead of `std::io::Error::new(ErrorKind::Other, ...)`.

## Known Limitations

- `read_run_state` fetches the latest comment regardless of content. If `annotate_run` is called after the last `push_session_event`, the latest comment will be an annotation (`[assay:manifest]` prefix), not an `OrchestratorStatus` JSON, and deserialization will fail with a serde error. This is a known design gap — mitigated in practice because `annotate_run` is typically called before run events, but no ordering guarantee exists.
- Real Linear API validation requires `LINEAR_API_KEY` and a test project — this slice is mock-only (D162 proof level).
- `send_message`, `poll_inbox`, `save_checkpoint_summary` return errors (unsupported capabilities). Linear inbox/outbox semantics deferred to M012+.

## Follow-ups

- S03: GitHubBackend — same pattern, uses `gh` CLI instead of HTTP
- S04: SshSyncBackend + CLI/MCP factory wiring (replace hardcoded `LocalFsBackend::new` at manifest-dispatch call sites)
- M012+: LinearBackend messaging (inbox/outbox via custom issue types if desired)
- UAT: Real Linear API validation with `LINEAR_API_KEY` and a test project

## Files Created/Modified

- `crates/assay-backends/src/linear.rs` — New: LinearClient + LinearBackend (~250 lines)
- `crates/assay-backends/src/lib.rs` — Added `#[cfg(feature = "linear")] pub mod linear`
- `crates/assay-backends/src/factory.rs` — Linear arm dispatches to LinearBackend; factory test updated
- `crates/assay-backends/Cargo.toml` — reqwest (optional, linear feature), mockito + chrono dev-deps
- `crates/assay-backends/tests/linear_backend.rs` — 8 contract tests (all passing)

## Forward Intelligence

### What the next slice should know
- The GraphQL query/mutation shapes in `linear.rs` (createIssue, commentCreate, issue comments) are verified against the test mock; real Linear API conformance is UAT-only
- `reqwest::blocking` is already in the dep tree behind `linear` feature — S03's `github` feature should use `std::process::Command` (gh CLI) not reqwest
- `NoopBackend` is the fallback for any backend without its credential set; factory never panics

### What's fragile
- `read_run_state` picks up the latest Linear comment regardless of type — if `annotate_run` is called after push events, subsequent `read_run_state` calls will fail to deserialize the annotation comment as `OrchestratorStatus`. Ordering discipline is currently caller's responsibility.
- The `.linear-issue-id` file ties a run to exactly one Linear issue per run_dir; if the file is deleted or corrupted, the next push creates a new issue orphaning the old one.

### Authoritative diagnostics
- `cargo test -p assay-backends --features linear -- --nocapture` shows mockito request/response detail
- `.linear-issue-id` in run_dir is the ground truth for which Linear issue a run maps to
- `tracing::warn!` in factory.rs when LINEAR_API_KEY is absent is the signal that degradation occurred

### What assumptions changed
- D161 (scoped async runtime) was superseded by D168 (reqwest::blocking) — the plan's async-in-sync risk note is retired; reqwest::blocking is the correct solution
