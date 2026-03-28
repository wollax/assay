# M012: Tracker-Driven Autonomous Dispatch

**Vision:** `smelt serve` autonomously polls GitHub Issues or Linear for work items, generates manifests from templates, dispatches Assay sessions, creates PRs from results, and transitions tracker state — closing the loop from "file an issue" to "review the PR" with zero human intervention.

## Success Criteria

- `smelt serve` with `[tracker]` config picks up GitHub Issues labeled `smelt:ready` and dispatches them automatically
- `smelt serve` with `[tracker]` config picks up Linear issues and dispatches them automatically
- Issue labels transition through the full lifecycle: ready → queued → running → pr-created → done/failed
- Template manifest provides environment/credentials/merge config; each issue injects only the spec
- Assay state_backend config in the manifest is forwarded into the container's RunManifest
- All existing tests pass (zero regressions); new capabilities have unit and integration tests
- R061 (flaky test) and R062 (tracing migration) are resolved

## Key Risks / Unknowns

- **`gh` CLI operational reliability** — GitHub poller depends on `gh` for issue operations; auth expiry, missing binary, or rate limiting could stall the loop
- **Linear GraphQL complexity** — Label-based lifecycle on Linear requires fetching/creating labels via GraphQL, which is more complex than `gh label`
- **Template manifest validation** — Bad templates must fail at startup, not mid-dispatch; validation surface is new
- **Double-dispatch race** — Two pollers (or two `smelt serve` instances) could pick up the same issue; label transition must be atomic-enough

## Proof Strategy

- **`gh` CLI reliability** → retire in S03 by proving GitHub poller handles missing `gh`, expired auth, and issue listing with label filters
- **Linear GraphQL** → retire in S04 by proving Linear poller creates/reads labels and transitions issue state via GraphQL
- **Template validation** → retire in S02 by proving `ServerConfig::load()` validates the template manifest at startup
- **Double-dispatch** → retire in S03 by proving label transition from `smelt:ready` to `smelt:queued` is the first action before enqueueing

## Verification Classes

- Contract verification: unit tests with mock trackers (MockTrackerSource), template parsing, state_backend serialization
- Integration verification: `gh` CLI integration tests against a real repo; Linear API integration tests (gated by env var)
- Operational verification: `smelt serve` with `[tracker]` runs and picks up issues without manual intervention
- UAT / human verification: file an issue, watch it flow through the lifecycle to a PR

## Milestone Definition of Done

This milestone is complete only when all are true:

- R061 and R062 are resolved (clean tracing, no flaky tests)
- TrackerSource trait implemented with GitHub and Linear backends
- Template manifest loading, validation, and issue injection work
- GitHub end-to-end: issue → dispatch → PR → lifecycle labels
- Linear end-to-end: issue → dispatch → PR → lifecycle state transitions
- State backend passthrough proven (manifest field → RunManifest TOML)
- All 298+ workspace tests pass, plus new tests for tracker functionality
- TUI displays tracker-sourced jobs correctly
- Documentation updated (README, server.toml example)

## Requirement Coverage

- Covers: R026, R061, R062, R070, R071, R072, R073, R074, R075
- Partially covers: none
- Leaves for later: R022 (budget tracking)
- Orphan risks: none

## Slices

- [x] **S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix** `risk:low` `depends:[]`
  > After this: all `eprintln!` calls in smelt-cli are replaced with structured tracing events; `test_cli_run_invalid_manifest` uses a 30s timeout and no longer flakes. 298+ tests pass.

- [x] **S02: TrackerSource Trait, Config, & Template Manifest** `risk:high` `depends:[S01]`
  > After this: `ServerConfig` accepts a `[tracker]` section; `TrackerSource` trait is defined; template manifest loading + validation + issue injection works; `MockTrackerSource` exercises the full trait contract. Proven by unit tests.

- [x] **S03: GitHub Issues Tracker Backend** `risk:medium` `depends:[S02]`
  > After this: `GithubTrackerSource` polls GitHub Issues via `gh` CLI, transitions labels, generates manifests from templates. Proven by unit tests with mock `gh` and integration tests against a real repo (gated by env var).

- [x] **S04: Linear Tracker Backend** `risk:medium` `depends:[S02]`
  > After this: `LinearTrackerSource` polls Linear via GraphQL API, transitions labels, generates manifests from templates. Proven by unit tests with mock HTTP and integration tests against a real Linear project (gated by env var).

- [ ] **S05: Dispatch Integration, State Backend Passthrough & Final Assembly** `risk:medium` `depends:[S03,S04]`
  > After this: TrackerPoller runs inside `smelt serve`'s main loop, enqueuing tracker issues into ServerState; `state_backend` in JobManifest is forwarded into the Assay RunManifest TOML; TUI shows tracker-sourced jobs; server.toml example and README updated. End-to-end proven by integration test.

## Boundary Map

### S01 → S02

Produces:
- Clean tracing infrastructure — all smelt-cli output goes through `tracing` (no `eprintln!`)
- Stable test suite — `test_cli_run_invalid_manifest` no longer flakes

Consumes:
- nothing (cleanup slice)

### S02 → S03, S04

Produces:
- `TrackerSource` trait in `smelt-cli` (`poll_ready_issues()`, `transition_state()`, `issue_to_manifest()`)
- `TrackerConfig` struct in `serve/config.rs` with `provider`, `manifest_template`, `poll_interval_secs`, `label_prefix`, provider-specific config
- `TrackerIssue` struct: `id`, `title`, `body`, `source_url`
- `TemplateManifest` loader: reads base manifest, validates it, injects session from issue
- `MockTrackerSource` for testing
- `JobSource::Tracker` variant in `types.rs`

Consumes from S01:
- Clean tracing (no eprintln conflicts during integration)

### S03 → S05

Produces:
- `GithubTrackerSource: TrackerSource` — `gh issue list --label`, `gh issue edit --add-label/--remove-label`
- `gh` subprocess wrapper with error handling for missing binary / auth

Consumes from S02:
- `TrackerSource` trait contract, `TrackerConfig`, `TrackerIssue`, `TemplateManifest`

### S04 → S05

Produces:
- `LinearTrackerSource: TrackerSource` — GraphQL queries for issue listing, label operations, state transitions
- Linear GraphQL HTTP client (minimal, pattern from Assay's `LinearBackend`)

Consumes from S02:
- `TrackerSource` trait contract, `TrackerConfig`, `TrackerIssue`, `TemplateManifest`

### S05 (terminal)

Produces:
- `TrackerPoller` background task integrated into `dispatch_loop` / `tokio::select!` in `smelt serve`
- `state_backend` field on `JobManifest`, serialized into RunManifest TOML by `AssayInvoker`
- Updated `ServerConfig` with `[tracker]` section
- TUI `Tracker` source column
- Updated `examples/server.toml` and `README.md`

Consumes from S03:
- `GithubTrackerSource`

Consumes from S04:
- `LinearTrackerSource`

Consumes from S02:
- `TrackerSource` trait, `TrackerConfig`, `TemplateManifest`, `JobSource::Tracker`
