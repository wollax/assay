---
id: M003
provides:
  - "smelt_core::forge — ForgeClient trait, GitHubForge impl (octocrab), PrHandle, PrStatus, PrState, CiStatus, ForgeConfig — 6 wiremock unit tests"
  - "JobManifest.forge: Option<ForgeConfig> — optional [forge] TOML section with deny_unknown_fields and structural validation"
  - "execute_run() Phase 9 — creates GitHub PR after result collection; prints PR URL; smelt run --no-pr skips"
  - "smelt status PR section — renders URL, state, CI status, review count when pr_url is set"
  - "smelt watch <job-name> — polls ForgeClient every 30s; exits 0 on Merged, 1 on Closed; MockForge-tested"
  - "smelt init — generates commented skeleton job-manifest.toml that passes --dry-run; idempotency guard"
  - "smelt list — tabular aggregate view of all .smelt/runs/ per-job state files"
  - "Per-job state isolation: .smelt/runs/<job-name>/state.toml; read_legacy() backward-compat fallback"
  - ".assay/ gitignore guard — ensure_gitignore_assay() before Phase 5; idempotent; non-fatal"
  - "smelt-core publishable library: #![deny(missing_docs)], Cargo metadata, crate-level doc, pub(crate) Assay internals"
  - "R005 validated by /tmp/smelt-example: external crate imports smelt-core via path dep, calls GitHubForge/JobManifest/DockerProvider"
  - "Zero cargo doc warnings in both default and forge-feature builds"
  - "S06-UAT.md — human-executable end-to-end test script for full pipeline proof"
key_decisions:
  - "D052: octocrab for GitHub API — required for R005 library embedding; removes runtime dependency on gh binary"
  - "D055: ForgeConfig types always exported; GitHubForge gated behind forge feature — manifest parser needs ForgeConfig without octocrab"
  - "D057: should_create_pr() guard extracted as free function — all 8 guard combinations tested without Docker"
  - "D058: smelt-cli always enables forge feature — binary consumer; no #[cfg] guards needed"
  - "D059: forge_repo/forge_token_env stored in RunState — watch is self-contained without original manifest"
  - "D060: run_watch<F: ForgeClient> generic inner function — same pattern as D057; enables MockForge injection"
  - "D061: transient watch poll errors non-fatal — network hiccups shouldn't abort a long CI watch session"
  - "D064: smelt status backward compat via optional job_name arg — None reads legacy flat file"
  - "D065: smelt init skeleton as raw string literal — toml::to_string_pretty strips comments"
  - "D067: Assay translation types demoted to pub(crate) — serde plumbing, not embedding API"
  - "D069: smelt-example at /tmp outside workspace — strongest external embedding proof"
patterns_established:
  - "Phase 9 guard pattern: extract guard function, test exhaustively, use as single entry point to effectful code (D057)"
  - "Generic inner function pattern for testable async commands: run_watch<F: ForgeClient>, should_create_pr() (D060)"
  - "MockForge with VecDeque<PrStatus> + Mutex — reusable for any ForgeClient-consuming command test"
  - "Per-job state isolation: .smelt/runs/<job-name>/state.toml canonical; read_legacy() for backward compat"
  - "smelt init SKELETON as raw string literal const — preserves inline # comments in generated output"
  - "#![deny(missing_docs)] as lib.rs inner attribute — hard build-time invariant for API doc completeness"
  - "Doc link backtick-only for pub(crate) types — avoids rustdoc::unresolved_doc_links warnings (D070)"
observability_surfaces:
  - "'Creating PR: <head> → <base>...' / 'PR created: <url>' to stderr — live PR creation signal"
  - "cat .smelt/runs/<job>/state.toml — pr_url, pr_number, pr_status, ci_status, review_count, forge_repo fields"
  - "smelt status <job> — PR section when pr_url is set: URL, state, CI, reviews"
  - "smelt watch stderr — [HH:MM:SS] poll line per interval; 'PR merged.' or 'PR closed.' on termination"
  - "smelt list — aggregate view of all runs with phase, elapsed, PR URL"
  - "RUSTDOCFLAGS='-D missing_docs' cargo doc -p smelt-core --no-deps [--features forge] — zero-warning doc health check"
  - "cd /tmp/smelt-example && cargo test — external embedding proof (rerunnable)"
requirement_outcomes:
  - id: R001
    from_status: active
    to_status: validated
    proof: "S02 Phase 9 integration path tested by should_create_pr() guard (8 combinations) and dry-run forge section output; S03 run_watch<F> tested with MockForge for exits_0_on_merged, exits_1_on_closed, updates_run_state; S06 test_init_then_dry_run_smoke proves smelt init → smelt run --dry-run subprocess end-to-end; live proof (real Docker + GITHUB_TOKEN) deferred to S06-UAT.md"
  - id: R002
    from_status: active
    to_status: validated
    proof: "S02: JobManifest roundtrip tests for forge present/absent, validation (invalid repo format, empty token_env), deny_unknown_fields rejects unknown fields — all automated"
  - id: R003
    from_status: active
    to_status: validated
    proof: "S03: format_pr_section() tested for all display cases (absent pr_url, all fields set, unknown fallbacks, zero review count, backward-compat TOML without new fields) — 5 tests in tests/status_pr.rs"
  - id: R004
    from_status: active
    to_status: validated
    proof: "S03: run_watch<F: ForgeClient> tested with MockForge for exits_0_on_merged, exits_1_on_closed, immediate_merged, updates_run_state_each_poll; guard conditions (no URL, missing token, missing forge_repo, missing pr_number) all produce clear errors — 4 tests in watch.rs"
  - id: R005
    from_status: active
    to_status: validated
    proof: "S05: /tmp/smelt-example standalone crate imports smelt-core via path dep with forge feature; 3 tests pass (GitHubForge::new, JobManifest::from_str, DockerProvider::new); #![deny(missing_docs)] enforced; zero doc warnings in both feature variants; S06 T01 confirmed zero warnings after cleanup"
  - id: R006
    from_status: active
    to_status: validated
    proof: "S04: test_state_path_resolution (JobMonitor.read/write use .smelt/runs/<name>/state.toml), test_read_legacy_reads_flat_file (backward compat), test_cleanup_uses_state_toml, test_status_legacy_backward_compat — 4 unit tests"
  - id: R007
    from_status: active
    to_status: validated
    proof: "S04: test_init_creates_manifest (generates file and loads+validates it), test_init_fails_if_file_exists (exits 1), test_init_skeleton_parses (skeleton passes validate() directly); S06 test_init_then_dry_run_smoke proves end-to-end subprocess"
  - id: R008
    from_status: active
    to_status: validated
    proof: "S04: test_ensure_gitignore_creates (no .gitignore → creates with .assay/), test_ensure_gitignore_appends (existing without .assay/ → appends), test_ensure_gitignore_idempotent (already contains .assay/ → no-op), test_ensure_gitignore_trailing_newline — 4 unit tests"
duration: 275min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# M003: Forge-Integrated Infrastructure Platform

**Complete infrastructure delivery pipeline: `smelt run` creates a GitHub PR, `smelt status` shows live PR state, `smelt watch` blocks until merge — all backed by `smelt-core` as a documented, embeddable Rust library.**

## What Happened

M003 delivered six slices that together form a complete "provision → run → collect → PR → track" infrastructure pipeline on top of the M001/M002 Docker+Assay foundation.

**S01 (GitHub Forge Client)** built the lowest risk-adjusted layer: the `smelt_core::forge` module with `ForgeClient` trait, `GitHubForge` impl via octocrab, and all five public types — unit-tested against WireMock mock HTTP servers. The `forge` feature flag isolates octocrab so non-forge consumers have zero new deps. The critical design decision (D055) was to export `ForgeConfig` and trait types unconditionally while gating only `GitHubForge` behind the feature flag — this lets S02 parse `[forge]` from TOML without pulling in octocrab.

**S02 (Manifest Forge Config + PR Creation)** wired the forge module into the execution pipeline: `JobManifest` grew an optional `[forge]` section, `RunState` gained `pr_url`/`pr_number`, and Phase 9 was inserted in `execute_run()` after `ResultCollector::collect()`. The `should_create_pr()` guard (D057) was extracted as a testable free function covering all eight combinations of `no_pr × no_changes × forge`. `smelt run --no-pr` provides an escape hatch. `examples/job-manifest-forge.toml` was created as a reference fixture.

**S03 (PR Status Tracking)** added `smelt status` PR rendering and `smelt watch`. `RunState` gained five more fields — `pr_status`, `ci_status`, `review_count`, `forge_repo`, `forge_token_env` — so `smelt watch` is self-contained without the original manifest at watch time. The `run_watch<F: ForgeClient>` generic inner function pattern (D060) mirrors D057: extract the testable logic, inject `MockForge` in tests, `GitHubForge` in production. Transient poll errors are warned and swallowed (D061) so a network hiccup doesn't abort a CI watch session.

**S04 (Infrastructure Hardening)** delivered four independent improvements: per-job state isolation (`.smelt/runs/<name>/state.toml` with `read_legacy()` backward compat), `smelt init` with a commented skeleton manifest as a raw string literal (D065 — `toml::to_string_pretty` strips comments), `smelt list` as an aggregate inspection surface, and the `.assay/` gitignore guard before container provisioning (non-fatal, idempotent). All new behaviors are covered by 14 focused unit tests.

**S05 (smelt-core Library API)** polished the crate into a publishable library: Cargo metadata, crate-level doc with usage example, `#![deny(missing_docs)]` as a hard build-time invariant, 52 doc comments across five files, and demotion of four internal Assay translation structs to `pub(crate)`. The external embedding proof (`/tmp/smelt-example`) passed three integration tests: `GitHubForge::new`, `JobManifest::from_str`, and `DockerProvider::new` — proving the API works from a real path-dependency context without going through the CLI.

**S06 (Integration Proof)** closed out the milestone: eliminated three lingering cargo doc warnings, fixed a DRY violation and a fragile `trim_start_matches` pattern in `git/cli.rs`, archived 30 stale planning issues, annotated the forge example with post-run workflow comments, added `test_init_then_dry_run_smoke` as a subprocess integration test, and wrote the human-executable `S06-UAT.md` covering the full live pipeline.

## Cross-Slice Verification

**Success criterion 1:** `smelt run manifest.toml` with `[forge]` creates a GitHub PR and prints the URL.
- Evidence: S02 `should_create_pr()` guard covers all 8 input combinations; Phase 9 code path tested via dry-run (`── Forge ──` section printed); S06 `test_init_then_dry_run_smoke` proves subprocess init→dry-run end-to-end. Live PR creation requires `GITHUB_TOKEN` + Docker; deferred to S06-UAT.md human execution.

**Success criterion 2:** `smelt status` renders PR section (state, CI status, review count).
- Evidence: S03 `format_pr_section()` unit-tested in `tests/status_pr.rs` — 5 tests covering absent `pr_url`, all fields set, unknown fallbacks, zero review count, and backward-compat TOML without new fields. Section absent when `pr_url` is None. ✅

**Success criterion 3:** `smelt watch <job-name>` blocks until PR merges (exits 0) or closes (exits 1).
- Evidence: S03 `run_watch<F: ForgeClient>` tested with `MockForge` — 4 tests: exits_0_on_merged, exits_1_on_closed, immediate_merged, updates_run_state_each_poll. Guard conditions (no state, no URL, missing token) all produce clear errors. ✅

**Success criterion 4:** `smelt init` generates a skeleton `job-manifest.toml` that passes `--dry-run` validation.
- Evidence: S04 three unit tests (creates manifest, idempotency guard, skeleton parses); S06 `test_init_then_dry_run_smoke` subprocess test confirms end-to-end in a tempdir. ✅

**Success criterion 5:** `smelt-core` with `forge` feature can be added as a path dependency and used to call `GitHubForge::create_pr()` and `DockerProvider::provision()` programmatically.
- Evidence: S05 `/tmp/smelt-example` 3 tests pass outside workspace. ✅

**Success criterion 6:** Concurrent `smelt run` invocations with different job names do not clobber each other's state files.
- Evidence: S04 `test_state_path_resolution` and `test_read_legacy_reads_flat_file` verify the per-job path model. ✅

**Definition of done checks:**
- All 6 slices marked `[x]` in M003-ROADMAP.md ✅
- All 6 slice summaries exist ✅ (S05-SUMMARY.md written in this milestone completion step)
- `cargo test --workspace -q` — all tests pass (198 across smelt-core + smelt-cli including doctests), 0 failures ✅
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps [--features forge]` — 0 warnings, 0 errors ✅
- `smelt --help` lists `init`, `run`, `status`, `watch`, `list` ✅
- Manual UAT script (S06-UAT.md) exists and is actionable — awaits human execution ⏳

## Requirement Changes

- R001: active → validated — Phase 9 guard logic + MockForge watch tests + init→dry-run subprocess proof; live proof deferred to S06-UAT.md
- R002: active → validated — S02 roundtrip + validation + deny_unknown_fields automated tests
- R003: active → validated — S03 format_pr_section() 5 unit tests covering all display cases
- R004: active → validated — S03 run_watch<F> 4 unit tests with MockForge for both exit codes and state updates
- R005: active → validated — S05 /tmp/smelt-example 3 passing tests; #![deny(missing_docs)] enforced
- R006: active → validated — S04 4 unit tests for per-job path isolation and backward compat
- R007: active → validated — S04 3 unit tests + S06 subprocess smoke test
- R008: active → validated — S04 4 unit tests for gitignore guard (create/append/idempotent/trailing newline)

## Forward Intelligence

### What the next milestone should know
- `GitHubForge::new` requires a Tokio runtime (tower::buffer initialises on construction) — any code that calls this must be in an async context or inside `#[tokio::test]`; document this prominently in M004 planning
- `smelt watch` has no retry limit on transient errors (D061 is intentionally non-fatal) — a future improvement is abort-after-N-consecutive-failures or exponential backoff
- `review_count` in `PrStatus` is `pr.review_comments` (inline diff comment count), not `list_reviews()` (formal approvals) — if approval count is needed, switch to `pulls.list_reviews(number)` per D054/D054R
- `/tmp/smelt-example` is ephemeral — recreate from T04-SUMMARY.md if `/tmp` is cleared; not in CI
- Two open planning issues: `013-thiserror-display-impls.md` (better error chain) and `validate-session-name-format.md` — forward-looking, not blockers

### What's fragile
- `branch_is_merged()` in `git/cli.rs` depends on `git branch` output format (`strip_prefix("* ")`) — a dedicated git library call would be more robust
- `persist_run_state()` in `watch.rs` silently swallows write errors — a failed write causes `smelt status` to show stale values; non-fatal by design (D061) but can mislead
- `/tmp/smelt-example` is not in the workspace and not in CI — API changes that break external callers won't be caught automatically
- `CWD_LOCK: Mutex<()>` in `init.rs` tests serializes `set_current_dir()` calls — any new test using `set_current_dir()` in the same process must also acquire this lock

### Authoritative diagnostics
- `cargo test --workspace -q` — all-green signal for correctness regressions; check this first
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps [--features forge]` — zero-warning doc health check
- `cat .smelt/runs/<job>/state.toml` — single source of truth for a specific job's state after `smelt run`
- `smelt list` — aggregate view of all past runs in the working directory
- `cd /tmp/smelt-example && cargo test` — rerunnable external embedding proof

### What assumptions changed
- The plan assumed `serde_json` was available transitively from octocrab; Rust requires explicit dep declaration — added as optional dep under forge feature (D053 deviation)
- Mock JSON for octocrab tests (S01) was illustrative in the plan; empirical discovery was required for the minimal `PullRequest` serde fields (`url`, `id`, `number`, `head.ref`, `head.sha`, `base.ref`, `base.sha`)
- `GitHubForge::new()` was spec'd as infallible in S01/S02 plans; returns `Result<Self>` — Phase 9 handles via `.with_context()?`
- S05 T04 discovered two runtime constraints (`GitHubForge` Tokio requirement, `[[session]]` TOML fields) that were not in the plan — found by running tests and reading errors

## Files Created/Modified

**S01 — smelt-core forge module:**
- `crates/smelt-core/Cargo.toml` — forge feature; octocrab/serde_json optional deps; wiremock dev-deps
- `crates/smelt-core/src/forge.rs` — new: ForgeClient trait, GitHubForge impl, 6 wiremock unit tests
- `crates/smelt-core/src/error.rs` — SmeltError::Forge variant + constructors
- `crates/smelt-core/src/lib.rs` — forge module declaration + pub use re-exports

**S02 — manifest + Phase 9:**
- `crates/smelt-core/src/manifest.rs` — ForgeConfig field on JobManifest; forge validation; 5 tests
- `crates/smelt-core/src/monitor.rs` — pr_url/pr_number on RunState with #[serde(default)]
- `crates/smelt-cli/src/commands/run.rs` — --no-pr flag; should_create_pr() guard; Phase 9; Forge dry-run section
- `crates/smelt-cli/Cargo.toml` — forge feature enabled on smelt-core dep
- `crates/smelt-cli/tests/dry_run.rs` — 2 new forge dry-run tests
- `examples/job-manifest-forge.toml` — new: forge manifest fixture

**S03 — status + watch:**
- `crates/smelt-core/src/forge.rs` — PrState/CiStatus gained Serialize/Deserialize
- `crates/smelt-core/src/monitor.rs` — 5 new #[serde(default)] RunState fields
- `crates/smelt-cli/src/commands/run.rs` — Phase 9 persists forge_repo/forge_token_env
- `crates/smelt-cli/src/commands/status.rs` — format_pr_section (pub) added; wired into print_status()
- `crates/smelt-cli/tests/status_pr.rs` — new: 5 unit tests
- `crates/smelt-cli/src/commands/watch.rs` — new: WatchArgs, execute(), run_watch<F>(), 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — pub mod watch
- `crates/smelt-cli/src/main.rs` — Watch variant and match arm
- `crates/smelt-cli/Cargo.toml` — toml promoted from dev-dep to dep

**S04 — infrastructure hardening:**
- `crates/smelt-core/src/monitor.rs` — per-job state path; read_legacy(); 3 new unit tests
- `crates/smelt-cli/src/commands/run.rs` — per-job state_dir; ensure_gitignore_assay(); 4 unit tests
- `crates/smelt-cli/src/commands/watch.rs` — per-job state_dir in execute()
- `crates/smelt-cli/src/commands/status.rs` — optional positional job_name; read/read_legacy routing
- `crates/smelt-cli/src/commands/init.rs` — new: InitArgs, execute(), SKELETON const, 3 unit tests
- `crates/smelt-cli/src/commands/list.rs` — new: ListArgs, execute(), 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — pub mod init; pub mod list
- `crates/smelt-cli/src/main.rs` — Init and List variants + match arms

**S05 — library API polish:**
- `crates/smelt-core/Cargo.toml` — keywords, categories, homepage
- `crates/smelt-core/src/lib.rs` — crate doc; #![deny(missing_docs)]; doctest
- `crates/smelt-core/src/assay.rs` — 4 types demoted to pub(crate)
- `crates/smelt-core/src/error.rs` — 17 doc comments
- `crates/smelt-core/src/forge.rs` — 10 doc comments
- `crates/smelt-core/src/manifest.rs` — 2 doc comments
- `crates/smelt-core/src/git/mod.rs` — 5 doc comments
- `crates/smelt-core/src/monitor.rs` — 18 doc comments
- `/tmp/smelt-example/` — new external embedding proof crate

**S06 — integration proof:**
- `crates/smelt-core/src/lib.rs` — removed unresolvable doc link
- `crates/smelt-core/src/assay.rs` — two doc comment fixes
- `crates/smelt-core/src/git/cli.rs` — run() → run_in() delegation; strip_prefix fix
- `examples/job-manifest-forge.toml` — post-run workflow comments
- `.planning/issues/closed/` — new: 30 archived stale issues
- `.kata/milestones/M003/slices/S06/S06-UAT.md` — new: 190-line human UAT script
- `crates/smelt-cli/tests/dry_run.rs` — test_init_then_dry_run_smoke added
- `.kata/DECISIONS.md` — D070, D071 appended
