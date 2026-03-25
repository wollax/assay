# Requirements

This file is the explicit capability and coverage contract for the project.

## Active

### R001 — smelt run creates GitHub PR from result branch
- Class: primary-user-loop
- Status: validated
- Description: After `smelt run` collects the result branch, Smelt automatically creates a GitHub PR from the result branch to the base ref, printing the PR URL on completion.
- Why it matters: The result branch is only useful when it triggers human review. PR creation completes the infrastructure delivery loop — from "container running" to "review requested."
- Source: user
- Primary owning slice: M003/S02
- Supporting slices: M003/S01
- Validation: validated
- Notes: Automated proof by M003/S02 (Phase 9 integration tests), M003/S03 (smelt watch unit tests), and M003/S06 `test_init_then_dry_run_smoke` (subprocess dry-run end-to-end). Live proof (real Docker + real GITHUB_TOKEN) deferred to human execution of S06-UAT.md.

### R002 — Job manifest supports forge configuration block
- Class: integration
- Status: validated
- Description: `JobManifest` accepts an optional `[forge]` section specifying the provider (github), repo (owner/repo), and token env var name. PR creation is skipped when `[forge]` is absent.
- Why it matters: Forge config is per-job — different jobs may target different repos or use different tokens. Manifest is the right place for it.
- Source: user
- Primary owning slice: M003/S02
- Supporting slices: none
- Validation: validated
- Notes: Forge credentials (GITHUB_TOKEN) stay on the host; never passed into the container. Proven by M003/S02 automated tests: roundtrip present/absent, validation (invalid repo format, empty token_env), deny_unknown_fields.

### R003 — smelt status shows PR state and CI status
- Class: failure-visibility
- Status: validated
- Description: When a job has created a PR, `smelt status` displays the PR URL, state (open/merged/closed), CI check status, and review count.
- Why it matters: After `smelt run` exits, the user needs a way to see whether the PR is blocked on CI or review without opening GitHub manually.
- Source: user
- Primary owning slice: M003/S03
- Supporting slices: M003/S02
- Validation: validated
- Notes: Proven by M003/S03: format_pr_section() unit-tested for all display cases (URL, state, CI, reviews, unknown fallbacks, backward-compat TOML); section is absent when pr_url is None.

### R004 — smelt watch blocks until PR merges or closes
- Class: operability
- Status: validated
- Description: `smelt watch` (or `smelt status --follow`) polls the PR state every 30s and exits 0 when the PR is merged or exits 1 when closed without merging.
- Why it matters: CI pipelines and developers who run smelt in automation need a blocking command rather than a one-shot status check.
- Source: user
- Primary owning slice: M003/S03
- Supporting slices: none
- Validation: validated
- Notes: Proven by M003/S03: run_watch<F: ForgeClient> unit-tested with MockForge for exits_0_on_merged, exits_1_on_closed, immediate_merged, updates_run_state_each_poll; guard conditions (no URL, missing token) tested. Live end-to-end proof deferred to S06. Polling interval configurable via --interval-secs. Does not merge the PR — user merges.

### R005 — smelt-core exposes a stable Rust library API
- Class: integration
- Status: validated
- Description: `smelt-core` is published with stable public API, documented types, a `forge` feature flag gating the octocrab dependency, and a usage example. External crates (e.g. Assay) can programmatically provision Docker environments and create PRs without going through the CLI.
- Why it matters: The "infrastructure layer" positioning requires a library surface — a CLI-only tool can't be embedded. Assay should be able to call Smelt's infrastructure from its own orchestration loop.
- Source: user
- Primary owning slice: M003/S05
- Supporting slices: M003/S01
- Validation: validated
- Notes: Proven by M003/S05: `/tmp/smelt-example` external crate imports smelt-core via path dependency and calls `GitHubForge::new().create_pr()` in a test; `#![deny(missing_docs)]` enforced via RUSTDOCFLAGS="-D missing_docs" in both default and forge feature variants (zero warnings confirmed by S06). crates.io publish deferred; path dependency is sufficient proof of API design.

### R006 — Concurrent smelt runs use isolated state directories
- Class: quality-attribute
- Status: validated
- Description: `.smelt/runs/<job-name>/state.toml` replaces the flat `.smelt/run-state.toml`. Multiple `smelt run` invocations with different job names don't clobber each other's state.
- Why it matters: The single-file model (D034) is a known scalability limit. Per-job directories unblock multi-job workflows.
- Source: inferred
- Primary owning slice: M003/S04
- Supporting slices: none
- Validation: validated
- Notes: Proven by M003/S04: test_state_path_resolution, test_read_legacy_reads_flat_file, test_cleanup_uses_state_toml, and test_status_legacy_backward_compat. `smelt status <job-name>` reads per-job path; `smelt status` (no args) reads legacy flat file via read_legacy().

### R007 — smelt init generates a skeleton job manifest
- Class: launchability
- Status: validated
- Description: `smelt init` creates a `./job-manifest.toml` with all required sections pre-filled with sensible defaults and inline comments, ready to edit and run.
- Why it matters: The manifest format has five required sections. New users struggle to write a valid first manifest from scratch. `smelt init` removes the blank-page problem.
- Source: inferred (brainstorm quick-win)
- Primary owning slice: M003/S04
- Supporting slices: none
- Validation: validated
- Notes: Proven by M003/S04: test_init_creates_manifest (loads and validates the generated file), test_init_fails_if_file_exists (idempotency guard exits 1), test_init_skeleton_parses (skeleton passes validate() directly). SKELETON is a raw string literal (D065) to preserve inline # comments.

### R008 — .assay/ is protected from accidental git commits
- Class: quality-attribute
- Status: validated
- Description: When `smelt run` writes `.assay/` to the bind-mounted host repo, it also ensures `.assay/` is in the repo's `.gitignore`, preventing ephemeral Assay state from being committed.
- Why it matters: Known M002 gap. Without this, every user who runs `smelt run` in their repo ends up with uncommitted `.assay/` showing in `git status`.
- Source: execution (M002 known issue)
- Primary owning slice: M003/S04
- Supporting slices: none
- Validation: validated
- Notes: Proven by M003/S04: test_ensure_gitignore_creates, test_ensure_gitignore_appends, test_ensure_gitignore_idempotent, test_ensure_gitignore_trailing_newline. ensure_gitignore_assay() placed after Phase 3 (D066); non-fatal on error.

---

## Validated

### R020 — Docker Compose runtime for multi-service environments
- Class: core-capability
- Status: validated
- Description: When `[environment] runtime = "compose"`, Smelt generates a `docker-compose.yml` from the manifest's `[[services]]` entries, injects a `smelt-agent` service (using `environment.image`, bind-mounting the repo, forwarding credentials), runs `docker compose up`, waits for all services to be healthy, runs Assay in the agent, and tears down with `docker compose down`.
- Why it matters: Projects with external service dependencies (Postgres, Redis, etc.) can't run in a single container. `runtime = "compose"` unblocks real-world projects without changing the rest of the Smelt workflow.
- Source: user (originally inferred, promoted to active for M004)
- Primary owning slice: M004/S03
- Supporting slices: M004/S01, M004/S02, M004/S04
- Validation: validated
- Notes: Proven by S03 integration tests: `test_compose_provision_exec_teardown`, `test_compose_healthcheck_wait_postgres`, `test_compose_teardown_after_exec_error` — all three pass against real Docker. S04 wired CLI dispatch (AnyProvider enum in run.rs) and --dry-run output (── Compose Services ──). 220 workspace tests, 0 failures. Assay generates the manifest including `[[services]]` entries. Full compose-service passthrough — each `[[services]]` entry is serialized to YAML as-is. Credentials injected into smelt-agent only, never into service containers.

### R010 — Docker container lifecycle: provision, exec, tear down
- Class: core-capability
- Status: validated
- Description: DockerProvider provisions a container from an image, executes commands inside it, and tears down on success, failure, timeout, and Ctrl+C.
- Why it matters: Foundation for all Smelt execution.
- Source: user
- Primary owning slice: M001/S02
- Supporting slices: M001/S05 (signal handling + timeout)
- Validation: validated
- Notes: Proven by 23 Docker integration tests in M001/M002.

### R011 — AssayInvoker generates valid RunManifest and spec files
- Class: integration
- Status: validated
- Description: AssayInvoker generates `[[sessions]]`-keyed RunManifest TOML and per-session spec files that a real `assay` binary accepts without schema errors.
- Why it matters: Assay uses `deny_unknown_fields` — any field mismatch causes silent failure.
- Source: execution
- Primary owning slice: M002/S01
- Supporting slices: M002/S02
- Validation: validated
- Notes: Proven by 13 unit tests + test_real_assay_manifest_parsing integration test.

### R012 — Gate output streams to terminal in real time
- Class: failure-visibility
- Status: validated
- Description: `smelt run` prints Assay gate output lines to stderr as they are produced, not buffered until exit.
- Why it matters: Assay sessions run for minutes. Buffered output would give no feedback during execution.
- Source: user
- Primary owning slice: M002/S03
- Supporting slices: none
- Validation: validated
- Notes: exec_streaming() with FnMut callback; test_exec_streaming_delivers_chunks_in_order.

### R013 — Exit code 2 (gate failures) distinguished from exit code 1 (errors)
- Class: failure-visibility
- Status: validated
- Description: `smelt run` exits 2 when `assay run` exits 2 (gate failures) and reports `JobPhase::GatesFailed`, distinct from `JobPhase::Failed`.
- Why it matters: Scripts and CI pipelines need to distinguish "gates didn't pass" from "smelt crashed."
- Source: user
- Primary owning slice: M002/S04
- Supporting slices: none
- Validation: validated
- Notes: test_job_phase_gates_failed_serde + exit-code-2 path in execute_run().

### R014 — smelt run --dry-run validates manifest without Docker
- Class: operability
- Status: validated
- Description: `smelt run --dry-run` parses and validates the manifest, prints the execution plan, and exits without provisioning a container.
- Why it matters: Users need a fast feedback loop to catch manifest errors before waiting for Docker.
- Source: user
- Primary owning slice: M001/S03
- Supporting slices: none
- Validation: validated
- Notes: 10 dry_run integration tests.

### R015 — smelt status shows live job progress
- Class: failure-visibility
- Status: validated
- Description: `smelt status` reads `.smelt/run-state.toml` and displays the current phase, container ID, elapsed time, and session list.
- Why it matters: Long-running jobs need an out-of-band status surface.
- Source: user
- Primary owning slice: M001/S05
- Supporting slices: none
- Validation: validated
- Notes: RunState written atomically at each phase transition.

### R002 — Job manifest supports forge configuration block
- Class: integration
- Status: validated
- Description: `JobManifest` accepts an optional `[forge]` section specifying the provider (github), repo (owner/repo), and token env var name. PR creation is skipped when `[forge]` is absent.
- Why it matters: Forge config is per-job — different jobs may target different repos or use different tokens. Manifest is the right place for it.
- Source: user
- Primary owning slice: M003/S02
- Supporting slices: none
- Validation: validated
- Notes: Proven by M003/S02 automated tests: roundtrip present/absent, validation (invalid repo format, empty token_env), deny_unknown_fields. GITHUB_TOKEN stays on host; never passed into container.

### R021 — Multi-machine coordination via Kubernetes
- Class: integration
- Status: validated
- Description: Smelt can run Assay sessions on remote machines via Kubernetes — a `KubernetesProvider: RuntimeProvider` creates a Pod on any cluster reachable via kubeconfig, clones the repo inside the Pod, runs Assay, and pushes the result branch back to the remote.
- Why it matters: Single-machine Docker execution limits session parallelism and resource availability. K8s enables remote, distributed execution on any cluster.
- Source: user (original vision, now active for M005)
- Primary owning slice: M005/S02
- Supporting slices: M005/S01, M005/S03, M005/S04
- Validation: validated
- Notes: M005 delivers single-node K8s proof (kind/minikube). S01 (manifest + generate_pod_spec + KubernetesConfig), S02 (full KubernetesProvider lifecycle integration tests against kind), S03 (push-from-Pod collection: SMELT_GIT_REMOTE injection, GitOps::fetch_ref(), Phase 8 host-side git fetch), S04 (CLI dispatch: AnyProvider::Kubernetes, ── Kubernetes ── dry-run section, dry-run integration test). All 5 slices complete. Automated proof: 27 dry-run tests, 155 smelt-core unit tests, k8s_lifecycle integration tests (SMELT_K8S_TEST=1). Live end-to-end proof (real kind cluster + real Assay image) deferred to S04-UAT.md. Parallel multi-session scheduling (R023) deferred to a later milestone.

### R023 — Parallel dispatch daemon (`smelt serve`)
- Class: core-capability
- Status: validated
- Description: `smelt serve` is a long-running daemon that accepts job manifests via directory watch or HTTP POST, dispatches up to N concurrent sessions (Docker/Compose/Kubernetes), enforces a `max_concurrent` cap, auto-retries failures with backoff, and exposes a Ratatui TUI for live observability.
- Why it matters: Single-job invocation is the limit of the current model. A parallel dispatch daemon enables autonomous multi-job workflows, headless server deployment, and programmatic job submission — without requiring Linear or any external tracker.
- Source: user
- Primary owning slice: M006/S01
- Supporting slices: M006/S02, M006/S03
- Validation: validated
- Notes: Proven across M006/S01 (JobQueue unit tests, concurrent dispatch, CancellationToken teardown), M006/S02 (DirectoryWatcher + HTTP API integration tests), M006/S03 (smelt serve assembly, HTTP smoke test, cargo test --workspace green). `smelt run` single-job path unchanged — zero regressions. Live end-to-end proof with real Docker jobs + Ctrl+C teardown deferred to S03-UAT.md.

### R024 — `smelt serve` HTTP API for job submission and status
- Class: integration
- Status: validated
- Description: `smelt serve` exposes a REST API: `POST /api/v1/jobs` (enqueue a manifest, return job_id), `GET /api/v1/jobs` (list all jobs), `GET /api/v1/jobs/:id` (single job state), `DELETE /api/v1/jobs/:id` (cancel queued job).
- Why it matters: Programmatic job submission enables CI integration, scripted batch runs, and future tracker integrations (Linear, GitHub Issues) without changing the manifest format.
- Source: user
- Primary owning slice: M006/S02
- Supporting slices: M006/S03
- Validation: validated
- Notes: Proven by M006/S02 integration tests (POST, GET, DELETE) and M006/S03 smoke test (GET /api/v1/jobs returns [] on clean start). JSON response format. TOML body for POST. No authentication in M006 (trusted local network assumed).

### R025 — Live terminal dashboard for `smelt serve`
- Class: failure-visibility
- Status: validated
- Description: When `smelt serve` runs, it displays a live Ratatui TUI table showing all queued, running, and completed jobs with job name, runtime, phase, attempt count, elapsed time, and exit status — updating in real time.
- Why it matters: Without live observability, an operator cannot tell which jobs are running, which are queued, or which failed — especially when dispatching many concurrent sessions.
- Source: user
- Primary owning slice: M006/S03
- Supporting slices: M006/S01
- Validation: validated
- Notes: Proven by M006/S03: ratatui TUI thread implemented; test_tui_render_no_panic via TestBackend confirms render doesn't panic; tracing redirected to .smelt/serve.log in TUI mode. Full live rendering with real Docker jobs deferred to S03-UAT.md. TUI is default-on; `--no-tui` disables it.

---

## Active

### R040 — Zero-warning cargo doc
- Class: quality-attribute
- Status: validated
- Description: `cargo doc --workspace --no-deps` exits 0 with zero warnings and zero errors.
- Why it matters: Broken doc builds prevent publishing to docs.rs and signal unmaintained code.
- Source: user
- Primary owning slice: M009/S01
- Supporting slices: none
- Validation: validated
- Notes: Proven by M009/S01: `cargo doc --workspace --no-deps` exits 0 with zero warnings. Broken intra-doc link in ssh.rs fixed (D070 backtick-only). All public items documented.

### R041 — Workspace README with usage documentation
- Class: launchability
- Status: validated
- Description: A comprehensive `README.md` at the workspace root explains what Smelt is, how to install it, and documents all subcommands with examples.
- Why it matters: No README exists. New users and contributors have no entry point.
- Source: user
- Primary owning slice: M009/S02
- Supporting slices: none
- Validation: validated
- Notes: Proven by M009/S02: 335-line README.md covers install, quickstart, all 6 subcommands (init, list, run, serve, status, watch) with exact flags from --help, server mode, examples directory, and Smelt/Assay/Cupel ecosystem. Human readability UAT in S02-UAT.md.

### R042 — deny(missing_docs) on smelt-cli
- Class: quality-attribute
- Status: validated
- Description: `#![deny(missing_docs)]` is enforced on `smelt-cli` and compiles without warnings. All public items have doc comments.
- Why it matters: smelt-core already enforces this (D070); smelt-cli should match. Undocumented public API is a maintenance liability.
- Source: user
- Primary owning slice: M009/S01
- Supporting slices: none
- Validation: validated
- Notes: Proven by M009/S01: `#![deny(missing_docs)]` in lib.rs compiles clean; all ~37 public items documented (D127). Self-enforcing — future undocumented items fail the build.

### R043 — No stale #[allow] annotations in production code
- Class: quality-attribute
- Status: validated
- Description: Every `#[allow(dead_code)]` or similar suppression in production code is either removed (code is now used) or justified with a comment referencing why the suppression is necessary.
- Why it matters: Stale annotations mask real dead code and signal neglect.
- Source: user
- Primary owning slice: M009/S01
- Supporting slices: none
- Validation: validated
- Notes: Proven by M009/S01: all 4 annotations audited — 2 removed (MockSshClient::with_probe_result was used in 12+ test sites; tests/docker_lifecycle.rs doesn't exist as source), 2 kept with updated rationale (retry_backoff_secs: serde forward-compat; PodState: fields stored for future use).

### R044 — Large file decomposition
- Class: quality-attribute
- Status: validated
- Description: Files over 500 lines are decomposed into focused modules along natural seams. Targets: run.rs (755L), ssh.rs (978L), serve/tests.rs (1322L).
- Why it matters: Large files are harder to navigate, review, and modify without merge conflicts.
- Source: user
- Primary owning slice: M009/S03
- Supporting slices: none
- Validation: validated
- Notes: Proven by M009/S03: run/mod.rs 116L (< 300), ssh/mod.rs 111L (< 400), tests/mod.rs 88L (< 500). All 286 tests pass. All public API signatures preserved via re-exports. deny(missing_docs) compiles clean.

### R045 — Example manifest documentation
- Class: launchability
- Status: validated
- Description: All example manifests in `examples/` have inline field-level comments explaining every field, valid defaults, and when to use each option.
- Why it matters: Examples are the primary learning tool. Uncommented examples force users to read source code.
- Source: user
- Primary owning slice: M009/S02
- Supporting slices: none
- Validation: validated
- Notes: Proven by M009/S02: all 7 example files have field-level comments (22-47 comment lines each); agent-manifest.toml fixed from broken to valid; bad-manifest.toml documents all 7 intentional errors with VIOLATION comments; all parseable examples verified with --dry-run.

### R050 — Bearer token authentication on smelt serve HTTP API
- Class: compliance/security
- Status: validated
- Description: `smelt serve` HTTP API supports optional bearer token authentication. When `[auth]` is configured in `server.toml`, requests without a valid `Authorization: Bearer <token>` header are rejected with 401. Tokens are configured via env var names (not raw values).
- Why it matters: Without authentication, any client on the network can enqueue arbitrary manifests, cancel jobs, or read job state. Required for deployment beyond localhost.
- Source: user
- Primary owning slice: M010/S01
- Supporting slices: M010/S03
- Validation: validated
- Notes: Proven by M010/S01: 4 integration tests (test_auth_missing_header_returns_401, test_auth_invalid_token_returns_403, test_auth_read_token_permission_split, test_auth_write_only_mode). Startup fails fast on missing/empty env vars. 401 JSON error for missing/malformed header. Follows D014/D112 env var passthrough pattern. Auth is opt-in — no `[auth]` = current behavior. S03 documents config.

### R051 — Read/write permission split for API tokens
- Class: compliance/security
- Status: validated
- Description: Two token levels: read-only (GET endpoints only) and read-write (all endpoints). A read-only token receives 403 Forbidden on POST/DELETE. A read-write token has full access.
- Why it matters: Monitoring systems and dashboards need read access without the ability to enqueue or cancel jobs.
- Source: user
- Primary owning slice: M010/S01
- Supporting slices: M010/S03
- Validation: validated
- Notes: Proven by M010/S01: test_auth_read_token_permission_split (read token GET→200, POST→403, DELETE→403; write token all→200) and test_auth_write_only_mode (no read token configured, write token full access). Two env var fields in `[auth]`: `read_token_env` and `write_token_env`. Write token implicitly has read access.

### R052 — Teardown error visibility
- Class: failure-visibility
- Status: validated
- Description: Container teardown failures produce visible `eprintln!` warnings instead of silent `let _ =` discards. Error chains are preserved via `.context()` instead of `anyhow!("{e}")`.
- Why it matters: Silent teardown failures leave orphaned containers and corrupt monitor state with no indication to the user.
- Source: execution (PR #33 review backlog)
- Primary owning slice: M010/S02
- Supporting slices: none
- Validation: validated
- Notes: Proven by M010/S02: `warn_teardown()` helper replaces 6 duplicated teardown blocks; 5 `anyhow!("{e}")` replaced with `.context()`; `rg 'let _ = provider\.teardown' phases.rs` returns 0; `rg 'anyhow!.*\{e\}' phases.rs` returns 0; all 155+ tests pass.

### R053 — SSH argument builder DRY cleanup
- Class: quality-attribute
- Status: validated
- Description: `build_ssh_args` and `build_scp_args` in the SSH client share a common helper instead of duplicating ~90% identical flag-building logic.
- Why it matters: Duplicated logic means any SSH flag change must be made in two places, risking divergence.
- Source: execution (PR #33 review backlog)
- Primary owning slice: M010/S02
- Supporting slices: none
- Validation: validated
- Notes: Proven by M010/S02: `build_common_ssh_args()` private helper extracted; both public methods are single-line delegations; 4 existing SSH arg tests pass unchanged; `cargo clippy`/`cargo doc` clean.

---

## Deferred

### R022 — Budget/cost tracking
- Class: admin/support
- Status: deferred
- Description: Track token/API cost per job run and surface in `smelt status`.
- Why it matters: Uncontrolled costs are a production risk.
- Source: inferred (brainstorm)
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: Requires token counting from Assay's output. Deferred until Assay surfaces cost data.

### R026 — Linear/GitHub Issues backlog integration for `smelt serve`
- Class: integration
- Status: deferred
- Description: `smelt serve` can poll a Linear project or GitHub Issues label for `Todo` issues and automatically dispatch an Assay session per issue, managing the full lifecycle from issue state transitions through PR creation and merge.
- Why it matters: Closes the autonomous loop from tracker to merged PR without any human dispatch step — the target end state for unattended agentic development workflows.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: unmapped
- Notes: Requires R023 (smelt serve parallel dispatch) and Assay changes to accept issue context. Deferred until M006 proves the dispatch daemon.

### R027 — SSH worker pools / remote dispatch
- Class: integration
- Status: validated
- Description: `smelt serve` can distribute job execution to remote machines via SSH — static `[[workers]]` list in `server.toml`, manifest delivered via scp, `smelt run` executed on the remote, state synced back to dispatcher.
- Why it matters: Remote dispatch enables multi-machine parallelism and resource isolation for large workloads without cloud infrastructure.
- Source: user (inspired by Symphony SSH worker pools)
- Primary owning slice: M008/S04
- Supporting slices: M008/S01, M008/S02, M008/S03
- Validation: validated
- Notes: Proven by M008 (all 4 slices complete). SSH subprocess approach (D111), MockSshClient integration tests, round-robin + failover + worker_host visibility. Live multi-host proof deferred to S04-UAT.md.

### R028 — Persistent queue across `smelt serve` restarts
- Class: operability
- Status: validated
- Description: Jobs queued in-memory at the time of a `smelt serve` crash or restart are automatically re-queued on the next startup; attempt counts preserved.
- Why it matters: Crash recovery prevents lost work in long-running unattended deployments.
- Source: inferred
- Primary owning slice: M007/S03
- Supporting slices: M007/S01, M007/S02
- Validation: validated
- Notes: Proven by M007 (all 3 slices complete). Atomic state file (S02) + ServerState::load_or_new startup wiring (S03) + 52 tests green.

---

## Out of Scope

### R030 — Spec authoring and gate definition
- Class: anti-feature
- Status: out-of-scope
- Description: Smelt does not help users write `.assay/specs/*.toml` files or define quality gate criteria.
- Why it matters: Prevents scope bleed with Assay, which owns this entirely.
- Source: user (D001)
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Smelt generates ephemeral spec files from its own session format; it does not help users author persistent Assay specs.

### R031 — smelt merges PRs automatically
- Class: anti-feature
- Status: out-of-scope
- Description: Smelt does not merge PRs. It creates them and tracks status, but the merge is always a human decision.
- Why it matters: Auto-merge without human review would remove the quality gate that justifies the PR step.
- Source: user
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: smelt watch exits 0 when a PR is merged by a human; it does not initiate merges.

### R032 — Web/mobile companion app
- Class: anti-feature
- Status: out-of-scope
- Description: Human interaction with Smelt is through the forge (GitHub PRs/Issues) and the CLI. No companion app.
- Why it matters: Developer tooling should integrate with existing workflows, not replace them.
- Source: user (original planning doc)
- Primary owning slice: none
- Supporting slices: none
- Validation: n/a
- Notes: Notification and escalation are forge primitives (PR comments, CI checks), not a custom app.

---

## Traceability

| ID   | Class                | Status      | Primary owner | Supporting           | Proof     |
|------|----------------------|-------------|---------------|----------------------|-----------|
| R001 | primary-user-loop    | validated   | M003/S02      | M003/S01,M003/S06    | validated |
| R002 | integration          | validated   | M003/S02      | none                 | validated |
| R003 | failure-visibility   | validated   | M003/S03      | M003/S02             | validated |
| R004 | operability          | validated   | M003/S03      | none                 | validated |
| R005 | integration          | validated   | M003/S05      | M003/S01,M003/S06    | validated |
| R006 | quality-attribute    | validated   | M003/S04      | none                 | validated |
| R007 | launchability        | validated   | M003/S04      | none                 | validated |
| R008 | quality-attribute    | validated   | M003/S04      | none                 | validated |
| R010 | core-capability      | validated   | M001/S02      | M001/S05             | validated |
| R011 | integration          | validated   | M002/S01      | M002/S02             | validated |
| R012 | failure-visibility   | validated   | M002/S03      | none                 | validated |
| R013 | failure-visibility   | validated   | M002/S04      | none                 | validated |
| R014 | operability          | validated   | M001/S03      | none                 | validated |
| R015 | failure-visibility   | validated   | M001/S05      | none                 | validated |
| R020 | core-capability      | validated   | M004/S03      | M004/S01,S02,S04     | validated |
| R021 | integration          | validated   | M005/S02      | M005/S01,S03,S04     | validated |
| R022 | admin/support        | deferred    | none          | none                 | unmapped  |
| R023 | core-capability      | validated   | M006/S01      | M006/S02,S03         | validated |
| R024 | integration          | validated   | M006/S02      | M006/S03             | validated |
| R025 | failure-visibility   | validated   | M006/S03      | M006/S01             | validated |
| R026 | integration          | deferred    | none          | none                 | unmapped  |
| R027 | integration          | validated   | M008/S04      | M008/S01,S02,S03     | validated |
| R028 | operability          | validated   | M007/S03      | M007/S01,S02         | validated |
| R030 | anti-feature         | out-of-scope| none          | none                 | n/a       |
| R031 | anti-feature         | out-of-scope| none          | none                 | n/a       |
| R032 | anti-feature         | out-of-scope| none          | none                 | n/a       |
| R040 | quality-attribute    | validated   | M009/S01      | none                 | validated |
| R041 | launchability        | validated   | M009/S02      | none                 | validated |
| R042 | quality-attribute    | validated   | M009/S01      | none                 | validated |
| R043 | quality-attribute    | validated   | M009/S01      | none                 | validated |
| R044 | quality-attribute    | validated   | M009/S03      | none                 | validated |
| R045 | launchability        | validated   | M009/S02      | none                 | validated |
| R050 | compliance/security  | validated   | M010/S01      | M010/S03             | validated |
| R051 | compliance/security  | validated   | M010/S01      | M010/S03             | validated |
| R052 | failure-visibility   | validated   | M010/S02      | none                 | validated |
| R053 | quality-attribute    | validated   | M010/S02      | none                 | validated |

## Coverage Summary

- Active requirements: 0
- Mapped to slices: 0
- Validated (all milestones through M010/S02): 31 (R001–R008, R010–R015, R020, R021, R023, R024, R025, R027, R028, R040, R041, R042, R043, R044, R045, R050, R051, R052, R053)
- Unmapped active requirements: 0
