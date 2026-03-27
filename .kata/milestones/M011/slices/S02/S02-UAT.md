# S02: LinearBackend — UAT

**Milestone:** M011
**Written:** 2026-03-27

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Contract tests with mockito prove all 7 StateBackend method implementations and correct GraphQL request shapes. Real Linear API validation requires external credentials and a test project — this is UAT-only and cannot be automated in CI.

## Preconditions

1. `LINEAR_API_KEY` env var set to a valid Linear personal API key
2. A Linear team exists with at least one project (note the team ID)
3. `cargo build -p assay-backends --features linear` succeeds
4. `just ready` is green (baseline check)

## Smoke Test

```bash
LINEAR_API_KEY=<your-key> cargo test -p assay-backends --features linear -- --nocapture
```

All 8 contract tests should pass with mockito server output showing GraphQL request shapes.

## Test Cases

### 1. First push creates a Linear issue

1. Set `LINEAR_API_KEY` to a valid key
2. Construct a `LinearBackend` with a real `team_id` pointing to a test project
3. Create a temp dir as `run_dir`
4. Call `push_session_event(run_id, sessions)` — first call
5. **Expected:** A new Linear issue appears in the team's project with title = `run_id`; `.linear-issue-id` file written to `run_dir` containing the Linear issue ID

### 2. Subsequent push appends a comment

1. Continue from test case 1 (`.linear-issue-id` exists in run_dir)
2. Call `push_session_event(run_id, sessions)` again with updated session data
3. **Expected:** A new comment appears on the existing Linear issue with serialized `OrchestratorStatus` JSON; `.linear-issue-id` file unchanged

### 3. read_run_state deserializes latest comment

1. Continue from test case 2
2. Call `read_run_state(run_id)`
3. **Expected:** Returns `Ok(Some(OrchestratorStatus { ... }))` with the status from the last comment body

### 4. annotate_run posts tagged manifest comment

1. Call `annotate_run(run_id, manifest)` where manifest is a `RunManifest`
2. **Expected:** A new comment appears on the Linear issue with body starting with `[assay:manifest]` followed by JSON

### 5. read_run_state when no issue exists

1. Create a fresh temp dir (no `.linear-issue-id`)
2. Call `read_run_state(run_id)` on a backend pointing to that dir
3. **Expected:** Returns `Ok(None)` — no error, no panic

### 6. Construction without LINEAR_API_KEY

1. Unset `LINEAR_API_KEY` in env
2. Call `LinearBackend::from_env(team_id, project_id, run_dir)`
3. **Expected:** Returns an error with message containing "LINEAR_API_KEY not set"

## Edge Cases

### Missing team_id produces GraphQL error

1. Construct `LinearBackend::new("invalid-team-id-xyz", ...)` with a valid API key
2. Call `push_session_event(...)`
3. **Expected:** Returns an error with a meaningful GraphQL error message; no panic; Linear API error surfaced via `AssayError::Io`

### API key missing from factory

1. Unset `LINEAR_API_KEY`
2. Call `backend_from_config(StateBackendConfig::Linear { team_id, project_id }, assay_dir)`
3. **Expected:** Returns `Arc<NoopBackend>` (not an error), and a `warn!` log line about missing LINEAR_API_KEY

## Failure Signals

- `reqwest::Error` in output — network or auth failure
- `GraphQL error: Entity not found` — invalid team_id or project_id
- `LINEAR_API_KEY not set` — credential not in environment
- `.linear-issue-id` absent after first push — issue creation silently failed
- `read_run_state` returns deserialization error — `annotate_run` was called after last push (ordering issue)

## Requirements Proved By This UAT

- R076 — `LinearBackend::push_session_event` creates a Linear issue on first call and appends a comment on subsequent calls against the real api.linear.app endpoint; `read_run_state` fetches and deserializes the latest comment; `LINEAR_API_KEY` enforced; real issue/comment IDs confirmed

## Not Proven By This UAT

- GitHubBackend and SshSyncBackend (R077, R078) — separate slices S03/S04
- Multi-machine smelt integration end-to-end — UAT deferred beyond M011
- LinearBackend messaging (send_message/poll_inbox) — capabilities=false; deferred to M012+
- Checkpoint persistence via LinearBackend — deferred to M012+
- `read_run_state` ordering robustness when `annotate_run` interleaves with push events — known limitation, no automated guard

## Notes for Tester

- Use a dedicated test Linear team/project to avoid polluting real project issue trackers
- The `.linear-issue-id` file persists in the run_dir between test runs — delete it to re-test first-push behavior
- `LINEAR_API_KEY` should never appear in any log output — verify by running with `RUST_LOG=debug` and searching output
- If the API key has insufficient permissions, issue creation will fail with a GraphQL auth error (not an HTTP 401)
