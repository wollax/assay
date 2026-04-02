---
id: T02
parent: S06
milestone: M003
provides:
  - S06-UAT.md — 190-line human-executable end-to-end test script for smelt init → run → status → watch → PR merge pipeline
  - test_init_then_dry_run_smoke — subprocess integration test proving smelt init skeleton passes dry-run validation
  - S06 marked done in M003-ROADMAP.md — all six M003 slices complete
  - STATE.md updated to Phase: Awaiting human UAT
key_files:
  - .kata/milestones/M003/slices/S06/S06-UAT.md
  - crates/smelt-cli/tests/dry_run.rs
  - .kata/milestones/M003/M003-ROADMAP.md
  - .kata/STATE.md
key_decisions:
  - No new architectural decisions; task was documentation + verification
patterns_established:
  - "Init→dry-run subprocess test pattern: use assert_cmd::Command::cargo_bin in a tempdir — tests the full init output and dry-run path without Docker"
observability_surfaces:
  - "cargo test -p smelt-cli --test dry_run confirms init/dry-run path is working (13 tests including new smoke)"
  - "cat .kata/milestones/M003/slices/S06/S06-UAT.md — human-executable live test script with pass/fail criteria per step"
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T02: Write UAT script and perform final workspace verification

**190-line UAT script and test_init_then_dry_run_smoke integration test written; all workspace tests green; 0 cargo doc warnings; M003 S06 complete pending human execution of UAT.**

## What Happened

**S06-UAT.md** written at `.kata/milestones/M003/slices/S06/S06-UAT.md` (190 lines). The script covers:
- Prerequisites section: Docker, GITHUB_TOKEN, repo with `main`, built binary
- Step 1: `smelt init` in temp dir → verify `job-manifest.toml` created, then edit for test repo + forge config
- Step 2: `smelt run --dry-run` → verify Execution Plan + Forge section printed, exits 0
- Step 3: Live run → verify `PR created: https://github.com/…` in stderr, exits 0
- Step 4: `smelt status <job>` → verify PR section with URL, state: open, CI status, review count
- Step 5: `smelt watch <job>` → verify polling lines every 30s, process blocks
- Step 6: User merges PR → watch prints `PR merged.` and exits 0
- Troubleshooting table: PR already exists, CiStatus::Unknown, "no state file", GITHUB_TOKEN not set, Docker not running
- Expected UAT Result table: 6-row pass/fail matrix

**test_init_then_dry_run_smoke** added to `crates/smelt-cli/tests/dry_run.rs`. The test:
1. Creates a `tempfile::TempDir`
2. Runs `smelt init` as subprocess in the tempdir — asserts exit 0 and `"Created job-manifest.toml"` in stdout
3. Asserts `job-manifest.toml` exists
4. Runs `smelt run job-manifest.toml --dry-run` — asserts exit 0 and `═══ Execution Plan ═══` + `my-job` in stdout

Uses `assert_cmd::Command::cargo_bin("smelt")` — same pattern as all other tests in the file.

**S06 checkbox** in M003-ROADMAP.md changed `[ ]` → `[x]`.

**STATE.md** updated: Active Slice → none, Phase → "Awaiting human UAT", Next Action → execute S06-UAT.md with real credentials.

## Verification

- `cargo test -p smelt-cli --test dry_run` → 13 tests pass (12 existing + new smoke), 0 failed
- `cargo test --workspace -q` → all suites green, 0 failures, 0 `FAILED` or `error[` lines
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep -c warning` → 0
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep -c warning` → 0
- `cat .kata/milestones/M003/slices/S06/S06-UAT.md | wc -l` → 190 (≥40 required)
- `grep '\[x\].*S06' .kata/milestones/M003/M003-ROADMAP.md` → found
- `grep "Awaiting human UAT" .kata/STATE.md` → found

## Diagnostics

- `cargo test -p smelt-cli --test dry_run` — run to confirm init/dry-run path; test names are self-documenting
- `cat .kata/milestones/M003/slices/S06/S06-UAT.md` — the human UAT script with explicit pass/fail criteria at each step
- If `test_init_then_dry_run_smoke` fails: the assert_cmd failure message includes subprocess exit code and full stdout/stderr from `smelt run --dry-run`, making the failure easy to localize

## Deviations

None.

## Known Issues

None. All workspace tests pass; doc warnings zero; R001 live proof awaits human execution of S06-UAT.md.

## Files Created/Modified

- `.kata/milestones/M003/slices/S06/S06-UAT.md` — new: 190-line human UAT script for full live pipeline
- `crates/smelt-cli/tests/dry_run.rs` — added `test_init_then_dry_run_smoke` test
- `.kata/milestones/M003/M003-ROADMAP.md` — S06 checkbox changed to `[x]`
- `.kata/STATE.md` — updated to Phase: Awaiting human UAT
- `.kata/milestones/M003/slices/S06/S06-PLAN.md` — T02 checkbox changed to `[x]`
