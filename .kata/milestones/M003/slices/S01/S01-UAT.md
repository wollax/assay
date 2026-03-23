# S01: GitHub Forge Client — UAT

**Milestone:** M003
**Written:** 2026-03-21

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S01 is a contract-verification slice. All proof is in unit tests against a mock HTTP server. There is no CLI surface, no Docker, and no real GitHub token required. The slice's defined proof level explicitly states "Real runtime required: no" and "Human/UAT required: no." A human tester can verify the same outcomes the CI system does by running the verification commands.

## Preconditions

- Rust toolchain installed (`cargo` available)
- In the `smelt` repository root
- No `GITHUB_TOKEN` required — all tests use wiremock mock servers

## Smoke Test

```bash
cargo test -p smelt-core --features forge -- forge 2>&1 | tail -10
```

Expected: `test result: ok. 6 passed; 0 failed`

## Test Cases

### 1. All 6 forge unit tests pass

```bash
cargo test -p smelt-core --features forge 2>&1 | tail -5
```

**Expected:** `test result: ok. 118 passed; 0 failed; 0 ignored`

### 2. No-forge build is clean (zero octocrab dependency)

```bash
cargo build -p smelt-core 2>&1 | grep "^error"
# (no output expected)

cargo tree -p smelt-core | grep octocrab
# (no output expected — must print nothing)
```

**Expected:** build succeeds with no errors; `grep octocrab` produces zero lines.

### 3. Forge feature adds octocrab

```bash
cargo tree -p smelt-core --features forge | grep octocrab
```

**Expected:** output includes `octocrab v0.49.x`

### 4. Workspace builds cleanly

```bash
cargo build --workspace 2>&1 | grep "^error" | wc -l
```

**Expected:** `0`

## Edge Cases

### create_pr with invalid repo format

The `parse_repo()` helper returns `SmeltError::Forge { operation: "create_pr", message: "invalid repo format" }` when the repo string has no `/`. This is covered by the unit test scaffold; can be observed by calling:

```bash
cargo test -p smelt-core --features forge test_create_pr -- --nocapture 2>&1 | head -30
```

**Expected:** all three create_pr tests pass; no panics.

### poll_pr_status CI status endpoint failure → CiStatus::Unknown

The non-fatal CI fallback is covered by the unit tests using a mock that returns a valid status. The `Unknown` fallback path is exercised when the mock is not mounted. Observable via `--nocapture`:

```bash
cargo test -p smelt-core --features forge test_poll_pr_status -- --nocapture 2>&1 | head -40
```

**Expected:** all three poll tests pass; wiremock request logs show both PR and status endpoint matches.

## Failure Signals

- Any `FAILED` line in `cargo test -p smelt-core --features forge` output
- `octocrab` appearing in `cargo tree -p smelt-core` (no-feature run)
- `error[E...]` lines in `cargo build -p smelt-core` or `cargo build --workspace`
- Zero tests running (indicates feature flag wiring broke)

## Requirements Proved By This UAT

- R001 (partial) — `GitHubForge::create_pr()` exists and is tested at the contract level; real PR creation via `execute_run()` Phase 9 requires S02
- R005 (partial) — `ForgeClient` trait and `GitHubForge` are the first forge API elements; full library API surface requires S05

## Not Proven By This UAT

- Real GitHub PR creation with a live `GITHUB_TOKEN` — requires S02 integration
- `smelt run manifest.toml` with `[forge]` block — requires S02
- `smelt status` PR section — requires S03
- `smelt watch` polling loop — requires S03
- Rate limit / ETag conditional request behavior — deferred to S03
- `review_count` accuracy (currently inline diff comments, not formal reviews) — deferred to S03 evaluation
- External crate embedding `smelt-core` as path dependency — requires S05

## Notes for Tester

This slice has no interactive surface — all verification is `cargo test` and `cargo tree`. The tests are deterministic (wiremock binds to a random localhost port; no network calls escape to GitHub). Running the smoke test is sufficient to confirm the slice is healthy. The "not proven" items above are intentional deferrals to later slices, not gaps.
