---
id: S01-UAT
slice: S01
milestone: M011
uat_type: automated
human_check_required: false
---

# S01 UAT: Decompose manifest.rs and git/cli.rs

## UAT Type

**Automated** — all verification is performed by the Rust toolchain (compiler, test runner, linter, doc builder). No runtime behavior was changed; no human inspection of output quality is needed.

## Requirements Proved By This UAT

| Requirement | How Proved |
|-------------|------------|
| R060 — Large file decomposition round 2 | `find crates/smelt-core/src/manifest/ crates/smelt-core/src/git/cli/ -name '*.rs' -exec wc -l {} + \| awk '$1 > 500 {found=1} END {exit found ? 1 : 0}'` passes (max file: 337L). Both `manifest.rs` and `git/cli.rs` no longer exist as flat files. All 290 workspace tests pass unchanged, confirming all public API paths preserved. |

## Not Proven By This UAT

| What | Why Deferred |
|------|-------------|
| R061 (flaky test fix) | Out of scope for S01; `test_cli_run_invalid_manifest` timeout issue addressed in S02 |
| R062 (tracing migration) | Out of scope for S01; `eprintln!` replacement addressed in S02 |
| R063 (health endpoint) | Out of scope for S01; `GET /health` addressed in S03 |
| Runtime behavior correctness | S01 is a pure structural refactor — all behavior is the same as before. The compiler and test suite are the proof; no live runtime verification needed. |

## Verification Checklist

| # | Check | Command | Expected | Result |
|---|-------|---------|----------|--------|
| 1 | All files under 500L | `find crates/smelt-core/src/manifest/ crates/smelt-core/src/git/cli/ -name '*.rs' -exec wc -l {} + \| awk '$1 > 500 {found=1} END {exit found ? 1 : 0}'` | exit 0 | ✓ PASS |
| 2 | 290+ tests, 0 failures | `cargo test --workspace` | ≥290 passed, 0 failed | ✓ PASS (290 passed, 0 failed, 9 ignored) |
| 3 | clippy clean | `cargo clippy --workspace` | zero warnings | ✓ PASS |
| 4 | doc clean | `cargo doc --workspace --no-deps` | zero warnings | ✓ PASS |
| 5 | manifest.rs removed | `test ! -f crates/smelt-core/src/manifest.rs` | exit 0 | ✓ PASS |
| 6 | git/cli.rs removed | `test ! -f crates/smelt-core/src/git/cli.rs` | exit 0 | ✓ PASS |
| 7 | build clean | `cargo build --workspace` | exit 0 | ✓ PASS (implicit in test/clippy) |

## Notes

This slice is a contract-level proof only. No new runtime behavior was introduced. The proof is:
1. The compiler accepts all import paths that existed before — no consumer of `smelt-core::manifest` or `smelt-core::git::GitCli` needed changes
2. All 290 tests pass, including the 48 manifest unit tests and 29 git/cli unit tests now living in their new submodule paths
3. Clippy and doc checks confirm no quality regressions from the restructuring
