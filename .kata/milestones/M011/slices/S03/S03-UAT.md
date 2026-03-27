# S03: GitHubBackend — UAT

**Milestone:** M011
**Written:** 2026-03-27

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Contract tests with mock `gh` binary prove the entire arg construction and data flow. Live-runtime UAT is needed only to validate `gh` CLI auth, real GitHub API responses, and issue number parsing from actual GitHub URLs. The mock tests provide mechanistic correctness; UAT closes the real-world gap.

## Preconditions

1. `gh` CLI installed and authenticated (`gh auth status` shows a logged-in account)
2. A test GitHub repository is available (e.g., `myorg/assay-test`)
3. The repository exists and the authenticated user has write access (to create issues)
4. `assay-backends` built with `--features github`
5. A valid `run_dir` temp directory is available for the backend to write `.github-issue-number`

## Smoke Test

Build the crate with the feature enabled and confirm no compile errors:
```
cargo build -p assay-backends --features github
```
Expected: zero errors, zero warnings.

## Test Cases

### 1. First push creates a GitHub issue

1. Create a `GitHubBackend::new("myorg/assay-test", Some("assay-run".to_string()))`
2. Call `push_session_event(run_dir, run_id, &orchestrator_status)` for the first time
3. Check that a new GitHub issue was created in `myorg/assay-test` with label `assay-run`
4. Check that `.github-issue-number` was written in `run_dir` with the issue number
5. **Expected:** Issue visible at `https://github.com/myorg/assay-test/issues/<N>`; `.github-issue-number` contains `<N>`

### 2. Subsequent push appends a comment

1. With `.github-issue-number` already written from Test Case 1
2. Call `push_session_event(run_dir, run_id, &updated_status)` a second time
3. **Expected:** A new comment is added to the existing issue (not a new issue); issue comment count increments by 1

### 3. read_run_state deserializes latest comment

1. After Test Case 2 (issue exists with at least one comment)
2. Call `read_run_state(run_dir, run_id)`
3. **Expected:** Returns `Ok(Some(OrchestratorStatus {...}))` matching the status passed in the most recent `push_session_event` call

### 4. read_run_state returns None for unknown run

1. Create a fresh `run_dir` with no `.github-issue-number` file
2. Call `read_run_state(run_dir, run_id)`
3. **Expected:** Returns `Ok(None)` — no issue mapped to this run

### 5. Issue label applied correctly

1. Inspect the issue created in Test Case 1
2. **Expected:** Issue has the configured label (`assay-run`) applied; label created in the repo if it didn't exist

### 6. capabilities() returns all-false

1. Call `backend.capabilities()` on any `GitHubBackend` instance
2. **Expected:** Returns `CapabilitySet { supports_messaging: false, supports_gossip_manifest: false, supports_annotations: false, supports_checkpoints: false }`

### 7. send_message returns Unsupported error

1. Call `backend.send_message(run_id, "peer", message)`
2. **Expected:** Returns `Err(AssayError::Io { ... })` with an unsupported/capability error (not a panic)

## Edge Cases

### gh not installed

1. Remove `gh` from PATH or test in environment without `gh`
2. Call `push_session_event` or `read_run_state`
3. **Expected:** Returns `Err(AssayError::Io { ... })` with `ErrorKind::NotFound`; clear error message indicating `gh` binary is missing

### gh auth expired

1. Log out of `gh` (`gh auth logout`)
2. Call `push_session_event`
3. **Expected:** Returns `Err` with stderr content from `gh` included in the error message; does not panic

### Repository does not exist

1. Construct `GitHubBackend::new("nonexistent/repo-xyz-99999", None)`
2. Call `push_session_event`
3. **Expected:** Returns `Err` with `gh` stderr about repository not found; `.github-issue-number` not written

### Issue body as fallback (no comments yet)

1. Ensure a GitHub issue exists but has zero comments
2. Call `read_run_state`
3. **Expected:** Returns the issue body deserialized as `OrchestratorStatus` (not `None`)

## Failure Signals

- `push_session_event` panics — implementation error, not expected
- `.github-issue-number` not written after first push — URL parsing failed silently
- Duplicate issues created on each `push_session_event` call — issue file not being read/written
- `read_run_state` returns `None` when issue exists — JSON parsing or comment extraction bug
- Any `tracing::warn!` with stderr content from `gh` — indicates auth or API failure

## Requirements Proved By This UAT

- R077 (GitHubBackend) — UAT proves real `gh` CLI invocation, real GitHub issue creation, comment appending, and `read_run_state` deserialization against a live GitHub repo

## Not Proven By This UAT

- Rate limiting behavior under high event volume (many push_session_event calls)
- Multi-machine smelt scenarios where two backends write to the same issue concurrently
- GitHubBackend behavior when GitHub API is unavailable (network partition)
- Integration with `OrchestratorConfig` in a full assay orchestration run (deferred to S04 wiring + end-to-end UAT)
- `assay-cli` and `assay-mcp` construction sites using `backend_from_config()` (S04 work)

## Notes for Tester

- The `label` parameter is optional — if `None`, no label is applied to the issue. If a label name is given and the label doesn't exist in the repo, `gh` will create it automatically.
- Issue numbers in `.github-issue-number` are stored as plain u64 (no newline confusion — file contains just the number).
- If a test run leaves issues open in the test repo, close them manually or use a dedicated test repo to avoid clutter.
- All `tracing` output is at `debug`/`info`/`warn` level — run with `RUST_LOG=debug` to see command construction and execution details.
