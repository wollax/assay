# S01: cargo doc zero-warning + deny(missing_docs) on smelt-cli — UAT

**Milestone:** M009
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: This slice is purely compile-time lint enforcement — no runtime behavior changes. All verification is via compiler output and grep checks. No live services or user interaction needed.

## Preconditions

- Rust toolchain installed (rustc, cargo)
- Repository checked out with S01 changes applied

## Smoke Test

Run `cargo doc --workspace --no-deps` — should exit 0 with zero warnings printed.

## Test Cases

### 1. Zero cargo doc warnings

1. Run `cargo doc --workspace --no-deps 2>&1 | grep -c warning`
2. **Expected:** Output is `0`

### 2. deny(missing_docs) is active

1. Run `grep 'deny(missing_docs)' crates/smelt-cli/src/lib.rs`
2. **Expected:** Matches `#![deny(missing_docs)]`

### 3. Build compiles clean

1. Run `cargo build -p smelt-cli 2>&1 | grep -c warning`
2. **Expected:** Output is `0`

### 4. All tests pass

1. Run `cargo test --workspace`
2. **Expected:** 286+ tests pass, 0 failures

### 5. deny(missing_docs) catches new undocumented items

1. Add a new `pub fn undocumented_test() {}` to any module in smelt-cli (e.g. lib.rs)
2. Run `cargo build -p smelt-cli`
3. **Expected:** Build fails with `missing documentation for a function` error
4. Remove the test function

## Edge Cases

### Stale #[allow(dead_code)] check

1. Run `grep -rn '#\[allow(dead_code)\]' crates/smelt-cli/src/ crates/smelt-core/src/`
2. **Expected:** Only 2 results, both with rationale comments:
   - `crates/smelt-cli/src/serve/config.rs` — retry_backoff_secs (serde forward-compat)
   - `crates/smelt-core/src/k8s.rs` — PodState (namespace/pod_name for future use)

## Failure Signals

- Any `warning` output from `cargo doc --workspace --no-deps`
- Any `error` output from `cargo build -p smelt-cli` related to missing_docs
- Test failures in `cargo test --workspace`
- Unexpected `#[allow(dead_code)]` annotations without justification comments

## Requirements Proved By This UAT

- R040 — Zero-warning cargo doc: test case 1 proves `cargo doc` exits clean
- R042 — deny(missing_docs) on smelt-cli: test cases 2, 3, and 5 prove the lint is active and enforcing
- R043 — No stale #[allow] annotations: edge case check proves all annotations are justified

## Not Proven By This UAT

- R044 (large file decomposition) — belongs to S03
- R041, R045 (README, example docs) — belong to S02
- Pre-existing clippy warnings in smelt-core are not addressed by this slice

## Notes for Tester

- `cargo clippy --workspace -- -D warnings` will fail due to 16 pre-existing collapsible-if warnings in smelt-core. This is expected and predates this slice — verify by checking `main` branch has the same warnings.
- The test in case 5 is destructive — remember to revert the added function after testing.
