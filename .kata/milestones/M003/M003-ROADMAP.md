# M003: Forge-Integrated Infrastructure Platform

**Vision:** Smelt delivers the complete infrastructure pipeline — provision container, run Assay, collect result branch, create GitHub PR, and track PR status — while publishing `smelt-core` as a stable Rust library that tools like Assay can embed for programmatic environment provisioning.

## Success Criteria

- `smelt run manifest.toml` with a `[forge]` section provisions a container, runs Assay, collects the result branch, and creates a GitHub PR — printing the PR URL on completion
- `smelt status` renders a PR section showing state (open/merged/closed), CI status, and review count when a PR exists
- `smelt watch <job-name>` blocks until the PR is merged (exits 0) or closed (exits 1)
- `smelt init` generates a skeleton `job-manifest.toml` that passes `--dry-run` validation
- `smelt-core` with `forge` feature can be added as a path dependency and used to call `GitHubForge::create_pr()` and `DockerProvider::provision()` programmatically
- Concurrent `smelt run` invocations with different job names do not clobber each other's state files

## Key Risks / Unknowns

- **GitHub API auth / rate limits** — octocrab is new to the codebase; fine-grained PAT scopes, rate limit handling, and conditional requests must be proven before wiring into `execute_run()`.
- **Library API surface stability** — Publishing a public API before the forge types are stable would lock bad choices. Must be last.
- **Multi-job state migration** — Old `.smelt/run-state.toml` must not break existing `smelt status` invocations during migration.

## Proof Strategy

- **GitHub API** → retire in S01 by unit-testing PR creation and status polling against mock HTTP responses, and manually confirming with a real `GITHUB_TOKEN` + test repo.
- **Library API stability** → retire in S05 by compiling a minimal external crate that imports `smelt_core` via path dependency and calls forge + docker APIs.
- **State migration** → retire in S04 by adding backward-compat fallback: read `.smelt/run-state.toml` if per-job directory doesn't exist.

## Verification Classes

- Contract verification: unit tests in `forge.rs` asserting PR creation payload shape, status polling deserialization, `deny_unknown_fields` on `ForgeConfig`, manifest parse roundtrips for `[forge]` section.
- Integration verification: `smelt run manifest.toml` with real `GITHUB_TOKEN` + test repo creates a real PR; `smelt status` renders the PR section; `smelt watch` blocks in a real CI shell.
- Operational verification: `smelt init` generates a valid manifest; `smelt list` shows past runs; `.assay/` appears in `.gitignore` after `smelt run`.
- UAT / human verification: full `smelt run` → PR created → `smelt watch` → merge PR → watch exits 0. Requires real Docker, real Assay binary, real GitHub repo, real `GITHUB_TOKEN`.

## Milestone Definition of Done

This milestone is complete only when all are true:

- `ForgeClient` trait, `GitHubForge` impl, and `PrHandle`/`PrStatus` types exist in `smelt-core::forge` behind `forge` feature flag, with unit tests for PR creation and status polling
- `JobManifest` accepts optional `[forge]` section; manifest roundtrip tests cover forge config present and absent
- `execute_run()` has Phase 9: creates PR if forge config present; `RunState` carries `pr_url` and `pr_number`
- `smelt status` renders PR section when `pr_url` is set; `smelt watch` polls and exits correctly
- `smelt init` creates a skeleton manifest; `smelt list` lists past runs
- Multi-job state isolation: `.smelt/runs/<job-name>/state.toml` with backward-compat fallback
- `.assay/` added to `.gitignore` in host repo on `smelt run`
- `smelt-core` compiles as a library with `forge` feature, `#![deny(missing_docs)]`, and clean public API
- Manual UAT: `smelt run` → PR created → `smelt watch` → merge → watch exits 0

## Requirement Coverage

- Covers: R001, R002, R003, R004, R005, R006, R007, R008
- Partially covers: none
- Leaves for later: R020 (Docker Compose), R021 (multi-machine), R022 (budget tracking)
- Orphan risks: none

## Slices

- [x] **S01: GitHub Forge Client** `risk:high` `depends:[]`
  > After this: `cargo test -p smelt-core --features forge` proves `GitHubForge::create_pr()` and `poll_pr_status()` against mock HTTP responses; the forge module compiles cleanly behind a feature flag with no impact on feature-less builds.

- [x] **S02: Manifest Forge Config + PR Creation** `risk:medium` `depends:[S01]`
  > After this: `smelt run manifest.toml` with a `[forge]` block creates a real GitHub PR and prints the URL; `smelt run --no-pr` skips creation; `smelt run` without `[forge]` is unchanged.

- [x] **S03: PR Status Tracking** `risk:medium` `depends:[S02]`
  > After this: `smelt status` shows the PR's state and CI status; `smelt watch <job-name>` blocks until the PR is merged (exit 0) or closed (exit 1); polling uses conditional requests to stay inside GitHub rate limits.

- [x] **S04: Infrastructure Hardening** `risk:low` `depends:[]`
  > After this: `smelt init` generates a valid skeleton manifest; concurrent `smelt run` jobs use isolated state directories; `.assay/` is added to `.gitignore` after `smelt run`; `smelt list` shows past runs.

- [x] **S05: smelt-core Library API** `risk:medium` `depends:[S01,S02,S03]`
  > After this: a minimal external Rust crate can add `smelt-core` as a path dependency and call `DockerProvider`, `GitHubForge`, and `JobManifest` without touching the CLI; `smelt-core` has `#![deny(missing_docs)]` and passes `cargo doc --no-deps`.

- [x] **S06: Integration Proof** `risk:low` `depends:[S01,S02,S03,S04,S05]`
  > After this: full end-to-end — `smelt run` → result branch → PR created → `smelt status` shows PR info → `smelt watch` resolves on merge — demonstrated with a real GitHub repo; example manifests updated; critical open issues from `.planning/issues/open/` triaged.

## Boundary Map

### S01 → S02, S03, S05

Produces:
- `smelt_core::forge::ForgeClient` trait: `create_pr(repo, head, base, title, body) -> Result<PrHandle>`, `poll_pr_status(repo, number) -> Result<PrStatus>`
- `smelt_core::forge::GitHubForge` struct: `new(token: String) -> Self`, implements `ForgeClient`
- `smelt_core::forge::PrHandle { url: String, number: u64 }` — result of PR creation
- `smelt_core::forge::PrStatus { state: PrState, ci_status: CiStatus, review_count: u32 }` — result of status poll
- `smelt_core::forge::PrState` enum: `Open`, `Merged`, `Closed`
- `smelt_core::forge::CiStatus` enum: `Pending`, `Passing`, `Failing`, `Unknown`
- `smelt_core::forge::ForgeConfig { provider: String, repo: String, token_env: String }` — parsed from `[forge]`
- `smelt-core/Cargo.toml`: `forge` feature gating `octocrab` dependency
- Unit tests: mock HTTP server for create_pr happy path, 401 auth error, 422 validation error, poll_pr_status open/merged/closed transitions

Consumes:
- nothing (first slice)

### S02 → S03, S05

Produces:
- `JobManifest.forge: Option<ForgeConfig>` — optional `[forge]` section, `deny_unknown_fields`
- Manifest validation: if `forge.is_some()`, validate `repo` is `owner/repo` format; validate `token_env` is non-empty
- `RunState.pr_url: Option<String>`, `RunState.pr_number: Option<u64>` — written at Phase 9
- Phase 9 in `execute_run()`: after `ResultCollector::collect()`, if `manifest.forge.is_some()`, read `GITHUB_TOKEN` from `token_env`, call `GitHubForge::create_pr()`, write PR info to `RunState`, print `PR created: <url>`
- `smelt run --no-pr` flag: skips Phase 9 even if forge config present
- Roundtrip tests for `JobManifest` with `[forge]` present and absent

Consumes from S01:
- `ForgeClient::create_pr()`, `GitHubForge::new()`, `PrHandle`, `ForgeConfig`

### S03 → S05

Produces:
- `RunState.pr_status: Option<PrState>`, `RunState.ci_status: Option<CiStatus>`, `RunState.review_count: Option<u32>` — updated by `smelt watch` or on-demand
- `smelt status` PR section: rendered only when `pr_url` is set; shows URL, state, CI status, review count
- `smelt watch <job-name>` command: polls `ForgeClient::poll_pr_status()` every 30s; uses ETag/conditional requests; exits 0 on `Merged`, exits 1 on `Closed`, continues on `Open`
- `ForgeClient::poll_pr_status()` unit-tested with state-transition sequences
- `smelt-cli/src/commands/watch.rs`

Consumes from S02:
- `RunState.pr_url`, `RunState.pr_number`
- `ForgeClient::poll_pr_status()`, `PrState`, `CiStatus` (from S01)

### S04 (independent — no deps, no output consumed by other slices)

Produces:
- `smelt init` command: writes `./job-manifest.toml` skeleton (fails if already exists); generated manifest passes `smelt run --dry-run`
- Per-job state directory: `.smelt/runs/<job-name>/state.toml` replaces flat `.smelt/run-state.toml`; backward-compat: `smelt status` falls back to flat file if per-job dir absent
- `smelt list` command: reads `.smelt/runs/` and prints job name, phase, created_at, and PR URL (if any) for each
- `.assay/` gitignore guard: during `smelt run`, before Phase 5.5, if `./‌.gitignore` exists and doesn't contain `.assay/`, append it; if no `.gitignore`, create it with `.assay/`
- Unit tests for `smelt init` (idempotency guard, generated manifest validation) and state path resolution

Consumes:
- nothing (independent)

### S05 → S06

Produces:
- `smelt_core` as a publishable library: `#![deny(missing_docs)]` on all pub types; crate-level doc comment with usage example; `Cargo.toml` metadata (description, keywords, categories, homepage)
- `forge` feature properly gates octocrab so `smelt-core` without feature has zero new dependencies
- Clean `pub use` re-exports: `smelt_core::{docker::DockerProvider, forge::{ForgeClient, GitHubForge, PrHandle, PrStatus, ForgeConfig}, manifest::JobManifest, monitor::{JobMonitor, RunState, JobPhase}, collector::{ResultCollector, BranchCollectResult}, provider::{RuntimeProvider, ContainerId, ExecHandle}}`
- Minimal external `smelt-example` test crate in `/tmp` that imports via path dependency and calls `GitHubForge::new("token").create_pr(...)` in a test — proves the API works without the CLI

Consumes from S01, S02, S03:
- All forge types (must be stable before publishing)
- All existing public types from M001/M002
