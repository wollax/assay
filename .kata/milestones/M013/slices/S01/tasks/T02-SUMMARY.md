---
id: T02
parent: S01
milestone: M013
provides:
  - Q001: tracing::warn! in GitHubBackend::new for empty or slash-missing repo
  - Q002: zero-guard in read_issue_number rejects issue number 0
  - Q003: GhRunner::gh_error helper extracted; 3 call sites deduplicated
  - Q004: factory.rs doc cleaned of (M011/S02), (M011/S03), (M011/S04)
key_files:
  - crates/assay-backends/src/github.rs
  - crates/assay-backends/src/factory.rs
key_decisions: []
patterns_established:
  - GhRunner::gh_error consolidates warn + error construction for all gh CLI failures
observability_surfaces:
  - "tracing::warn! at construction time for malformed repo (field: repo)"
  - "read_issue_number Err includes '0' and path for diagnosability"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T01:10:00Z
blocker_discovered: false
---

# T02: Implement Q001–Q004 fixes

**All four correctness fixes implemented: malformed-repo warn, zero-guard, `gh_error` helper extraction, factory doc cleanup. All 11 tests pass.**

## What Happened

Implemented four changes in order:

1. **Q003**: Extracted `GhRunner::gh_error(&self, operation, output) -> AssayError` that does `from_utf8_lossy(stderr)` → `tracing::warn!` → `AssayError::io`. Replaced the 3 duplicated error blocks in `create_issue`, `create_comment`, and `get_issue_json` with one-liner calls to `self.gh_error(...)`.

2. **Q001**: Added repo validation in `GitHubBackend::new` — if `repo.is_empty() || !repo.contains('/')`, emits `tracing::warn!(repo, "malformed GitHub repo — expected 'owner/repo' format")`. Constructor stays infallible per D177.

3. **Q002**: Added `if number == 0` guard in `read_issue_number` after the `parse::<u64>()` call. Returns `Err(AssayError::io("invalid issue number: 0 (possible file corruption)", &path, ...))`.

4. **Q004**: Removed `(M011/S02)`, `(M011/S03)`, `(M011/S04)` parentheticals from `backend_from_config` doc comment in `factory.rs`.

## Verification

- `cargo test -p assay-backends --features github` — 11 passed, 0 failed ✓
- Q001: `test_new_warns_on_empty_repo` passes (logs_contain("malformed")) ✓
- Q001: `test_new_warns_on_repo_missing_slash` passes ✓
- Q002: `test_read_issue_number_rejects_zero` passes (Err returned before gh call) ✓
- Q003: `gh_error` helper used by all 3 methods ✓
- Q004: `grep -c '(M011/S' crates/assay-backends/src/factory.rs` → `0` ✓
- `from_utf8_lossy` count: 2 (1 in `gh_error` for stderr, 1 in `create_issue` for stdout URL parsing — the stdout one is not an error-handling duplicate)

### Slice-level verification (intermediate — T03 remains):
- ✅ `cargo test -p assay-backends --features github` — all 11 tests pass
- ⏳ `just ready` — not yet run (T03 scope)
- ✅ `grep -c '(M011/S' factory.rs` → `0`
- ✅ Q001 warn tests pass
- ✅ Q002 issue-0 test passes
- ✅ Q003 refactor complete (`from_utf8_lossy` in error path is 1; stdout parsing is separate)

## Diagnostics

- `RUST_LOG=assay_backends=warn` shows malformed-repo warnings at runtime
- `read_issue_number` error for 0 includes path and value for debugging
- `cargo test -p assay-backends --features github -- --nocapture` shows tracing output

## Deviations

- `from_utf8_lossy` count is 2, not 1 as the slice plan expected. The second occurrence is in `create_issue` for parsing the issue URL from stdout — this is not part of the error-handling duplication that Q003 targets. The plan's grep check was slightly overspecified.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-backends/src/github.rs` — Q001 warn in `new()`, Q002 zero-guard in `read_issue_number`, Q003 `gh_error` helper with 3 call sites
- `crates/assay-backends/src/factory.rs` — removed milestone identifiers from doc comment
