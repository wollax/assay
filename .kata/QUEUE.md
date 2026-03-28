# Kata Queue

<!-- Append-only log of queued work items. Never edit or remove existing entries.
     To cancel an item, add a new entry superseding it.
     Format: ## [QNN] Title — one entry per item, appended in order. -->

---

## [Q001] GitHubBackend: validate repo format at construction time

**Queued:** 2026-03-27
**Source:** PR #193 review backlog
**Scope:** `crates/assay-backends/src/github.rs` — `GitHubBackend::new`

`GitHubBackend::new` accepts `repo = ""` or `repo = "no-slash"` silently. These fail
at subprocess time with a confusing `gh` error rather than at construction. Options:

1. Return `Result<Self>` and validate `owner/repo` format at construction.
2. Keep infallible constructor but add `tracing::warn!` when `repo` is empty or
   missing a `/` — low-cost runtime signal during development.

Also: add explicit `GhRunner::new(repo: String) -> Self` constructor so validation
has a single home when it's added.

---

## [Q002] GitHubBackend: reject issue number 0 in read_issue_number

**Queued:** 2026-03-27
**Source:** PR #193 review backlog
**Scope:** `crates/assay-backends/src/github.rs` — `read_issue_number`

GitHub issue numbers start at 1. If `.github-issue-number` contains `"0"` (file
corruption, hand-edit, or future bug), `read_issue_number` returns `Ok(Some(0))`
and `get_issue_json` runs `gh issue view 0 --repo ...`, producing a runtime error
from `gh`. Add a post-parse guard:

```rust
if number == 0 {
    return Err(AssayError::io(
        "parsing .github-issue-number",
        &path,
        std::io::Error::new(std::io::ErrorKind::InvalidData, "issue number 0 is invalid"),
    ));
}
```

---

## [Q003] GitHubBackend: extract repeated stderr-capture error pattern in GhRunner

**Queued:** 2026-03-27
**Source:** PR #193 review backlog
**Scope:** `crates/assay-backends/src/github.rs` — `GhRunner` methods

All three `GhRunner` methods (`create_issue`, `create_comment`, `get_issue_json`)
have identical non-zero-exit error handling: capture stderr, `tracing::warn!`,
return `Err(AssayError::io(...))`. Extract a helper to reduce duplication:

```rust
fn gh_error(operation: &str, status: &std::process::ExitStatus, stderr: &str) -> assay_core::Error {
    tracing::warn!(exit_code = status.code(), stderr = %stderr, "{operation} failed");
    AssayError::io(format!("{operation} failed: {stderr}"), "gh", std::io::Error::other(stderr.to_string()))
}
```

---

## [Q004] factory.rs: remove milestone identifiers from public API doc

**Queued:** 2026-03-27
**Source:** PR #193 review backlog
**Scope:** `crates/assay-backends/src/factory.rs` — `backend_from_config` doc comment

The doc comment contains planning artefacts `(M011/S02)`, `(M011/S03)`, `(M011/S04)`
that add no value to crate consumers and will silently go stale as work progresses.
Remove the milestone identifiers from the three bullet points in the function doc.

## [Q005] GitHubBackend: add tracing to silent fallback paths in read_run_state

**Queued:** 2026-03-28
**Source:** PR #197 review (finding #1)
**Scope:** `crates/assay-backends/src/github.rs` — `read_run_state`

The comment→issue-body fallback chain in `read_run_state` silently collapses
structurally invalid JSON into `Ok(None)`. When `gh issue view` returns unexpected
JSON (missing `"comments"` key, not an array, `"body"` not a string), every
`.and_then` folds to `None` and the method returns `Ok(None)` as if no state exists.

Add `tracing::debug!` when taking the fallback path (legitimate first-push case)
and `tracing::warn!` when neither comment body nor issue body yields usable data.

---

## [Q006] GitHubBackend: include repo in AssayError returned by gh_error

**Queued:** 2026-03-28
**Source:** PR #197 review (finding #2)
**Scope:** `crates/assay-backends/src/github.rs` — `gh_error`

`gh_error` includes `repo` in the `tracing::warn!` but not in the returned
`AssayError` message. In multi-repo setups the user sees
`"gh issue create failed: HTTP 422"` with no indication which repo. Fix:

```rust
format!("{operation} failed for repo '{}': {stderr}", self.repo)
```

---

## [Q007] GitHubBackend: add tracing::warn on URL parse failure in create_issue

**Queued:** 2026-03-28
**Source:** PR #197 review (finding #3)
**Scope:** `crates/assay-backends/src/github.rs` — `create_issue`

When `gh issue create` succeeds but returns unexpected stdout, the
`ParseIntError` from `.parse::<u64>()` is silently discarded via `.ok()`.
This is the only gh error path without a `tracing::warn!`. Add a warn log
before returning the error, including `repo` and `raw_output` fields.

---

## [Q008] GitHubBackend: add tracing::debug on "assay-run" title fallback

**Queued:** 2026-03-28
**Source:** PR #197 review (finding #4)
**Scope:** `crates/assay-backends/src/github.rs` — `push_session_event`

When `run_dir.file_name()` returns `None`, the issue title silently falls back
to `"assay-run"`. Add `tracing::debug!` with the `run_dir` display value for
traceability.

---

## [Q009] factory.rs: add #[traced_test] for NoopBackend fallback warning

**Queued:** 2026-03-28
**Source:** PR #197 review (finding #5)
**Scope:** `crates/assay-backends/src/factory.rs` — tests

The feature-gated `NoopBackend` fallback emits `tracing::warn!` correctly but
no test asserts the warning. Add `#[traced_test]` + `logs_contain("falling back
to NoopBackend")` on `factory_github_capabilities` (and linear equivalent) when
the corresponding feature is disabled.
