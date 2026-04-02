# M012: Tracker-Driven Autonomous Dispatch — Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel ecosystem. It provisions isolated Docker/Compose/K8s environments, runs Assay sessions inside them, collects result branches, and creates GitHub PRs. `smelt serve` is a parallel dispatch daemon with HTTP API, directory watch, SSH worker pools, persistent queue, and bearer token auth.

M012 closes the autonomous loop: `smelt serve` polls a tracker (GitHub Issues or Linear) for work items, generates manifests from templates, dispatches Assay sessions, creates PRs from results, and updates tracker state — all without human intervention between "file an issue" and "review the PR."

## Why This Milestone

All infrastructure building blocks are proven: container runtimes (Docker/Compose/K8s), parallel dispatch, persistent queue, SSH workers, forge integration, health checks, auth. The missing piece is the *trigger* — today, jobs enter the queue via directory watch or HTTP POST, requiring human or script action. R026 (deferred since M006) is now unblocked. Assay's sister codebase has also matured: `StateBackend` trait with Linear/GitHub/SSH backends, `StateBackendConfig` in `RunManifest`, and the smelt-agent plugin provide the other half of the loop.

## User-Visible Outcome

### When this milestone is complete, the user can:

- File a GitHub Issue with label `smelt:ready` and see it automatically dispatched, executed, and a PR created — with the issue's labels reflecting each lifecycle phase
- File a Linear issue and see the same autonomous loop via Linear's API
- Configure a template manifest once in `server.toml` and have every tracked issue inherit the infrastructure config
- See Assay orchestrator state flow to the same tracker via state_backend passthrough

### Entry point / environment

- Entry point: `smelt serve --config server.toml` with `[tracker]` section
- Environment: local dev or server, with GitHub/Linear API access
- Live dependencies involved: GitHub API (via `gh` CLI), Linear API (via HTTP GraphQL), Docker/Compose/K8s for dispatch

## Completion Class

- Contract complete means: all tracker polling, manifest generation, lifecycle transitions, and backend passthrough are exercised by unit/integration tests with mock trackers
- Integration complete means: GitHub Issues end-to-end flow works against a real repo with `gh` CLI; Linear flow works against a real Linear project
- Operational complete means: `smelt serve` with `[tracker]` runs continuously, picks up new issues, and creates PRs without manual intervention

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- A GitHub Issue with `smelt:ready` label is automatically picked up by `smelt serve`, dispatched, and results in a PR creation with issue labels transitioning through the full lifecycle
- A Linear issue in the configured state is automatically picked up, dispatched, and results in a PR creation with issue state transitions
- Orchestrator state from inside the container is visible in the tracker via state_backend passthrough
- All 298+ existing workspace tests still pass (zero regressions)

## Risks and Unknowns

- **`gh` CLI availability and auth** — GitHub tracker relies on `gh` for issue operations; must handle missing binary or expired auth gracefully
- **Linear API rate limits** — Polling every 30s with GraphQL queries may hit limits in high-volume scenarios
- **Manifest template validation** — Template must be a valid partial manifest; invalid templates should fail fast at startup, not at dispatch time
- **Label race conditions** — Multiple `smelt serve` instances polling the same tracker could pick up the same issue; label transition must be atomic enough to prevent double-dispatch

## Existing Codebase / Prior Art

- `crates/smelt-cli/src/serve/` — existing dispatch_loop, ServerState, JobQueue, HTTP API, directory watcher, SSH dispatch, TUI
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` with `deny_unknown_fields`; new `[tracker]` section extends this
- `crates/smelt-cli/src/serve/types.rs` — `JobSource` enum (DirectoryWatch, HttpApi); needs `Tracker` variant
- `crates/smelt-core/src/assay.rs` — `AssayInvoker` builds RunManifest TOML from JobManifest; state_backend passthrough extends this
- `crates/smelt-core/src/manifest/` — `JobManifest` struct; needs optional `state_backend` field
- `../assay/crates/assay-types/src/state_backend.rs` — `StateBackendConfig` enum (local_fs, linear, github, ssh, custom)
- `../assay/crates/assay-backends/src/` — Linear, GitHub, SSH backend implementations (pattern reference)
- `crates/smelt-core/src/forge.rs` — `ForgeClient` trait, `GitHubForge`; PR creation at Phase 9

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — it is an append-only register; read it during planning, append to it during execution.

## Relevant Requirements

- R026 — Tracker-driven dispatch (promoted from deferred, parent requirement)
- R061 — Flaky test fix (M011 leftover, S01 cleanup)
- R062 — Full tracing migration (M011 leftover, S01 cleanup)
- R070 — GitHub Issues autonomous dispatch
- R071 — Linear autonomous dispatch
- R072 — TrackerSource trait abstraction
- R073 — Template manifest with issue injection
- R074 — Label-based lifecycle state machine
- R075 — State backend passthrough

## Scope

### In Scope

- M011/S02 leftover cleanup (R061, R062) — already researched, plan exists
- TrackerSource trait with capability-based degradation
- GitHub Issues poller via `gh` CLI
- Linear Issues poller via GraphQL API
- Template manifest loading and issue injection
- Label-based lifecycle state transitions (smelt:ready → smelt:queued → smelt:running → smelt:pr-created → smelt:done/smelt:failed)
- State backend passthrough in JobManifest → Assay RunManifest
- Integration with existing dispatch_loop, ServerState, TUI
- server.toml `[tracker]` config section

### Out of Scope / Non-Goals

- Webhook-driven triggers (polling only for M012)
- Jira or other tracker backends (trait is extensible, but only GitHub + Linear impls)
- Auto-merge of PRs (R031 out-of-scope, user merges)
- Budget/cost tracking (R022 remains deferred)
- Multi-session orchestration from a single issue (one issue = one Assay session)
- Bidirectional state sync (Smelt pushes state to tracker, doesn't read Assay's tracker backend output)

## Technical Constraints

- `deny_unknown_fields` on `ServerConfig` — new `[tracker]` section must be added to the struct
- `deny_unknown_fields` on `JobManifest` — new `state_backend` field must be added
- D002 (no Assay crate dependency) — state_backend config is a Smelt-side serde struct mirroring Assay's format, not an import
- D017 (strict manifest parsing) — template manifest validation at startup
- D098 (in-process dispatch) — tracker jobs go through the same `dispatch_loop` as HTTP/watcher jobs

## Integration Points

- **GitHub API** — via `gh issue list/edit/create` CLI commands for issue polling, label management, and PR creation
- **Linear API** — via `reqwest::blocking::Client` GraphQL calls (pattern from Assay's LinearBackend)
- **Existing dispatch_loop** — TrackerPoller enqueues jobs into `ServerState`, dispatch_loop picks them up unchanged
- **AssayInvoker** — Extended to serialize `state_backend` into the RunManifest TOML
- **TUI** — `JobSource::Tracker` reflected in job source column

## Open Questions

- **Poll interval configurability** — default 30s, configurable via `[tracker] poll_interval_secs`; current thinking: yes
- **Multiple tracker sources per server** — current thinking: one tracker source per `smelt serve` instance; multiple sources deferred
- **Label creation** — should Smelt auto-create lifecycle labels on first run? Current thinking: yes for GitHub (via `gh label create`), document for Linear
