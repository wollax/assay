# S01: GitHubBackend correctness fixes (Q001–Q004) — UAT

**Milestone:** M013
**Written:** 2026-03-28

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All four fixes are internal correctness improvements with no user-visible UI or interactive flow. The contracts are fully exercisable by unit/integration tests with mock subprocess environments. Real `gh` CLI validation is the only remaining gap, which requires an authenticated GitHub environment and is intentionally deferred.

## Preconditions

- `gh` CLI installed and authenticated against a real GitHub repo
- `RUST_LOG=assay_backends=warn` set (optional, for warn visibility)
- A real repo in `owner/repo` format available for testing

## Smoke Test

```
cargo test -p assay-backends --features github
```

All 11 tests pass. No failures. No warnings emitted from valid `owner/repo` input.

## Test Cases

### 1. Q001 — warn on malformed repo at construction

1. Set `RUST_LOG=assay_backends=warn`
2. Construct a `GitHubBackend` with `repo = ""` or `repo = "noslash"`
3. **Expected:** `WARN assay_backends::github: malformed GitHub repo — expected 'owner/repo' format repo=...` appears in log output. No panic, no error returned — constructor stays infallible.

### 2. Q002 — zero-guard in read_issue_number

1. Write `"0"` to `.github-issue-number` in a temp assay dir
2. Call any operation that triggers `read_run_state` (e.g. `push_session_event` on an already-created issue, or a direct test)
3. **Expected:** Returns `Err` with message containing `"invalid issue number: 0 (possible file corruption)"` and the file path. No `gh issue view 0` subprocess is spawned.

### 3. Q003 — gh_error error shape consistency

1. Set up a mock `gh` binary that exits 1 with a known stderr message
2. Call `create_issue`, `create_comment`, and `get_issue_json` so the mock fails
3. **Expected:** All three return `AssayError::Io` with the stderr message included. A `WARN` log event is emitted for each failure. Error shapes are identical across all three methods.

### 4. Q004 — factory.rs doc clean

1. Run: `grep '(M011/S' crates/assay-backends/src/factory.rs`
2. **Expected:** No output (zero matches).

## Edge Cases

### Valid owner/repo format — no warn

1. Construct `GitHubBackend::new("owner/repo", None)`
2. **Expected:** No warn emitted. Constructor succeeds silently.

### Multiple slashes — valid

1. Construct `GitHubBackend::new("owner/repo/extra", None)` (contains at least one `/`)
2. **Expected:** No warn emitted (the guard is `!repo.contains('/')` — one slash is sufficient).

### Issue number 1 — accepted

1. Write `"1"` to `.github-issue-number`
2. Call `read_run_state`
3. **Expected:** Proceeds to `gh issue view 1` — no early rejection.

## Failure Signals

- If `test_new_warns_on_empty_repo` fails: Q001 warn is not being emitted — check `GitHubBackend::new` for the repo validation block
- If `test_read_issue_number_rejects_zero` fails: zero-guard is missing or the error message doesn't match the expected pattern — check `read_issue_number` for `if number == 0`
- If `grep -c '(M011/S' crates/assay-backends/src/factory.rs` returns > 0: Q004 cleanup was not applied or was reverted

## Requirements Proved By This UAT

- R081 (GitHubBackend construction validation) — Q001 warn, Q002 zero rejection, Q003 error helper, Q004 doc cleanup all verified by contract tests with mock gh environment

## Not Proven By This UAT

- Real `gh` CLI interaction — tests use mock subprocess environments; behavior with the actual GitHub API is not validated here
- Performance characteristics of malformed-repo detection at scale
- That downstream callers of `GitHubBackend::new` with malformed repos actually see and act on the warn log (advisory only per D177 infallible constructor constraint)

## Notes for Tester

- The `#[traced_test]` + `logs_contain()` tests capture tracing events in-process. They do not require a running tracing subscriber in production — only in test context.
- The `from_utf8_lossy` grep count will return 2, not 1 — this is expected and documented. The second occurrence (stdout URL parsing in `create_issue`) is unrelated to Q003.
- Q001/Q002 tests do not require `#[serial]` because they don't mutate PATH or global state.
