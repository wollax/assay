# S05: Dispatch Integration, State Backend Passthrough & Final Assembly — UAT

**Milestone:** M012
**Written:** 2026-03-28

## UAT Type

- UAT mode: mixed (artifact-driven for state_backend serialization + poller unit tests; live-runtime for real GitHub/Linear end-to-end)
- Why this mode is sufficient: The automated test suite proves the integration contract — TrackerPoller lifecycle, label transition sequencing (D157), manifest generation, ServerState enqueue, and state_backend TOML output. Real-world UAT below is required for GitHub (`gh` CLI) and Linear (GraphQL API) operational verification, which cannot be mocked at unit level.

## Preconditions

**For automated verification (already completed):**
- `cargo test --workspace` passes 398 tests

**For live GitHub UAT:**
- `gh` CLI installed and authenticated (`gh auth status` passes)
- A GitHub repo with `smelt:ready` label created
- `server.toml` with `[tracker]` section pointing to the repo and a valid `manifest_template` path
- Valid `manifest_template.toml` with zero `[[session]]` entries and correct `[environment]` / `[forge]` config

**For live Linear UAT:**
- `LINEAR_API_KEY` env var set with a valid API key
- A Linear project with at least one issue
- `server.toml` with `[tracker]` provider = "linear" and the correct `project_id` / `team_key`

## Smoke Test

Run `smelt serve` with a `[tracker]` section configured and observe the startup log:

```
SMELT_LOG=info smelt serve --config server.toml
```

Expected: log line showing `"Tracker poller configured"` with provider name and poll interval.

## Test Cases

### 1. state_backend passthrough (automated — already proven)

1. Create a `job-manifest.toml` with:
   ```toml
   [state_backend.linear]
   team_id = "TEAM-1"
   project_id = "PROJ-1"
   ```
2. Submit via `POST /api/v1/jobs`
3. **Expected:** The generated RunManifest TOML inside the container contains a `[state_backend.linear]` section with `team_id` and `project_id`. `build_run_manifest_toml()` unit tests confirm this (T01 verification).

### 2. GitHub tracker end-to-end (live UAT)

1. Create a GitHub Issue in the configured repo
2. Apply the `smelt:ready` label to the issue
3. Start `smelt serve` with `[tracker]` section pointing to the repo
4. Wait one poll interval (default 30s)
5. **Expected:**
   - Issue label transitions: `smelt:ready` removed, `smelt:queued` added (before dispatch)
   - A job appears in the TUI with Source = "Tracker"
   - `GET /api/v1/jobs` returns the job with `"source": "Tracker"`
   - When dispatch completes: label transitions to `smelt:running`, then `smelt:pr-created`

### 3. Linear tracker end-to-end (live UAT)

1. Create a Linear issue in the configured project
2. Apply the `smelt:ready` label to the issue
3. Start `smelt serve` with `[tracker]` provider = "linear"
4. Wait one poll interval
5. **Expected:**
   - Issue label changes from `smelt:ready` → `smelt:queued` before enqueue
   - Job appears in TUI with Source = "Tracker"
   - Label transitions through running → pr-created lifecycle

### 4. TUI Source column (automated — already proven)

1. Enqueue jobs from three sources: HTTP API, directory watch, and MockTrackerSource
2. **Expected:** TUI shows "HTTP", "DirWatch", "Tracker" in the Source column respectively (proven by `test_tui_render_tracker_source`, `test_tui_render_dirwatch_source`, `test_tui_render_worker_host` TUI tests).

### 5. Double-dispatch prevention (D157)

1. Run two `smelt serve` instances simultaneously against the same GitHub repo
2. File an issue with `smelt:ready`
3. **Expected:** Only one instance picks up the issue. The instance that wins the `gh issue edit --remove-label smelt:ready --add-label smelt:queued` race proceeds; the other's next poll finds no issues with `smelt:ready` and skips.

## Edge Cases

### Tracker not configured (None path)

1. Start `smelt serve` with a `server.toml` that has no `[tracker]` section
2. **Expected:** Serve starts normally, no tracker log lines, `pending()` arm never fires, all existing functionality unchanged.

### ensure_labels failure at startup

1. Start `smelt serve` with an invalid `LINEAR_API_KEY`
2. **Expected:** `tracing::error!` log indicating label provisioning failure; serve exits (poller startup is fatal).

### Poll error resilience

1. During a running `smelt serve` session with GitHub tracker, revoke `gh` auth temporarily
2. **Expected:** `tracing::warn!` log for each failed poll cycle; poller continues; when auth is restored, issues are picked up on the next cycle (D172).

### state_backend absent (backward compat)

1. Submit a job manifest with no `[state_backend]` section
2. **Expected:** RunManifest TOML contains no `[state_backend]` section; no TOML parsing errors; all existing tests still pass (proven by T01 `test_run_manifest_no_state_backend_when_none`).

## Failure Signals

- No "Tracker poller configured" info log at serve startup → tracker section not parsed correctly
- Issue remains with `smelt:ready` label after two poll intervals → poller not running or transition failing; check `SMELT_LOG=warn` output
- Job appears in TUI but Source column shows "HTTP" or "DirWatch" instead of "Tracker" → JobSource::Tracker not being set by TrackerPoller
- `[state_backend]` section missing from generated RunManifest → build_run_manifest_toml() not cloning state_backend; check assay.rs
- Two instances both pick up same issue → D157 label transition race; check gh CLI rate limits and timing

## Requirements Proved By This UAT

- R075 — State backend passthrough in JobManifest: proven by automated unit tests in T01; `build_run_manifest_toml()` with Linear/LocalFs/None variants all produce correct TOML; `[state_backend.linear]` section with team_id/project_id appears exactly as specified
- R070 (partial) — GitHub tracker dispatch: end-to-end flow proven with MockTrackerSource through all stages (poll → transition → manifest → enqueue); real `gh` CLI lifecycle requires live UAT in steps 2 above
- R071 (partial) — Linear tracker dispatch: end-to-end flow proven with MockTrackerSource; real GraphQL API requires live UAT in step 3 above
- R072 — TrackerSource trait abstraction: AnyTrackerSource enum dispatches GitHub/Linear/Mock variants; poller works identically regardless of backend (proven by MockTrackerSource tests)
- R073 — Template manifest with issue injection: `issue_to_manifest()` merges template + issue title/body → session entry; proven by T02 `test_build_manifest_toml_roundtrips`
- R074 — Label-based lifecycle state machine: Ready→Queued transition before enqueue proven by T02; full lifecycle (running, pr-created, done) requires live UAT

## Not Proven By This UAT

- Real `gh` CLI auth flow, rate limiting, and network failure resilience — requires live GitHub credentials and network
- Real Linear GraphQL API with auth, rate limiting, and label UUID caching across a multi-hour session — requires real `LINEAR_API_KEY`
- Full lifecycle label transitions (smelt:running → smelt:pr-created → smelt:done) — those are set by the dispatch pipeline after job execution, not by TrackerPoller; require a complete Docker/Assay execution to verify
- Live Docker execution with tracker-dispatched manifests — state_backend TOML passthrough reaches the RunManifest but whether a real Assay binary reads it correctly requires live container test
- Multi-server double-dispatch race condition under concurrent load — unit tests simulate sequential behavior; race conditions require concurrent process execution
- TUI Source column on a real terminal — proven by TestBackend renders; live rendering requires manual inspection

## Notes for Tester

- The `smelt:ready` label must exist in the repo/project before starting the poller (or must be in the label_prefix list for auto-creation via `ensure_labels()`).
- GitHub UAT requires the `gh` binary to be on PATH and authenticated. Use `gh auth status` to verify.
- Linear UAT requires `LINEAR_API_KEY` as an environment variable at `smelt serve` startup — not in `server.toml`. The config stores the env var *name* (`api_key_env`), not the value.
- For the double-dispatch test (case 5), the race window is small. Use `sleep 1` between starting the two instances to make observation easier.
- The `examples/server.toml` `[tracker]` section is fully commented out — uncomment the relevant provider block to enable tracker dispatch.
