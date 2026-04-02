# M003: Forge-Integrated Infrastructure Platform — Context

**Gathered:** 2026-03-21
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. M001 delivered the Docker container lifecycle; M002 delivered real Assay integration with contract-correct manifest generation and streaming output. M003 completes the delivery pipeline: result branch → GitHub PR → status tracking, while also publishing `smelt-core` as a stable Rust library for programmatic embedding.

## Why This Milestone

M002 ends with a result branch sitting on the host repo. That's half the delivery loop — the half that doesn't require human judgment. The other half (getting the work into review) currently requires the user to manually create a PR. M003 automates this, making `smelt run` a complete "submit work for review" primitive.

Simultaneously, positioning Smelt as the infrastructure layer for the smelt/assay/cupel toolkit requires a library API, not just a CLI. Assay v0.6.0 already implements its own worktree/session management, but future integration (Assay calling Smelt for containerized provisioning) is only possible if `smelt-core` is a library.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Run `smelt run manifest.toml` with a `[forge]` config block and have a GitHub PR created automatically from the result branch, with the PR URL printed on completion.
- Run `smelt status` to see the job's current phase **and** the PR state (open/merged/closed, CI status, review count).
- Run `smelt watch` (or `smelt status --follow`) in a CI pipeline and have it block until the PR is merged or closed — exiting 0 on merge, 1 on close-without-merge.
- Run `smelt init` to generate a skeleton `job-manifest.toml` in the current directory, ready to fill in.
- Depend on `smelt-core` as a Rust library and programmatically provision Docker environments and create PRs without going through the CLI.

### Entry point / environment

- Entry point: `smelt run`, `smelt status`, `smelt watch`, `smelt init` (CLI)
- Environment: local dev or CI with Docker daemon running + GitHub repo + `GITHUB_TOKEN` in environment
- Live dependencies: Docker daemon, `assay` binary (in container), GitHub API

## Completion Class

- Contract complete means: `ForgeClient::create_pr()` and `poll_pr_status()` unit-tested with mock HTTP responses; `smelt-core` compiles as a library with `forge` feature; all new manifest fields parse and validate.
- Integration complete means: `smelt run manifest.toml` with a real GitHub repo and `GITHUB_TOKEN` provisions container → runs assay → collects branch → creates PR → prints URL; `smelt status` shows PR state.
- Operational complete means: `smelt watch` blocks in a real CI pipeline until PR is merged; `smelt init` generates a manifest that passes `--dry-run` validation.

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `smelt run manifest.toml` (with `[forge]` section) provisions a container, runs `assay run`, collects the result branch, creates a GitHub PR, and prints the PR URL — demonstrated with a real GitHub repo and `GITHUB_TOKEN`.
- `smelt status` renders a PR section showing the PR's current state (obtained from the GitHub API).
- `smelt-core` with `forge` feature can be added as a path dependency to another Rust crate and used to call `DockerProvider::provision()`, `AssayInvoker` methods, and `GitHubForge::create_pr()` programmatically.

## Risks and Unknowns

- **GitHub API rate limits during watch polling** — If `smelt watch` polls too aggressively in CI, it may hit secondary rate limits. Mitigation: exponential backoff + 30s default interval + conditional requests (ETag/Last-Modified). Retire in S03.
- **PR creation on repos without `contents: write` token** — Fine-grained PATs may not have `pull_requests: write`. Error messages must be clear. Retire in S01 by surfacing octocrab errors verbatim.
- **octocrab async runtime conflicts** — smelt-core already uses tokio. octocrab is tokio-native. No conflict expected, but feature flag gating is needed to avoid pulling octocrab into pure-library consumers who don't want forge. Retire in S01.
- **Multi-job state migration** — Moving from `.smelt/run-state.toml` to `.smelt/runs/<job>/state.toml` requires backward-compat handling if old state files exist. Retire in S04 by reading old location as fallback.
- **smelt-core public API stability** — Wrong choices in the library surface are hard to reverse. S05 must be planned after S01–S03 stabilize the forge types. Retire in S05 by reviewing all public types before declaring stability.

## Existing Codebase

- `crates/smelt-core/src/lib.rs` — Already re-exports all public types. Adding `forge` module here behind feature flag.
- `crates/smelt-core/src/manifest.rs` — `JobManifest` with `job`, `environment`, `credentials`, `session`, `merge` sections. `[forge]` section added in S02.
- `crates/smelt-core/src/monitor.rs` — `RunState` with `JobPhase`. Gains `pr_url`, `pr_number`, `pr_status` fields in S02/S03.
- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` 8-phase pipeline. Phase 9 (PR creation) added in S02.
- `crates/smelt-cli/src/commands/status.rs` — Reads `RunState`, renders to terminal. Gains PR section in S03.
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait. No changes expected in M003.
- `crates/smelt-core/src/docker.rs` — `DockerProvider`. No changes expected in M003.
- `crates/smelt-core/src/collector.rs` — `ResultCollector`. No changes expected.
- `.smelt/run-state.toml` — Current single-job state file. Replaced by per-job directory in S04.

> See `.kata/DECISIONS.md` for all architectural decisions — D001–D051 cover M001/M002; D052 covers M003 GitHub API choice.

## Relevant Requirements

- R001 — PR creation (primary deliverable)
- R002 — Manifest forge config
- R003 — smelt status PR rendering
- R004 — smelt watch blocking behavior
- R005 — smelt-core library API
- R006 — Multi-job state isolation
- R007 — smelt init
- R008 — .assay/ gitignore protection

## Scope

### In Scope

- GitHub PR creation via `octocrab` (host-side; token never enters container)
- `[forge]` section in `JobManifest` with provider, repo, token_env fields
- `--no-pr` flag on `smelt run`
- Phase 9 in `execute_run()`: PR creation after result collection
- `smelt status` PR section (state, CI status, review count)
- `smelt watch` / `smelt status --follow` polling until PR merges
- `smelt-core` library API with `forge` feature flag
- `smelt init` command
- Multi-job state isolation (`.smelt/runs/<job-name>/state.toml`)
- `.assay/` gitignore protection on host repo

### Out of Scope / Non-Goals

- Smelt merges PRs (human decision only)
- Docker Compose runtime — M004
- Multi-machine coordination — future
- Azure DevOps / GitLab forge support — only GitHub in M003
- Webhook triggers (Assay's domain)
- Budget/cost tracking

## Technical Constraints

- D001 is firm: Smelt is infrastructure; PR creation and status are infrastructure operations (deliver result, track delivery).
- D002 is firm: No Smelt crate dependency on Assay. The reverse (Assay depending on smelt-core) is allowed and is the goal of R005/S05.
- D014 pattern: `GITHUB_TOKEN` is read from the host environment and used in API calls on the host. It NEVER enters the container. The `token_env` field in `[forge]` names the env var to read, not the token value.
- RPITIT (D019) is firm: no `async_trait` macro; forge trait methods use RPITIT.
- `deny_unknown_fields` (D017): `ForgeConfig` must use it, like all manifest structs.
- octocrab version: use `0.41` or latest; confirm tokio compatibility.

## Integration Points

- **GitHub API (octocrab)** — PR creation, PR status polling. Rate limits apply. Use conditional requests for polling.
- **`GITHUB_TOKEN` env var** — Host-side credential. Fine-grained PAT needs `pull_requests: write` (for public repos) or `contents: write` + `pull_requests: write`.
- **Assay binary in container** — No changes to Assay integration in M003. Phase 9 runs after Assay exits.
- **ResultCollector output** — `BranchCollectResult.branch` is the head branch for the PR; `manifest.job.base_ref` is the base.

## Open Questions

- Should `[forge]` be optional in the manifest (skipped if absent) or required (validation error if absent)? — Optional; forge config absent = no PR created. This preserves backward compat for users who just want local runs.
- Should `smelt watch` be a standalone command or `smelt status --follow`? — Standalone `smelt watch` is cleaner for CI usage (`smelt watch job-name`). Decide in S03 planning.
- Should `smelt list` list past runs from `.smelt/runs/`? — Yes, as a minor addition in S04 alongside `smelt init`.
