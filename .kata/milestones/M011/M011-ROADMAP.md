# M011: Concrete Remote Backends

**Vision:** Introduce three production `StateBackend` implementations — `LinearBackend`, `GitHubBackend`, and `SshSyncBackend` — in a new `assay-backends` crate, gated by feature flags. Extend `StateBackendConfig` with named `Linear`, `GitHub`, and `Ssh` variants. Add a backend factory function consumed by CLI/MCP construction sites. Smelt workers can now push Tier-2 orchestrator events to an external store without SCP; the abstraction built in M010 delivers its first real value.

## Success Criteria

- `just ready` green with all 1488+ tests passing after every slice
- `StateBackendConfig` has `Linear`, `GitHub`, and `Ssh` named variants; schema snapshots updated and committed
- `assay-backends` crate exists with `linear`, `github`, `ssh` feature flags; each backend compiles and passes contract tests
- `LinearBackend::push_session_event` creates a Linear issue (first call) or appends a comment (subsequent calls); `read_run_state` reads the latest comment back
- `GitHubBackend::push_session_event` creates a GitHub issue (first call) or appends a comment (subsequent calls) via `gh` CLI; `read_run_state` reads back via `gh issue view`
- `SshSyncBackend` implements all 7 trait methods by shelling out to `scp`; `CapabilitySet::all()` returned
- `backend_from_config()` factory fn resolves any `StateBackendConfig` variant to an `Arc<dyn StateBackend>`; CLI/MCP construction sites use it
- No existing behavior changes for `local_fs` users — `LocalFsBackend` remains default

## Key Risks / Unknowns

- **Async-in-sync runtime conflict** — `LinearBackend` needs HTTP (reqwest/async) inside sync trait methods. The `new_current_thread` pattern (D143) works in production but may panic if tests already run inside a tokio runtime. Must use mock transport in unit tests to avoid this.
- **Linear GraphQL API shape** — Must craft correct GraphQL mutations (createIssue, createComment) and queries (issue comments) against `api.linear.app/graphql`. Wrong field names fail silently or with opaque GraphQL errors.
- **StateBackendConfig variant addition** — Adding three new variants changes the JSON Schema. Both orchestrate/non-orchestrate snapshot tests (D159) must be regenerated. If the schema update is wrong, all schema tests fail.
- **scp injection** — `SshSyncBackend` constructs scp arguments from user-provided host/path strings. Must split args explicitly (no shell interpolation) to prevent injection.
- **GitHubBackend `send_message`/`poll_inbox`** — No native inbox concept in GitHub Issues. These methods degrade (`supports_messaging = false`). The degradation path (from S03) handles this but the capability declaration must be correct.

## Proof Strategy

- Async-in-sync runtime conflict → retire in S02 (LinearBackend) by unit-testing with a mock HTTP server (no real runtime contention) and verifying `push_session_event` + `read_run_state` compile and pass
- Linear GraphQL API shape → retire in S02 by validating query shapes against Linear's public schema in contract tests; real API validation is UAT
- StateBackendConfig variant addition → retire in S01 by regenerating all schema snapshots and confirming `just ready` green
- scp injection → retire in S04 by using `Command::arg()` chaining (never shell string) and a test that exercises the arg construction with a path containing spaces

## Verification Classes

- Contract verification: unit tests on each backend with mock transport; `StateBackendConfig` serde round-trip tests for each new variant; schema snapshot regeneration; factory fn dispatch tests
- Integration verification: existing 1488+ tests pass unchanged; `OrchestratorConfig` constructed with each backend type and exercised by `test_mesh_degrades` / `test_gossip_degrades` degradation test patterns
- Operational verification: SshSyncBackend scp arg construction verified; no real SSH connection required in tests
- UAT / human verification: LinearBackend with real `LINEAR_API_KEY` against api.linear.app; GitHubBackend with real `gh` CLI against a test repo; SshSyncBackend against localhost SSH

## Milestone Definition of Done

This milestone is complete only when all are true:

- `assay-backends` crate exists, builds clean, and is listed in workspace `members`
- `StateBackendConfig` has `Linear { team_id, project_id }`, `GitHub { repo, label }`, `Ssh { host, remote_assay_dir, user, port }` variants; schema snapshots committed and green
- `LinearBackend` (feature: `linear`), `GitHubBackend` (feature: `github`), `SshSyncBackend` (feature: `ssh`) all implement `StateBackend` with contract tests passing
- `backend_from_config()` in `assay-backends` resolves all four variants to `Arc<dyn StateBackend>`
- `assay-cli` and `assay-mcp` use `backend_from_config()` at construction sites; no hardcoded `LocalFsBackend::new(...)` at call sites that receive a `RunManifest`
- `just ready` green with 1488+ tests — zero regression
- D160–D165 decisions documented in DECISIONS.md

## Requirement Coverage

- Covers: R076, R077, R078, R079
- Partially covers: none
- Leaves for later: LinearBackend messaging (inbox/outbox semantics), GitHubBackend messaging, checkpoint persistence on remote backends (M012+)
- Orphan risks: multi-machine smelt integration testing end-to-end — UAT only in this milestone

## Slices

- [x] **S01: assay-backends crate scaffold and StateBackendConfig variants** `risk:high` `depends:[]`
  > After this: `assay-backends` crate exists in workspace; `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` named variants; schema snapshots updated; `backend_from_config()` factory fn compiles and dispatches all four variants to stub backends; `just ready` green.

- [x] **S02: LinearBackend** `risk:high` `depends:[S01]`
  > After this: `LinearBackend` implements all 7 `StateBackend` methods; `push_session_event` creates/comments on a Linear issue; `read_run_state` fetches latest comment back; contract tests with mock HTTP pass; `capabilities()` returns messaging:false, checkpoints:false, annotations:true, gossip_manifest:false; `just ready` green.

- [x] **S03: GitHubBackend** `risk:medium` `depends:[S01]`
  > After this: `GitHubBackend` implements all 7 `StateBackend` methods; `push_session_event` shells out to `gh issue create`/`comment`; `read_run_state` shells out to `gh issue view`; contract tests with mock `gh` binary pass; `capabilities()` returns all-false except annotations where it returns true (or false — see plan); `just ready` green.

- [ ] **S04: SshSyncBackend and CLI/MCP factory wiring** `risk:medium` `depends:[S01]`
  > After this: `SshSyncBackend` implements all 7 methods via `scp`; arg construction verified against injection; `CapabilitySet::all()` returned; `backend_from_config()` wired into all `assay-cli` and `assay-mcp` construction sites that receive a `RunManifest`; `just ready` green with 1488+ tests passing.

## Boundary Map

### S01 → S02, S03, S04

Produces:
- `assay_backends` crate at `crates/assay-backends/` with `Cargo.toml` (`linear`, `github`, `ssh` feature flags) and `src/lib.rs`
- `StateBackendConfig::Linear { team_id: String, project_id: Option<String> }` variant
- `StateBackendConfig::GitHub { repo: String, label: Option<String> }` variant
- `StateBackendConfig::Ssh { host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16> }` variant
- Updated JSON Schema snapshots: `schema_snapshots__state-backend-config-schema.snap` and `schema_snapshots__run-manifest-orchestrate-schema.snap`
- `backend_from_config(config: &StateBackendConfig, assay_dir: PathBuf) -> Arc<dyn StateBackend>` in `assay_backends::factory` — dispatches `LocalFs` → `LocalFsBackend`, others → stub `NoopBackend` pending S02–S04
- Serde round-trip tests for all four `StateBackendConfig` variants

Consumes:
- nothing (first slice) — depends on M010 work already on main

### S02 → S04

Produces:
- `assay_backends::linear::LinearBackend` struct implementing `StateBackend` behind `cfg(feature = "linear")`
- `LinearBackend::new(api_key: String, team_id: String, project_id: Option<String>) -> Self`
- `LinearClient` internal module: `create_issue(title, description) -> String` (returns issue id), `create_comment(issue_id, body)`, `get_latest_comment(issue_id) -> Option<String>`
- Serde types for Linear GraphQL request/response
- `reqwest` as a dep of `assay-backends` behind `linear` feature
- Mock HTTP contract tests for `push_session_event` (first call: creates issue; subsequent calls: creates comment) and `read_run_state`
- `backend_from_config()` updated: `Linear` variant → `Arc::new(LinearBackend::new(...))`

Consumes from S01:
- `assay_backends::factory::backend_from_config` stub (to replace)
- `StateBackendConfig::Linear { team_id, project_id }` variant

### S03 → S04

Produces:
- `assay_backends::github::GitHubBackend` struct implementing `StateBackend` behind `cfg(feature = "github")`
- `GitHubBackend::new(repo: String, label: Option<String>) -> Self`
- `GhRunner` internal module: `create_issue(title, body) -> u64` (issue number), `create_comment(issue_number, body)`, `get_issue_body(issue_number) -> Option<String>`
- Contract tests with a mock `gh` binary (PATH override) proving arg shapes for create/comment/view
- `backend_from_config()` updated: `GitHub` variant → `Arc::new(GitHubBackend::new(...))`

Consumes from S01:
- `assay_backends::factory::backend_from_config` stub (to replace)
- `StateBackendConfig::GitHub { repo, label }` variant

### S04 → (M012)

Produces:
- `assay_backends::ssh::SshSyncBackend` struct implementing `StateBackend` behind `cfg(feature = "ssh")`
- `SshSyncBackend::new(host: String, remote_assay_dir: String, user: Option<String>, port: Option<u16>) -> Self`
- `scp_push(local: &Path, remote: &str)` and `scp_pull(remote: &str, local: &Path)` helpers using `Command::arg()` chaining
- Contract test proving scp arg construction for a path with spaces does not produce a shell-injection risk
- `backend_from_config()` fully resolved: all four variants dispatch to real backends
- `assay-cli` `run.rs` and `assay-mcp` `server.rs` construction sites use `backend_from_config(manifest.state_backend.as_ref().unwrap_or(&StateBackendConfig::LocalFs), assay_dir.clone())` — no hardcoded `LocalFsBackend::new` at manifest-dispatch call sites
