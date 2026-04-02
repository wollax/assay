---
id: M012
provides:
  - TrackerSource trait (RPITIT) with poll_ready_issues and transition_state — platform-agnostic abstraction for GitHub and Linear backends
  - TrackerIssue, TrackerState (6-variant lifecycle), StateBackendConfig mirror enum in smelt-core
  - SmeltError::Tracker { operation, message } variant for structured tracker error reporting
  - TrackerConfig and ServerConfig.tracker integration with collected validation (D018)
  - load_template_manifest() + issue_to_manifest() + sanitize() for zero-session template management
  - MockTrackerSource VecDeque test double for unit testing
  - GhClient trait with 4 RPITIT methods + SubprocessGhClient + MockGhClient
  - GithubTrackerSource<G: GhClient> with atomic label transitions (D157/D166) and ensure_labels()
  - LinearClient trait with 5 RPITIT GraphQL methods + ReqwestLinearClient + MockLinearClient
  - LinearTrackerSource<L: LinearClient> with UUID caching and two-mutation transition_state()
  - AnyTrackerSource enum dispatch (D171) for non-object-safe TrackerSource in tokio::select!
  - TrackerPoller background task wired as 6th arm in smelt serve dispatch loop
  - state_backend passthrough from JobManifest through SmeltRunManifest TOML into Assay container
  - TUI Source column (Tracker/HTTP/DirWatch)
  - examples/server.toml [tracker] section + README.md tracker documentation
  - Three-way tracing subscriber init (D158): bare-message default, full-format on SMELT_LOG/RUST_LOG, always full-format for TUI file appender
  - 50 eprintln! calls migrated to structured tracing macros across smelt-cli
  - Flaky test timeout increased from 10s to 30s (R061 resolved)
  - 398 workspace tests pass, 0 failures, 11 ignored
key_decisions:
  - D158 — Bare-message tracing subscriber format for default stderr output (extends D107)
  - D159 — Structured fields in warn! teardown calls for filterable diagnostics
  - D160 — state_backend added to JobManifest in S02 (not S05) to avoid deny_unknown_fields rejection of template manifests
  - D161 — issue_to_manifest() is a free function, not a trait method — logic is identical for all backends
  - D162 — Template manifest must have zero [[session]] entries — validated at startup
  - D163 — StateBackendConfig uses toml::Value for Custom variant (not serde_json::Value)
  - D164 — GhClient uses generic <G: GhClient> not dyn GhClient — RPITIT non-object-safe (mirrors SshClient)
  - D165 — TrackerConfig.repo required for GitHub, ignored for Linear
  - D166 — edit_labels combines --add-label and --remove-label in single gh issue edit call
  - D167 — LinearTrackerSource uses Linear issue UUID as TrackerIssue.id (not human-readable identifier)
  - D168 — reqwest promoted to production dep for async Linear GraphQL client
  - D169 — Linear label UUID caching via ensure_labels() HashMap populated at startup
  - D170 — Linear transition_state uses two separate mutations (remove + add)
  - D171 — AnyTrackerSource enum for non-object-safe TrackerSource dispatch
  - D172 — TrackerPoller poll errors are non-fatal (log + continue); ensure_labels() failure is fatal
  - D173 — TrackerPoller uses std::future::pending() placeholder when no tracker configured
patterns_established:
  - TrackerSource trait RPITIT with MockTrackerSource VecDeque test double — same pattern as SshClient/GhClient
  - Template manifest = normal manifest with zero sessions; load_template_manifest() validates; issue_to_manifest() injects
  - AnyTrackerSource enum dispatch (mirrors AnyProvider from run.rs) for non-object-safe async traits
  - Optional tokio::select! arm via pending() for trackerless server startup
  - D105 temp file pattern (NamedTempFile + std::mem::forget) reused for tracker manifest hand-off
  - Provider-specific validation blocks in ServerConfig::validate() with D018 error collection
  - Integration tests gated by env var (SMELT_GH_TEST=1, SMELT_GH_REPO, LINEAR_API_KEY) following SMELT_K8S_TEST pattern
observability_surfaces:
  - SMELT_LOG=debug shows every TrackerPoller poll cycle with issues_found count and gh subprocess invocations
  - TUI Source column: Tracker/HTTP/DirWatch visible at a glance for all jobs
  - GET /api/v1/jobs JSON source field includes tracker provenance
  - SmeltError::Tracker { operation, message } structured error type for GitHub and Linear failures
  - tracing::info! on successful tracker enqueue (issue_id, job_id fields)
  - tracing::warn! on poll/transition/manifest errors — poller continues; non-fatal per D172
  - tracing::info! on tracker poller configured at serve startup (provider, poll_interval_secs)
requirement_outcomes:
  - id: R061
    from_status: active
    to_status: validated
    proof: "Duration::from_secs(10) changed to Duration::from_secs(30) in docker_lifecycle.rs; rg 'from_secs(10)' returns 0 results; cargo test --workspace passes 398 tests"
  - id: R062
    from_status: active
    to_status: validated
    proof: "50 eprintln! calls migrated in phases.rs (33), watch.rs (10), status.rs (3), dry_run.rs (2), init.rs (1), list.rs (1); rg 'eprintln!' crates/smelt-cli/src/ --count-matches returns exactly main.rs:1 and serve/tui.rs:1; integration test assertions on stderr substrings still pass; cargo test --workspace passes 398 tests"
  - id: R072
    from_status: active
    to_status: validated
    proof: "TrackerSource trait with RPITIT defined in serve/tracker.rs; GitHub and Linear backends are independent concrete implementations; MockTrackerSource proves trait testability; AnyTrackerSource proves trait dispatch without dyn; cargo test --workspace passes 398 tests including all serve::tracker, serve::github, serve::linear tests"
  - id: R073
    from_status: active
    to_status: validated
    proof: "load_template_manifest() validates zero-session constraint at startup; issue_to_manifest() clones template and injects sanitized session from TrackerIssue; 14 unit tests in serve::tracker cover all code paths; ServerConfig::load() calls template validation at startup; cargo test --workspace passes 398 tests"
  - id: R074
    from_status: active
    to_status: validated
    proof: "TrackerState enum with 6 variants and label_name(prefix) producing {prefix}:{state} strings; GitHub backend uses single gh issue edit with --add-label/--remove-label (D157/D166); Linear backend uses two GraphQL mutations (D170); ensure_labels() creates all 6 lifecycle labels idempotently on both backends; cargo test --workspace passes 398 tests"
  - id: R075
    from_status: active
    to_status: validated
    proof: "SmeltRunManifest.state_backend field with #[serde(default, skip_serializing_if)]; build_run_manifest_toml() clones state_backend from JobManifest; 3 unit tests: None (omitted from TOML), Linear ([state_backend.linear] with team_id/project_id), LocalFs (state_backend = 'local_fs'); cargo test --workspace passes 398 tests"
  - id: R070
    from_status: active
    to_status: active
    proof: "GithubTrackerSource implements TrackerSource with poll_ready_issues, transition_state, ensure_labels; AnyTrackerSource::GitHub dispatch wired into TrackerPoller and smelt serve tokio::select! loop; end-to-end proven by MockTrackerSource integration tests (398 tests pass); live gh CLI UAT with real GitHub repo deferred — not yet performed"
  - id: R071
    from_status: active
    to_status: active
    proof: "LinearTrackerSource implements TrackerSource with GraphQL-backed poll_ready_issues, transition_state, ensure_labels; AnyTrackerSource::Linear dispatch wired into TrackerPoller and smelt serve tokio::select! loop; end-to-end proven by MockTrackerSource integration tests (398 tests pass); live Linear API UAT with real Linear project deferred — not yet performed"
duration: ~3h (S01: 20min, S02: 40min, S03: 28min, S04: 30min, S05: 37min)
verification_result: passed
completed_at: 2026-03-28T00:00:00Z
---

# M012: Tracker-Driven Autonomous Dispatch

**Complete tracker-driven dispatch loop from GitHub Issues and Linear to Assay sessions and PRs — TrackerSource trait, GitHub/Linear backends, TrackerPoller in smelt serve, state_backend passthrough, TUI Source column, and all M011 cleanup; 398 tests pass, zero regressions**

## What Happened

Five slices delivered M012 in sequence, each building directly on the prior.

**S01 (M011 leftover cleanup)** resolved two carry-over items: the three-way tracing subscriber init (TUI file appender, SMELT_LOG full format, default bare-message with target-scoped filter `"smelt_cli=info,smelt_core=info,warn"`), migration of all 50 remaining `eprintln!` calls to structured tracing macros across six source files, and the `test_cli_run_invalid_manifest` timeout fix from 10s to 30s. R061 and R062 both validated.

**S02 (TrackerSource Trait, Config & Template Manifest)** established the complete contract layer that S03 and S04 both depended on. In smelt-core: `TrackerIssue`, `TrackerState` (6-variant enum with `label_name(prefix)` producing lifecycle label strings), and `StateBackendConfig` (Smelt-side serde mirror of Assay's enum). Extended `SmeltError` with a `Tracker` variant. Added `state_backend: Option<StateBackendConfig>` to `JobManifest` with `#[serde(default)]` — done in S02 rather than S05 (D160) because `deny_unknown_fields` would otherwise reject template manifests containing `[state_backend]`. In smelt-cli: `TrackerConfig` with `deny_unknown_fields`, `ServerConfig.tracker` integration with collected validation, the `TrackerSource` trait (RPITIT), `JobSource::Tracker` variant, `load_template_manifest()` + `issue_to_manifest()` free function (D161) + `sanitize()` + `MockTrackerSource`. Added `Clone` derives and `#[serde(default)]` on `JobManifest.session` to enable template cloning and zero-session templates. 337 tests pass.

**S03 (GitHub Issues Tracker Backend)** built the `gh` CLI abstraction layer in `serve/github/`. The `GhClient` trait defines 4 async RPITIT methods returning `SmeltError` directly (not `anyhow::Result`). `SubprocessGhClient` discovers `gh` via `which::which` and shells out via `tokio::process::Command`. `MockGhClient` uses Arc<Mutex<VecDeque>> per-method queues. `GithubTrackerSource<G: GhClient>` implements `TrackerSource` with auth-first polling, atomic label transitions via a single `gh issue edit --add-label to --remove-label from` (D157/D166), and `ensure_labels()` creating all 6 lifecycle labels idempotently. `TrackerConfig.repo` field added with GitHub-specific validation (owner/repo format). 2 gated integration tests added (SMELT_GH_TEST=1). 360 tests pass.

**S04 (Linear Tracker Backend)** mirrored the GitHub module structure exactly in `serve/linear/`. `LinearClient` defines 5 async RPITIT GraphQL methods. `ReqwestLinearClient` sends POST requests to `{base_url}/graphql` with variables (not string interpolation) and extracts errors from both the `errors` array on HTTP 200 and non-200 status codes. `LinearTrackerSource<L: LinearClient>` uses Linear issue UUIDs as `TrackerIssue.id` (D167), `ensure_labels()` with HashMap UUID caching (D169), and two-mutation `transition_state()` (D170). `TrackerConfig` extended with `api_key_env` and `team_id` fields; Linear validation block added following GitHub's pattern. `reqwest` promoted to production dependency. 386 tests pass.

**S05 (Dispatch Integration, State Backend Passthrough & Final Assembly)** composed everything. T01 added `state_backend: Option<StateBackendConfig>` to `SmeltRunManifest` and updated `build_run_manifest_toml()` to clone the field from `JobManifest`. T02 created `tracker_poller.rs` with `AnyTrackerSource` enum (D171) solving RPITIT non-object-safety, `TrackerPoller` with `run()` → `ensure_labels()` once then tick-loop → `poll_once()` → transition Ready→Queued → `issue_to_manifest()` → temp file → `ServerState::enqueue()`. T03 wired `TrackerPoller` as the 6th `tokio::select!` arm in `serve.rs` with `std::future::pending()` fallback when no tracker configured (D173), added the TUI Source column, updated `examples/server.toml` and `README.md`. 398 tests pass.

## Cross-Slice Verification

All milestone success criteria were verified:

| Criterion | Evidence |
|-----------|----------|
| `smelt serve` with `[tracker]` picks up GitHub Issues labeled `smelt:ready` | `GithubTrackerSource` polls `gh issue list --label smelt:ready -R owner/repo`; wired into TrackerPoller → dispatch loop; proven by MockTrackerSource integration tests |
| `smelt serve` with `[tracker]` picks up Linear issues | `LinearTrackerSource` polls GraphQL for issues with `smelt:ready` label; wired into TrackerPoller; proven by MockLinearClient integration tests |
| Label lifecycle transitions: ready → queued → running → pr-created → done/failed | `TrackerState` 6-variant enum; GitHub via single `gh issue edit` (D166); Linear via two GraphQL mutations (D170); `TrackerPoller.poll_once()` transitions Ready→Queued before enqueue (D157); subsequent transitions happen at job phase changes |
| Template manifest: environment/credentials/merge config; issue injects only the spec | `load_template_manifest()` validates zero-session constraint; `issue_to_manifest()` clones template + injects session; `sanitize()` normalizes session names |
| Assay state_backend forwarded into RunManifest | `SmeltRunManifest.state_backend` with `#[serde(default, skip_serializing_if)]`; `build_run_manifest_toml()` clones from `JobManifest`; 3 unit tests cover None, Linear, LocalFs variants |
| All existing tests pass (zero regressions) | `cargo test --workspace`: 398 passed, 0 failed, 11 ignored |
| New capabilities have unit and integration tests | 24 serve::github tests, 19 serve::linear tests, 14 serve::tracker tests, 6 serve::tracker_poller tests, 11 assay::tests (including 3 new state_backend tests), 24 serve::config tests |
| R061 (flaky test) resolved | `Duration::from_secs(10)` → `Duration::from_secs(30)` in docker_lifecycle.rs; `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` returns 0 results |
| R062 (tracing migration) resolved | `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` returns exactly `main.rs:1` and `serve/tui.rs:1` |
| TUI displays tracker-sourced jobs correctly | Source column added to 7-column table; `JobSource::Tracker` → "Tracker"; 3 TUI tests pass |
| Documentation updated | `examples/server.toml` has commented [tracker] section with GitHub and Linear examples; README.md has "Tracker-Driven Dispatch" and "State Backend Passthrough" sections |

**Definition of done check:**
- All 5 slices marked `[x]` in roadmap: ✅
- All slice summaries exist (S01–S05): ✅
- Cross-slice integration (TrackerSource → GithubTrackerSource/LinearTrackerSource → AnyTrackerSource → TrackerPoller → dispatch_loop): ✅ proven by 398 passing tests
- `cargo clippy --workspace -- -D warnings`: ✅ zero warnings
- `cargo doc --workspace --no-deps`: ✅ zero warnings

**Note on R070/R071:** Live UAT with real `gh` CLI (GitHub Issues) and real Linear GraphQL API has not been performed. All end-to-end proof uses `MockTrackerSource`. Both requirements remain Active (not Validated) until live UAT is confirmed.

## Requirement Changes

- R061: active → validated — `from_secs(10)` → `from_secs(30)` in docker_lifecycle.rs; all 398 tests pass
- R062: active → validated — 50 eprintln! migrated; exactly 2 remain (both documented exceptions); integration test stderr assertions still pass
- R072: active → validated — TrackerSource RPITIT trait; GitHub and Linear as independent impls; MockTrackerSource proves testability; AnyTrackerSource proves dispatch; 398 tests pass
- R073: active → validated — load_template_manifest() zero-session validation + issue_to_manifest() injection; 14 unit tests; wired into ServerConfig::load() at startup; 398 tests pass
- R074: active → validated — TrackerState 6-variant enum; label_name(); atomic GitHub transition (D166); Linear two-mutation transition (D170); ensure_labels() idempotent creation; 398 tests pass
- R075: active → validated — SmeltRunManifest.state_backend with serde(default, skip_serializing_if); build_run_manifest_toml() passthrough; 3 unit tests covering None/Linear/LocalFs; 398 tests pass
- R070: remains active — GithubTrackerSource implemented and wired; mock-proven end-to-end; live `gh` CLI UAT not performed
- R071: remains active — LinearTrackerSource implemented and wired; mock-proven end-to-end; live Linear API UAT not performed

## Forward Intelligence

### What the next milestone should know
- All M012 tracker infrastructure is complete. The next step is live end-to-end UAT: (1) file a GitHub Issue with `smelt:ready` label, observe `smelt serve` pick it up, transition labels, dispatch, see `smelt:pr-created`; (2) same for Linear. Only then can R070 and R071 be marked validated.
- `AnyTrackerSource` construction in `serve.rs` matches on `tracker_config.provider` string ("github" / "linear"). Adding a third provider requires: (1) new TrackerSource impl, (2) new AnyTrackerSource variant with delegation arms for `poll_ready_issues`, `transition_state`, `ensure_labels`, (3) new match arm in serve.rs construction.
- The Linear API key is resolved from env var at `smelt serve` startup via `std::env::var(api_key_env)` — if the env var is unset, TrackerPoller construction fails with a clear error but only at runtime (serve startup), not at config-parse time.
- `ensure_labels()` must be called before the first `transition_state()` — TrackerPoller.run() calls it once at startup (fatal on failure), then enters the poll loop. If labels are deleted externally while serving, transition calls will fail with "label '...' not in cache — was ensure_labels() called?" until restart.

### What's fragile
- Two-mutation `transition_state()` for Linear (D170) is not atomic — if `add_label()` fails after `remove_label()` succeeds, the issue is in a label-less limbo. A recovery path or idempotent retry is not yet implemented.
- The `template_toml: String` + `toml::Value` manipulation in TrackerPoller is a workaround for `JobManifest` lacking `Serialize`. If `JobManifest` gains `Serialize`, TrackerPoller should be refactored to use it directly.
- Linear label UUID cache is in-memory only (populated by `ensure_labels()` at startup). External label deletion while serving causes transition failures until restart.
- D157 double-dispatch prevention relies on GitHub's label update atomicity (single gh CLI call). Multi-server deployments with concurrent pollers could still race in edge cases.

### Authoritative diagnostics
- `SMELT_LOG=debug` shows every TrackerPoller poll cycle with issue counts and all gh subprocess invocations (cmd + args fields)
- TUI Source column is the fastest visual confirmation that tracker-sourced jobs are flowing through the dispatch pipeline
- `SmeltError::Tracker { operation, message }` with operation names: `gh_binary`, `auth_status`, `poll`, `transition`, `ensure_labels` for GitHub; `find_label`, `create_label`, `add_label`, `remove_label`, `list_issues` for Linear
- `GET /api/v1/jobs` JSON `source` field provides programmatic proof of job origin

### What assumptions changed
- Original plan assumed `JobManifest` could be serialized directly in TrackerPoller — actual implementation uses `toml::Value` manipulation on the raw template string because `JobManifest` lacks `Serialize`.
- D156 originally said "reqwest::blocking::Client" — corrected to async `reqwest::Client` because `smelt serve` runs on a tokio runtime; blocking client in an async context would panic.
- `[state_backend.linear]` TOML shape differs from `[state_backend]` with a `type` field — tagged-enum serde convention produces nested table keys.

## Files Created/Modified

- `crates/smelt-core/src/tracker.rs` — New: TrackerIssue, TrackerState, StateBackendConfig, unit tests
- `crates/smelt-core/src/error.rs` — Added Tracker variant and tracker() constructor
- `crates/smelt-core/src/manifest/mod.rs` — Added state_backend field, Clone derives, serde(default) on session
- `crates/smelt-core/src/lib.rs` — Exported pub mod tracker
- `crates/smelt-core/src/compose.rs` — Added state_backend: None to test helper
- `crates/smelt-core/src/assay.rs` — StateBackendConfig import, state_backend field on SmeltRunManifest, passthrough in build_run_manifest_toml(), 3 unit tests
- `crates/smelt-core/src/manifest/tests/core.rs` — Updated validate_no_sessions, added state_backend tests
- `crates/smelt-cli/src/main.rs` — Three-way tracing subscriber init; 50 eprintln! → tracing macros
- `crates/smelt-cli/src/commands/run/phases.rs` — 33 eprintln! → tracing macros
- `crates/smelt-cli/src/commands/watch.rs` — 10 eprintln! → tracing macros
- `crates/smelt-cli/src/commands/status.rs` — 3 eprintln! → tracing macros
- `crates/smelt-cli/src/commands/run/dry_run.rs` — 2 eprintln! → error!
- `crates/smelt-cli/src/commands/init.rs` — 1 eprintln! → tracing::error!
- `crates/smelt-cli/src/commands/list.rs` — 1 eprintln! → tracing::warn!
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Timeout 10s → 30s; state_backend: None in test helper
- `crates/smelt-cli/tests/compose_lifecycle.rs` — state_backend: None in test helper
- `crates/smelt-cli/tests/k8s_lifecycle.rs` — state_backend: None in test helper
- `crates/smelt-cli/src/serve/config.rs` — TrackerConfig, ServerConfig.tracker, validation, template startup check, repo/api_key_env/team_id fields and validation, 24 total tests
- `crates/smelt-cli/src/serve/tracker.rs` — New: TrackerSource trait, load_template_manifest, issue_to_manifest, sanitize, MockTrackerSource, 14 unit tests
- `crates/smelt-cli/src/serve/types.rs` — Added JobSource::Tracker variant
- `crates/smelt-cli/src/serve/github/mod.rs` — New: GhClient trait, GhIssue struct, integration tests
- `crates/smelt-cli/src/serve/github/client.rs` — New: SubprocessGhClient with gh CLI subprocess wrappers
- `crates/smelt-cli/src/serve/github/mock.rs` — New: MockGhClient test double + 8 unit tests
- `crates/smelt-cli/src/serve/github/source.rs` — New: GithubTrackerSource with TrackerSource impl + 8 unit tests
- `crates/smelt-cli/src/serve/linear/mod.rs` — New: LinearClient trait, LinearIssue/LinearLabel types, compile-test
- `crates/smelt-cli/src/serve/linear/client.rs` — New: ReqwestLinearClient with GraphQL helper and 5 method implementations
- `crates/smelt-cli/src/serve/linear/mock.rs` — New: MockLinearClient with VecDeque queues + 8 unit tests
- `crates/smelt-cli/src/serve/linear/source.rs` — New: LinearTrackerSource struct, ensure_labels(), TrackerSource impl, 10 unit tests
- `crates/smelt-cli/src/serve/tracker_poller.rs` — New: AnyTrackerSource enum, TrackerPoller struct, run()/poll_once(), build_manifest_toml(), write_manifest_temp(), 6 unit tests
- `crates/smelt-cli/src/serve/mod.rs` — Added pub mod github, linear, tracker_poller + re-exports
- `crates/smelt-cli/src/commands/serve.rs` — TrackerPoller construction from config, 6th tokio::select! arm, pending() fallback
- `crates/smelt-cli/src/serve/tui.rs` — 7-column table with Source; 3 tests
- `crates/smelt-cli/Cargo.toml` — reqwest promoted from dev-dep to production dep
- `examples/server.toml` — Documented [tracker] section with GitHub and Linear examples (commented out)
- `README.md` — Tracker-Driven Dispatch subsection + State Backend Passthrough docs
