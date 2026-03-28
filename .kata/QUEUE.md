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

## [Q010] TUI trace viewer: surface skipped file count in trace list title

**Queued:** 2026-03-28
**Source:** PR #198 review
**Scope:** `crates/assay-tui/src/trace_viewer.rs` — `load_traces`

When bad trace files are skipped (unreadable/unparseable), the user sees fewer
traces with no indication some files were skipped. Return a `(Vec<TraceEntry>, usize)`
tuple with the skip count and show "18 traces (2 unreadable)" in the list title.
S02-PLAN noted this was possible but deferred; the current warn-only approach is
acceptable but not ideal for TUI UX.

---

## [Q011] TUI: `a` and `t` keys silent no-op when project_root is None

**Queued:** 2026-03-28
**Source:** PR #198 review
**Scope:** `crates/assay-tui/src/app.rs` — Dashboard key handlers

Both `a` (Analytics) and `t` (TraceViewer) silently ignore keypresses when
`project_root` is `None`. The `t` handler now has `tracing::debug!` but `a`
does not. Consider transitioning to the screen with an empty-state message
instead of ignoring the keypress — the draw functions already handle empty data.

---

## [Q012] TUI trace viewer: inline D098 doc explanation

**Queued:** 2026-03-28
**Source:** PR #198 review (suggestion)
**Scope:** `crates/assay-tui/src/app.rs` — `handle_trace_viewer_event` doc

The doc references "D098" without explanation. Inline the pattern description
so future readers don't need to consult the decisions register:

```rust
/// Extracted as a method to avoid borrow-splitting issues in the main `handle_event`
/// match (same pattern as `handle_mcp_panel_event` — extracting to a `&mut self` method
/// lets us re-borrow `self.screen` mutably after reading mode from it, D098).
```

## [Q013] OTel metrics: expose metric name strings as pub const

**Queued:** 2026-03-28
**Source:** PR #199 review (suggestion #8)
**Scope:** `crates/assay-core/src/telemetry.rs` — `init_metric_handles`

Metric instrument names (`assay.sessions.launched`, etc.) are string literals
buried inside `init_metric_handles`. A typo or rename would silently break
dashboards/alerts with no test catching it. Extract as `pub const` values so
they can be tested, referenced by documentation, and used by future attribute/
label configuration without duplicating the strings.

---

## [Q014] OTel metrics: test that agent_run_duration is NOT recorded on failure

**Queued:** 2026-03-28
**Source:** PR #199 review (suggestion)
**Scope:** `crates/assay-core/tests/otel_metrics.rs` or pipeline tests

The histogram only records on successful agent runs (the `?` propagates errors
before the recording call). This is an intentional behavioral contract per
T03-SUMMARY, but no test verifies it. A future refactor moving the recording
call inside the closure could silently change behavior. Add a test asserting
agent duration is NOT incremented when the agent stage fails.

---

## [Q015] OTel metrics: test build_otel_metrics returns None when endpoint is absent

**Queued:** 2026-03-28
**Source:** PR #199 review (suggestion)
**Scope:** `crates/assay-core/tests/otel_metrics.rs` or `telemetry.rs` unit tests

`build_otel_metrics` with `otlp_endpoint: None` is the most common path (default
config) but is not directly tested. The existing `test_init_tracing_returns_guard`
covers `TracingConfig::default()` but does not assert `_meter_provider` is `None`.
Add an indirect check (e.g. verify `SESSIONS_LAUNCHED.get()` returns `None` after
`init_tracing(TracingConfig::default())`).

---

## [Q016] OTel metrics: reduce dual-cfg recording function repetition

**Queued:** 2026-03-28
**Source:** PR #199 review (suggestion)
**Scope:** `crates/assay-core/src/telemetry.rs`

Each of the five recording functions is written twice (real + stub), totaling
10 function definitions. Any signature change must be made in two places. Consider
a declarative macro or a `mod metrics_impl` with `pub use metrics_impl::*` pattern
to eliminate duplication. Low-urgency but worth addressing before more metrics are
added.

## [Q017] CriterionOrString: custom deserializer for actionable error messages

**Queued:** 2026-03-28
**Source:** PR #200 review (suggestion #4)
**Scope:** `crates/assay-mcp/src/server.rs` — `CriterionOrString`

With `#[serde(untagged)]`, malformed input (e.g. `42` — a JSON number) produces
a generic `"data did not match any variant of untagged enum CriterionOrString"`
error with no guidance. Replace with a custom `Deserialize` impl or wrapping
validator that produces:

```
"Invalid criterion at index N: expected a string (criterion name) or an object
 with \"name\" (required), \"cmd\" (optional), and \"description\" (optional).
 Got: 42"
```

At minimum, add a schemars doc comment so schema-aware callers get format guidance.

---

## [Q018] TUI wizard: remove unused `_chunk_count` param from `step_prompt`

**Queued:** 2026-03-28
**Source:** PR #200 review (suggestion)
**Scope:** `crates/assay-tui/src/wizard.rs` — `step_prompt`

The `_chunk_count` parameter is explicitly suppressed with a leading underscore
and never used. If not retained for a planned future use, remove it from the
signature and all call sites.

---

## [Q019] MCP: document CriterionInputParam.description → CriterionInput.description mapping

**Queued:** 2026-03-28
**Source:** PR #200 review (suggestion)
**Scope:** `crates/assay-mcp/src/server.rs` — `From<CriterionOrString> for CriterionInput`

`CriterionInputParam.description` is `Option<String>` while `CriterionInput.description`
is a plain `String`. The conversion uses `unwrap_or_default()` to map `None → ""`.
Add a doc comment on the `From` impl noting this intentional mapping, so future
validation on `description` in `CriterionInput` doesn't miss the `None` → `""` case.

---

## [Q020] MCP: add test for malformed CriterionOrString (missing `name` field)

**Queued:** 2026-03-28
**Source:** PR #200 review (test gap)
**Scope:** `crates/assay-mcp/src/server.rs` — unit tests

No test verifies that `{"cmd": "cargo test"}` (object without required `name`)
is correctly rejected by `CriterionOrString` deserialization. A future change
making `name` optional could silently alter the contract. Add:

```rust
#[test]
fn criterion_or_string_rejects_object_without_name() {
    let json = r#"{"cmd": "cargo test"}"#;
    let result = serde_json::from_str::<CriterionOrString>(json);
    assert!(result.is_err());
}
```

---

## [Q021] TUI wizard: end-to-end test from key events through to gates.toml cmd field

**Queued:** 2026-03-28
**Source:** PR #200 review (test gap)
**Scope:** `crates/assay-tui/tests/wizard_round_trip.rs`

`test_wizard_submit_produces_correct_wizard_inputs` tests in-memory struct fields
but not the TOML output. The round-trip test drives `create_from_inputs` but its
assertions check criterion names only — should also assert `cmd` field appears in
the generated `gates.toml` text to close the TUI→TOML gap.
