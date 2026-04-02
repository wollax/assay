---
estimated_steps: 5
estimated_files: 4
---

# T02: Write UAT script and perform final workspace verification

**Slice:** S06 — Integration Proof
**Milestone:** M003

## Description

With the codebase cleaned up (T01), this task produces the two remaining deliverables that close out M003: (1) a human-executable end-to-end UAT script that covers the full `smelt init` → `smelt run` → `smelt status` → `smelt watch` → merge pipeline against a real GitHub repo; and (2) an automated `test_init_then_dry_run_smoke` integration test in `dry_run.rs` that proves the `smelt init` skeleton passes `smelt run --dry-run` validation end-to-end as a subprocess (S04 follow-up). After writing both, the task performs the final workspace verification (tests + doc), marks S06 done in the roadmap, and updates STATE.md.

The UAT script is the agent's final deliverable for the milestone. The human executes it when ready (real Docker, real `GITHUB_TOKEN`, real GitHub repo with CI). The agent cannot execute this path — it records the expected outcomes so the user has an unambiguous pass/fail criterion at each step.

## Steps

1. **Write `S06-UAT.md`** in `.kata/milestones/M003/slices/S06/`. The script must cover:
   - **Prerequisites section**: Docker daemon running; `GITHUB_TOKEN` env var set with `pull_requests: write` scope; a GitHub repo the user controls (must have a `main` branch); Smelt binary built (`cargo build --release --bin smelt` or `cargo install --path crates/smelt-cli`).
   - **Step 1 — Init**: `smelt init` in a new temp directory → verify `job-manifest.toml` created; edit `job.repo` to point to the test repo path, `environment.image` to `alpine:3`, `[forge].repo` to `owner/repo`, `[forge].token_env = "GITHUB_TOKEN"`.
   - **Step 2 — Dry run**: `smelt run job-manifest.toml --dry-run` → expected output includes session plan, `── Forge ──` section, exits 0.
   - **Step 3 — Live run**: `smelt run job-manifest.toml` → expected: container provisions, Assay runs sessions, result branch is collected, `PR created: https://github.com/owner/repo/pull/N` printed to stderr, process exits.
   - **Step 4 — Status**: `smelt status <job-name>` → expected: `── Pull Request ──` section visible with URL, state `open`, CI status, review count.
   - **Step 5 — Watch**: `smelt watch <job-name>` → expected: polling lines `[HH:MM:SS] PR #N — state: open | CI: pending | reviews: 0` every 30s; process blocks.
   - **Step 6 — Merge (human)**: User merges the PR on GitHub. Watch should print `PR merged.` and exit 0.
   - **Troubleshooting section**: "PR already exists" error (use a new job name or delete PR); `CiStatus::Unknown` (expected on repos without CI checks configured); `smelt watch` reports "no state file" (check job name matches `manifest.job.name`); `GITHUB_TOKEN not set` error.
   - **Expected UAT result**: All 6 steps complete without error; exit codes match spec.

2. **Add `test_init_then_dry_run_smoke` to `crates/smelt-cli/tests/dry_run.rs`** — this test:
   - Creates a `tempdir`.
   - Calls the `smelt` binary (via `std::process::Command`) with subcommand `init` in the tempdir, asserts exit 0 and that `job-manifest.toml` exists.
   - Calls `smelt run job-manifest.toml --dry-run` in the same tempdir, asserts exit 0.
   - This proves the full `smelt init` → dry-run path works end-to-end without Docker. Use the same subprocess pattern as existing dry_run tests (they use `assert_cmd` or `std::process::Command` with `cargo_bin("smelt")`).

3. **Run `cargo test --workspace -q`** and confirm all tests pass including the new smoke test. Record test counts.

4. **Run `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps` and `--features forge`** — confirm 0 warnings in both variants (validation of T01 outcomes).

5. **Mark S06 complete in `M003-ROADMAP.md`** — change `- [ ] **S06: Integration Proof**` to `- [x] **S06: Integration Proof**`. Update `STATE.md`: set Active Milestone to M003 (complete pending UAT), Active Slice to none, Phase to "Awaiting human UAT", Next Action to "Execute S06-UAT.md with real GITHUB_TOKEN + GitHub repo to confirm R001 live proof."

## Must-Haves

- [ ] `cat .kata/milestones/M003/slices/S06/S06-UAT.md | wc -l` → ≥40 lines
- [ ] `S06-UAT.md` contains sections for Prerequisites, Steps 1–6 (init through merge), Troubleshooting, and Expected UAT result
- [ ] `cargo test -p smelt-cli --test dry_run -q 2>&1 | grep test_init_then_dry_run` → `test test_init_then_dry_run_smoke ... ok`
- [ ] `cargo test --workspace -q 2>&1 | grep -E "^(FAILED|error\[)"` → empty
- [ ] `grep '\[x\].*S06' .kata/milestones/M003/M003-ROADMAP.md` → shows S06 checked
- [ ] `grep "Awaiting human UAT\|Phase.*UAT\|complete pending" .kata/STATE.md` → found

## Verification

- `cargo test -p smelt-cli --test dry_run` → all tests including `test_init_then_dry_run_smoke` pass
- `cargo test --workspace -q 2>&1 | tail -10` → all suites green
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps 2>&1 | grep -c warning` → 0
- `RUSTDOCFLAGS="-D missing_docs" cargo doc -p smelt-core --no-deps --features forge 2>&1 | grep -c warning` → 0
- `head -3 .kata/milestones/M003/slices/S06/S06-UAT.md` → non-empty

## Observability Impact

- Signals added/changed: `test_init_then_dry_run_smoke` adds a regression signal for the `smelt init` → dry-run path in the automated test suite
- How a future agent inspects this: `cargo test -p smelt-cli --test dry_run` confirms the init/dry-run path is working; `cat S06-UAT.md` gives the live test script
- Failure state exposed: if `test_init_then_dry_run_smoke` fails, the failure message will include the subprocess exit code and stderr from `smelt run --dry-run`, making the error easy to localize

## Inputs

- `crates/smelt-cli/tests/dry_run.rs` — existing dry-run tests; follow their pattern for subprocess invocation
- T01 outcomes — `cargo doc` must already be clean (0 warnings) before this task's verification step
- S01–S05 summaries — authoritative on what `smelt run` and `smelt watch` should produce in the live run

## Expected Output

- `.kata/milestones/M003/slices/S06/S06-UAT.md` — new: ≥40-line human UAT script
- `crates/smelt-cli/tests/dry_run.rs` — `test_init_then_dry_run_smoke` test added
- `.kata/milestones/M003/M003-ROADMAP.md` — S06 checkbox changed to `[x]`
- `.kata/STATE.md` — updated to reflect M003 complete pending human UAT
