# S06: Integration Proof

**Goal:** Deliver a clean, well-documented codebase ready for production use — with cargo doc warnings eliminated, stale issues triaged, quick-win code fixes applied, example manifests updated, and a written UAT script the user can execute to confirm the full `smelt run` → PR created → `smelt watch` → merge → exit 0 pipeline.
**Demo:** `cargo doc -p smelt-core --no-deps [--features forge]` exits with 0 warnings; `cargo test --workspace` shows all tests passing; `smelt --help` lists all commands including `watch`, `init`, and `list`; `.planning/issues/` is triaged with stale issues moved to `closed/`; a `S06-UAT.md` script exists and is ready for human execution against a real GitHub repo.

## Must-Haves

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` — 0 warnings (both with and without `--features forge`)
- `cargo test --workspace -q` — all tests pass, 0 failures
- All 32 open issues triaged: stale issues moved to `.planning/issues/closed/`, actionable quick-wins fixed
- `crates/smelt-core/src/git/cli.rs` — `run()` delegates to `run_in(&self.repo_root, args)` (DRY fix)
- `crates/smelt-core/src/git/cli.rs` — `branch_is_merged()` uses `strip_prefix("* ")` instead of `trim_start_matches("* ")` (fragility fix)
- `examples/job-manifest-forge.toml` — includes inline comments about `smelt watch` and `smelt status` post-run usage
- `S06-UAT.md` — exists in `.kata/milestones/M003/slices/S06/` and provides a step-by-step live end-to-end test script

## Proof Level

- This slice proves: final-assembly (cleanup + UAT script ready for human execution)
- Real runtime required: yes (for the human UAT step — requires real Docker, real `GITHUB_TOKEN`, real GitHub repo)
- Human/UAT required: yes — agent completes all automatable work; user executes `S06-UAT.md` to confirm live end-to-end

## Verification

- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep -c warning` → 0
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep -c warning` → 0
- `cargo test --workspace -q 2>&1 | grep -E "^(FAILED|error\[)"` → empty
- `ls .planning/issues/closed/ | wc -l` → at least 18 files (stale issues moved)
- `grep -n "run_in" crates/smelt-core/src/git/cli.rs | grep "fn run\b"` → `run` now delegates via `run_in`
- `grep -n "strip_prefix" crates/smelt-core/src/git/cli.rs` → fix present in `branch_is_merged`
- `cat .kata/milestones/M003/slices/S06/S06-UAT.md | head -5` → file exists and is non-empty

## Observability / Diagnostics

- Runtime signals: none added (cleanup slice — no new code paths)
- Inspection surfaces: `cargo doc 2>&1 | grep warning` confirms zero doc warnings; `smelt --help` confirms all commands present
- Failure visibility: any remaining cargo doc warning is immediately visible via `RUSTDOCFLAGS="-D missing_docs"` flag in verification commands
- Redaction constraints: none

## Integration Closure

- Upstream surfaces consumed: `crates/smelt-core/src/lib.rs` (doc link fix), `crates/smelt-core/src/assay.rs` (doc link fixes), `crates/smelt-core/src/git/cli.rs` (DRY + fragility fixes)
- New wiring introduced in this slice: none — pure cleanup and documentation
- What remains before the milestone is truly usable end-to-end: human execution of `S06-UAT.md` with real credentials to confirm live GitHub PR creation and `smelt watch` resolution

## Tasks

- [x] **T01: Fix doc warnings, triage issues, and apply quick-win code fixes** `est:45m`
  - Why: Three cargo doc warnings block `RUSTDOCFLAGS="-D missing_docs"` clean pass; 32 open issues contain noise that obscures real work; two small code quality issues (DRY and fragile parsing) are quick wins with clear fixes.
  - Files: `crates/smelt-core/src/lib.rs`, `crates/smelt-core/src/assay.rs`, `crates/smelt-core/src/git/cli.rs`, `.planning/issues/closed/` (create dir + move files), `examples/job-manifest-forge.toml`
  - Do: (1) Fix `lib.rs:11` — change `[`GitHubForge`]` doc link to backtick-only `` `GitHubForge` `` to eliminate unresolved-link warning when forge feature is disabled. (2) Fix `assay.rs` — remove explicit link target `(crate::manifest::SessionDef)` from `[`SessionDef`]` reference (redundant explicit link warning); change `[`SmeltManifestSession`]` to plain `` `SmeltManifestSession` `` (private item link warning). (3) In `git/cli.rs` — refactor `run()` to delegate to `self.run_in(&self.repo_root, args).await`. (4) In `git/cli.rs` — fix `branch_is_merged()`: replace `line.trim().trim_start_matches("* ")` with a `strip_prefix("* ")` pattern. (5) Create `.planning/issues/closed/` directory; move all stale issues into it (those referencing `worktree/`, `script.rs`, `runner.rs`, `process.rs`, `cli_session.rs`, `session.rs`, `let-chain` — see T01-PLAN.md for full list); update `test-manifest-load-from-file.md` issue as already-resolved. (6) Add inline `smelt watch` and `smelt status` usage comments to `examples/job-manifest-forge.toml`.
  - Verify: `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep warning` → empty; same with `--features forge`; `cargo test -p smelt-core -q` → all pass; `ls .planning/issues/closed/ | wc -l` → ≥18
  - Done when: `cargo doc` produces 0 warnings in both feature variants; all stale issues are in `closed/`; `git/cli.rs` DRY and strip_prefix fixes are in place; example manifest has watch notes; `cargo test --workspace` still all green.

- [x] **T02: Write UAT script and perform final workspace verification** `est:30m`
  - Why: S06's demo requires a human-executable end-to-end test script (R001 live proof); the workspace must be verified clean before the milestone is declared complete.
  - Files: `.kata/milestones/M003/slices/S06/S06-UAT.md`, `crates/smelt-cli/tests/dry_run.rs`, `.kata/milestones/M003/M003-ROADMAP.md`, `.kata/STATE.md`
  - Do: (1) Write `S06-UAT.md` — a numbered step-by-step script covering: prerequisites (GITHUB_TOKEN, Docker daemon, real GitHub repo), `smelt init` → edit manifest → `smelt run --dry-run` → `smelt run` (live) → `smelt status <job>` shows PR section → `smelt watch <job>` blocks → user merges PR on GitHub → watch exits 0. Include expected output at each step and a troubleshooting section. (2) Add `test_init_then_dry_run_smoke` to `crates/smelt-cli/tests/dry_run.rs`: calls `smelt init` in a tempdir, then runs `smelt run <generated-file> --dry-run` and asserts exit 0 — proves the `smelt init` skeleton passes validation end-to-end in a real subprocess. (3) Run `cargo test --workspace -q` and confirm all green. (4) Run `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` and `--features forge` and confirm 0 warnings. (5) Mark S06 done in `M003-ROADMAP.md` checkbox. (6) Update `STATE.md` to reflect M003 complete pending human UAT.
  - Verify: `cargo test -p smelt-cli --test dry_run -q 2>&1 | grep test_init_then_dry_run` → ok; `cargo test --workspace -q` → all pass; `cat S06-UAT.md | wc -l` → ≥40 lines; roadmap S06 checkbox is `[x]`
  - Done when: UAT script is written and actionable; `test_init_then_dry_run_smoke` passes; all workspace tests still green; S06 marked complete in roadmap; STATE.md updated.

## Files Likely Touched

- `crates/smelt-core/src/lib.rs`
- `crates/smelt-core/src/assay.rs`
- `crates/smelt-core/src/git/cli.rs`
- `examples/job-manifest-forge.toml`
- `.planning/issues/closed/` (created + populated)
- `.kata/milestones/M003/slices/S06/S06-UAT.md`
- `crates/smelt-cli/tests/dry_run.rs`
- `.kata/milestones/M003/M003-ROADMAP.md`
- `.kata/STATE.md`
