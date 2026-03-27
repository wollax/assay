---
id: S03
milestone: M011
status: ready
---

# S03: GitHubBackend — Context

## Goal

Implement `GitHubBackend` in `assay_backends::github` behind the `github` feature flag, with all 7 `StateBackend` methods shelling out to `gh` CLI, mock `gh` binary contract tests, and `backend_from_config()` updated to dispatch the `GitHub` variant to a real backend.

## Why this Slice

S02 (LinearBackend) established the backend implementation pattern — mock transport, issue-per-run with local ID tracking, full JSON state as comments. S03 applies the same pattern to GitHub Issues via the `gh` CLI, proving that the abstraction works across two different external stores. It unblocks S04 which needs all three remote backends implemented before wiring the factory into CLI/MCP construction sites.

## Scope

### In Scope

- `assay_backends::github::GitHubBackend` implementing all 7 `StateBackend` methods behind `cfg(feature = "github")`
- `GitHubBackend::new(repo: String, label: Option<String>) -> Self`
- `GhRunner` internal module: `create_issue(title, body) -> u64` (issue number), `create_comment(issue_number, body)`, `get_issue_body(issue_number) -> Option<String>`
- `push_session_event` behavior: first call creates a GitHub issue (title `"Assay run <run_id>"`), subsequent calls append a comment — issue number tracked in `.assay/orchestrator/<run_id>/github_issue_number`
- `read_run_state` behavior: shell out to `gh issue view --json body` on the tracked issue number, parse fenced JSON block back to `OrchestratorStatus`
- Comment/body format: full `OrchestratorStatus` JSON as a fenced code block — same format as LinearBackend; `read_run_state` deserializes it directly
- `capabilities()`: all false — `messaging=false, gossip_manifest=false, annotations=false, checkpoints=false`
- `annotate_run`: no-op returning `Ok(())` — `supports_annotations = false`; GitHub Issues have no meaningful annotation concept distinct from comments, and gossip manifest paths are local filesystem paths irrelevant to remote GitHub observers
- `send_message` / `poll_inbox`: no-op returning `Ok(())` / `Ok(vec![])` — `supports_messaging = false`
- `save_checkpoint_summary`: no-op returning `Ok(())` — `supports_checkpoints = false`
- Error handling: `gh` not installed, non-zero exit (unauthenticated, repo not found, etc.) returns `Err` — fail the orchestration run, no silent degradation
- Issue creation uses `gh issue create --repo <repo> --title "Assay run <run_id>" --body <body>` with optional `--label <label>` when configured
- `backend_from_config()` GitHub arm updated: `StateBackendConfig::GitHub { .. }` → `Arc::new(GitHubBackend::new(...))` (still logs warn when `github` feature is not enabled)
- Contract tests with a mock `gh` binary (PATH override in test) proving: arg shapes for `gh issue create`, `gh issue comment`, `gh issue view`; first-call creates issue; subsequent calls add comments
- `just ready` green with 1499+ tests

### Out of Scope

- `annotate_run` posting a tagged comment — capability is false; no-op only
- Inbox/outbox semantics via issue comments — messaging deferred; GitHub Issues have no inbox concept
- `save_checkpoint_summary` persisting to GitHub Issues — TeamCheckpoint format doesn't map; deferred
- GitHub App auth or OAuth — personal token via `gh` CLI auth only
- Direct GitHub REST/GraphQL API calls — all operations go through `gh` CLI (D162)
- CLI/MCP construction site wiring — that is S04

## Constraints

- D162: all GitHub operations via `gh` CLI (`gh issue create`, `gh issue comment`, `gh issue view --json`). No direct GitHub REST/GraphQL. Consistent with D008 (git CLI-first) and D065 (gh CLI-first).
- D077: use `--json` flag for stable machine-readable output from `gh`. `gh issue create --json number,url` for creation, `gh issue view <number> --json body` for reading.
- D007: sync core. `GhRunner` calls use `std::process::Command` — no async needed since `gh` is a subprocess, not an async HTTP client. No `new_current_thread` runtime needed.
- D150: trait methods are sync. No change needed — subprocess calls are already synchronous.
- D160: `assay-backends` is a leaf crate. No new HTTP dep for GitHubBackend — `gh` CLI handles auth and HTTP.

## Integration Points

### Consumes

- `assay_core::state_backend::{StateBackend, CapabilitySet}` — trait to implement
- `assay_types::orchestrate::OrchestratorStatus` — data shape serialized into issue body/comments
- `assay_types::StateBackendConfig::GitHub { repo, label }` — config variant (from S01)
- `assay_backends::factory::backend_from_config` stub — the `GitHub` arm to replace
- `gh` CLI (subprocess) — `gh issue create`, `gh issue comment`, `gh issue view`

### Produces

- `crates/assay-backends/src/github.rs` — `GitHubBackend` + `GhRunner` internal module
- `gh`-driven issue creation: one issue per run_id (title `"Assay run <run_id>"`), comments per `push_session_event`
- `.assay/orchestrator/<run_id>/github_issue_number` — persisted issue number for comment routing
- Mock `gh` binary contract tests proving arg shapes for create/comment/view
- Updated `backend_from_config()` GitHub arm

## Open Questions

- `read_run_state` reads the issue body (first write) or the latest comment (subsequent writes). Current thinking: use `gh issue view --json body,comments` and take the latest comment if any exist, otherwise fall back to the body. This matches the LinearBackend pattern (`get_latest_comment`) and handles the case where `push_session_event` was called only once (issue body = first state, no comments yet).
- `gh issue comment` does not return a comment ID or number in its default output. For `read_run_state`, reading back via the issue body+comments is the right approach (no comment ID needed). Current thinking: confirmed correct — no comment ID persistence required.
