---
id: T02
parent: S02
milestone: M011
provides:
  - LinearClient GraphQL HTTP wrapper with create_issue, create_comment, get_latest_comment
  - LinearBackend implementing all 7 StateBackend methods behind cfg(feature = "linear")
  - Factory dispatch Linear → LinearBackend with LINEAR_API_KEY env-var fallback to NoopBackend
key_files:
  - crates/assay-backends/src/linear.rs
  - crates/assay-backends/src/factory.rs
  - crates/assay-backends/src/lib.rs
key_decisions:
  - LinearClient appends /graphql to base_url (base_url is root without path suffix) — aligns with test mock pattern of server.url() + POST /graphql
  - from_env constructor validates LINEAR_API_KEY; factory gracefully falls back to NoopBackend when key is absent (no panic)
  - Authorization header set as default header with set_sensitive(true) — reqwest redacts it from debug output
patterns_established:
  - GraphQL client pattern — struct wraps reqwest::blocking::Client, graphql() method checks for errors array in 200 responses
  - .linear-issue-id file lifecycle — write on first push_session_event, read on subsequent calls
observability_surfaces:
  - tracing::info! on issue creation (logs issue_id) and comment creation
  - tracing::warn! on GraphQL error responses
  - tracing::debug! on HTTP request URL
  - AssayError::Io with operation labels for all failure modes
duration: 12m
verification_result: passed
completed_at: 2026-03-27
blocker_discovered: false
---

# T02: Implement LinearClient and LinearBackend

**Implemented LinearClient GraphQL wrapper and LinearBackend with all 7 StateBackend methods; all 8 contract tests pass; factory dispatches Linear to LinearBackend.**

## What Happened

Created `crates/assay-backends/src/linear.rs` with two components:

1. **LinearClient** — internal struct wrapping `reqwest::blocking::Client` with configurable `base_url`. Provides `create_issue()`, `create_comment()`, and `get_latest_comment()` methods using validated GraphQL mutation/query shapes from S02-RESEARCH. The `graphql()` method checks for GraphQL-level errors (200 with `errors` array) and surfaces them as `AssayError::Io`.

2. **LinearBackend** — public struct implementing `StateBackend`. `push_session_event` creates an issue on first call (writes ID to `.linear-issue-id`), appends a comment with serialized `OrchestratorStatus` JSON on subsequent calls. `read_run_state` fetches the latest comment and deserializes it. `annotate_run` posts a `[assay:manifest]` tagged comment. `send_message`, `poll_inbox`, `save_checkpoint_summary` return errors (unsupported). `capabilities()` returns D164 flags.

Updated `factory.rs` to dispatch `Linear` → `LinearBackend::new()` when `LINEAR_API_KEY` is set, falling back to `NoopBackend` with a warning when absent. Added `#[cfg(feature = "linear")] pub mod linear` to `lib.rs`.

## Verification

- `cargo test -p assay-backends --features linear` — all 8 contract tests pass ✅
- `cargo test -p assay-backends` — all 5 factory tests pass ✅
- `cargo clippy -p assay-backends --features linear` — zero warnings ✅
- `just ready` — 1499 tests pass, zero regression ✅

Slice-level verification (all pass — this is the final task):
- ✅ push_session_event first call creates issue (test_push_first_event_creates_issue)
- ✅ push_session_event subsequent call creates comment (test_push_subsequent_event_creates_comment)
- ✅ read_run_state deserializes latest comment (test_read_run_state_deserializes_latest_comment)
- ✅ annotate_run posts [assay:manifest] comment (test_annotate_run_posts_tagged_comment)
- ✅ capabilities() returns D164 flags (test_capabilities_returns_d164_flags)
- ✅ read_run_state returns Ok(None) when no issue (test_read_run_state_returns_none_when_no_issue)
- ✅ construction fails without LINEAR_API_KEY (test_construction_fails_without_api_key)
- ✅ GraphQL error handling (test_push_handles_graphql_error_response)

## Diagnostics

- `.linear-issue-id` file in run_dir maps each run to a Linear issue
- `read_run_state` returns the latest status from Linear comments
- Error messages include operation context: "creating Linear issue", "creating comment", "LINEAR_API_KEY not set"
- GraphQL errors embedded in `AssayError::Io` with the Linear error message
- `LINEAR_API_KEY` never logged — only referenced by name

## Deviations

- `base_url` convention: LinearClient takes a root URL (e.g. `https://api.linear.app`) and appends `/graphql` internally, rather than taking the full GraphQL endpoint URL. This aligns with the test pattern where `mockito::Server::url()` returns the root and mocks are set up on `POST /graphql`.
- Added `Debug` impl for `LinearBackend` manually (required by test's `unwrap_err()`) with redacted client fields.
- Applied clippy's `io_other_error` suggestion: `std::io::Error::other()` instead of `std::io::Error::new(ErrorKind::Other, ...)`.

## Known Issues

- `read_run_state` fetches the latest comment regardless of content. If `annotate_run` is called after the last `push_session_event`, the latest comment will be an annotation, not an `OrchestratorStatus` JSON, and deserialization will fail. Documented in S02-RESEARCH as a known risk.

## Files Created/Modified

- `crates/assay-backends/src/linear.rs` — New: LinearClient + LinearBackend implementation (~250 lines)
- `crates/assay-backends/src/lib.rs` — Added `#[cfg(feature = "linear")] pub mod linear`
- `crates/assay-backends/src/factory.rs` — Updated Linear arm to dispatch to LinearBackend; updated factory test
