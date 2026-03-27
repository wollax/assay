# M011: Concrete Remote Backends — Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

## Project Description

Assay is a spec-driven development platform for AI-augmented workflows. M010 delivered the `StateBackend` trait abstraction: all orchestrator state writes (session transitions, mesh routing, gossip manifests, checkpoint summaries) now flow through `Arc<dyn StateBackend>`. `LocalFsBackend` is the sole concrete implementation. M011 makes the abstraction useful by adding three remote backends that let smelt workers push state to a shared external store — without SCP back to the controller.

## Why This Milestone

The `StateBackend` trait exists precisely for this: decoupling state transport from execution. Smelt workers running on remote machines need somewhere to push Tier-2 events (session transitions, phase changes) without SCP file transfers back to the controller. M011 delivers the first three production backends that prove the abstraction is real and useful.

- **LinearBackend**: most immediately useful for teams already using Linear for project tracking. Run state appears as issues + comments in the same place where work is planned.
- **GitHubBackend**: useful for open-source projects or CI-first teams. Run state visible in the repo's issue tracker. Reuses `gh` CLI auth already required for PR creation.
- **SshSyncBackend**: the "smelt native" transport. Workers push state files via `scp` to the controller — same mechanism smelt already uses, now encapsulated in the trait.

## User-Visible Outcome

### When this milestone is complete, the user can:
- Set `state_backend = "linear"` in their `RunManifest`, run an orchestrated job from a smelt worker on another machine, and watch session transitions appear as comments on a Linear issue — without SCP.
- Set `state_backend = "github"` and watch run state appear as a GitHub issue with per-session-event comments in their repo.
- Set `state_backend = "ssh"` with a host/path, run a smelt job, and have workers push `state.json` and inbox/outbox files via `scp` back to the controller's `.assay/` directory.
- Run `just ready` on any configuration and see 1488+ tests pass with zero regression.

### Entry point / environment
- Entry point: `RunManifest.state_backend` field (`"linear"` | `"github"` | `"ssh"` | `"local_fs"`)
- Environment: Rust codebase, test suite, CLI; real APIs tested via UAT only
- Live dependencies: `LINEAR_API_KEY` env var for LinearBackend; `gh` CLI for GitHubBackend; SSH key/agent for SshSyncBackend

## Completion Class

- Contract complete means: each backend compiles and passes unit/contract tests with mocked API responses. `StateBackendConfig` enum variants added and schema-snapshot-locked. `assay-backends` crate builds with all feature flags.
- Integration complete means: `LocalFsBackend` remains the default; all 1488+ existing tests continue to pass. `OrchestratorConfig` can be constructed with any of the four backends and exercised by the mock-runner test harness.
- Operational complete means: N/A for LinearBackend and GitHubBackend (no daemon lifecycle). SshSyncBackend's scp invocation is exercised against a real local SSH target in UAT.

## Final Integrated Acceptance

To call this milestone complete:
- `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` named variants (no longer `Custom`-only), schema snapshots updated
- `assay-backends` crate exists with `linear`, `github`, `ssh` feature flags; each backend implements all 7 `StateBackend` methods
- `reqwest` is in `assay-backends` Cargo.toml behind `linear`/`github` features; no HTTP dep leaks into `assay-core`
- Running `just ready` is green with 1488+ tests — zero regressions
- At least one mock-API contract test per backend proves the trait interface compiles and each method returns the expected `Ok(_)`
- UAT: manually configure LinearBackend with a real `LINEAR_API_KEY`, dispatch a 2-session orchestrated run, confirm a Linear issue appears with session-event comments

## Risks and Unknowns

- **reqwest async in sync trait** — `StateBackend` methods are sync (D150). `reqwest` is async. The established pattern (D143) is to use a scoped `tokio::runtime::Builder::new_current_thread` runtime inside each method. However, if `assay-core` tests share a global tokio runtime, nested runtime creation panics. Must verify the test harness doesn't create runtime conflicts.
- **Linear GraphQL API surface** — The built-in Kata CLI Linear integration uses a private GraphQL client. `LinearBackend` needs its own client (cannot import from Kata). Must use `reqwest` + hand-rolled GraphQL queries against `api.linear.app/graphql`. Query shape must be validated against Linear's API.
- **gh CLI output parsing** — `GitHubBackend` shells out to `gh issue create --json number,url` (established pattern from D077). `send_message`/`poll_inbox` have no gh analog — these must use the GitHub REST API directly or degrade gracefully when `supports_messaging = false`.
- **scp path validation** — `SshSyncBackend` must validate the remote host string to prevent shell injection in `scp` invocations. Use `std::process::Command` with explicit arg splitting (same pattern as D008 git CLI calls).
- **Schema snapshot update** — Adding `Linear`, `GitHub`, `Ssh` variants to `StateBackendConfig` changes the JSON Schema. Both orchestrate/non-orchestrate snapshot tests from D159 must be updated.

## Existing Codebase / Prior Art

- `crates/assay-core/src/state_backend.rs` — `StateBackend` trait (7 methods), `CapabilitySet`, `LocalFsBackend`, `NoopBackend`. The template for any new backend.
- `crates/assay-types/src/state_backend.rs` — `StateBackendConfig` enum. Add `Linear`, `GitHub`, `Ssh` variants here.
- `crates/assay-types/tests/snapshots/schema_snapshots__state-backend-config-schema.snap` — Must be regenerated after adding variants.
- `crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-orchestrate-schema.snap` — Also changes.
- `crates/assay-core/src/orchestrate/executor.rs` — Where `OrchestratorConfig.backend` is constructed; CLI/MCP sites override with explicit `LocalFsBackend`. S01 adds factory fn for backend resolution from config.
- `crates/assay-cli/src/commands/run.rs` — Constructs `OrchestratorConfig` with explicit backend; will use factory fn in M011.
- `crates/assay-mcp/src/server.rs` — Same; will use factory fn.
- `plugins/claude-code/`, `plugins/codex/` — Model for new crate structure conventions.
- D008 (git CLI-first), D065 (gh CLI-first), D077 (gh --json for stable output), D143 (scoped tokio runtime for async-in-sync), D150 (sync trait, async internalized), D153 (StateBackendConfig enum), D159 (split schema snapshot tests).

> See `.kata/DECISIONS.md` for all architectural decisions. D150, D153, D159 are especially relevant.

## Relevant Requirements

- R076 — LinearBackend: `push_session_event` / `read_run_state` via Linear GraphQL
- R077 — GitHubBackend: `push_session_event` / `read_run_state` via `gh` CLI + optional REST API
- R078 — SshSyncBackend: all 7 methods via `scp` file sync
- R079 — `assay-backends` crate with feature-gated backends and backend factory function

## Scope

### In Scope

- `assay-backends` crate with `linear`, `github`, `ssh` feature flags
- `LinearBackend`: `push_session_event` (create issue / comment), `read_run_state` (fetch latest comment), `capabilities()` (messaging: false, gossip_manifest: false, annotations: true, checkpoints: false)
- `GitHubBackend`: `push_session_event` (gh issue create/comment), `read_run_state` (gh issue view), `capabilities()` (messaging: false, gossip_manifest: false, annotations: false, checkpoints: false)
- `SshSyncBackend`: all 7 methods via scp, `capabilities()` matching LocalFsBackend (all true)
- `StateBackendConfig` enum gains `Linear { team_id: String, project_id: Option<String> }`, `GitHub { repo: String, label: Option<String> }`, `Ssh { host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16> }` variants
- Schema snapshots regenerated
- Backend factory function `backend_from_config(config: &StateBackendConfig, assay_dir: PathBuf) -> Arc<dyn StateBackend>`
- CLI/MCP construction sites use factory fn
- All 1488+ existing tests continue to pass

### Out of Scope / Non-Goals

- LinearBackend `send_message`/`poll_inbox` (messaging) — Linear has no inbox concept; capability stays false
- GitHubBackend `send_message`/`poll_inbox` — same; gh issue comments are not per-session inboxes
- LinearBackend `save_checkpoint_summary` — TeamCheckpoint format doesn't map cleanly to Linear; capability false
- Multi-machine smelt integration testing (automated) — UAT only
- Token refresh / OAuth for LinearBackend — personal API key only (LINEAR_API_KEY)
- GitHub App authentication — personal token via gh CLI only

## Technical Constraints

- D001: Zero-trait convention holds — no new traits introduced. `StateBackend` is the sole exception.
- D007: Sync core. Each backend method that needs async HTTP wraps a `tokio::runtime::Builder::new_current_thread().build()` runtime scoped to the method. Do NOT use `tokio::runtime::Handle::current()` — tests may not run under a tokio runtime.
- D008/D065: Shell out to git/gh CLI for git and GitHub operations. `gh issue create`, `gh issue comment`, `gh issue view` are the GitHub transport.
- D150: Trait methods sync. Async backends internalize their runtime (same pattern as OTLP in D143).
- D153: `StateBackendConfig` enum variants must be named and schema-snapshot-locked.
- D159: Schema snapshot split — update both orchestrate and non-orchestrate snapshots after adding variants.
- No secrets in TOML/JSON on disk — Linear API key via `LINEAR_API_KEY` env var only.

## Open Questions

- `GitHubBackend.send_message` / `poll_inbox`: degrade (capabilities false) or implement via issue comments with a structured tag? — Decision: degrade gracefully with `capabilities().supports_messaging = false`. Implementing inbox semantics via issue comments would require parsing comment threads — too fragile.
- Runtime conflict in tests for LinearBackend: does the tokio test runtime conflict with `new_current_thread` inside sync methods? — Decision: mock the HTTP layer in unit tests (don't make real API calls). Contract tests use a mock server or stub that doesn't need a runtime at all.
