# S03: Large file decomposition — UAT

**Milestone:** M009
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Pure refactoring — no new runtime behavior, no new UI, no new API endpoints. All verification is structural (file sizes, test counts, compiler checks). The automated test suite is the complete verification surface.

## Preconditions

- Rust toolchain installed (`cargo`, `rustc`)
- Repository checked out at the S03 branch or main after merge

## Smoke Test

Run `cargo test --workspace` — all 286+ tests should pass with 0 failures. This confirms no regressions from the module decomposition.

## Test Cases

### 1. run.rs decomposition — size threshold

1. Run `wc -l crates/smelt-cli/src/commands/run/mod.rs`
2. **Expected:** Under 300 lines (actual: 116)

### 2. ssh.rs decomposition — size threshold

1. Run `wc -l crates/smelt-cli/src/serve/ssh/mod.rs`
2. **Expected:** Under 400 lines (actual: 111)

### 3. tests.rs decomposition — size threshold

1. Run `wc -l crates/smelt-cli/src/serve/tests/mod.rs`
2. **Expected:** Under 500 lines (actual: 88)

### 4. Full test suite — zero regressions

1. Run `cargo test --workspace`
2. **Expected:** 286+ tests pass, 0 failures

### 5. Documentation — zero warnings

1. Run `cargo doc --workspace --no-deps 2>&1 | grep -c warning`
2. **Expected:** 0

### 6. Clippy — clean workspace

1. Run `cargo clippy --workspace -- -D warnings`
2. **Expected:** Exit 0, no warnings

### 7. Child modules exist with real content

1. Verify these files exist and are non-empty:
   - `crates/smelt-cli/src/commands/run/phases.rs`
   - `crates/smelt-cli/src/commands/run/dry_run.rs`
   - `crates/smelt-cli/src/commands/run/helpers.rs`
   - `crates/smelt-cli/src/serve/ssh/client.rs`
   - `crates/smelt-cli/src/serve/ssh/operations.rs`
   - `crates/smelt-cli/src/serve/ssh/mock.rs`
   - `crates/smelt-cli/src/serve/tests/queue.rs`
   - `crates/smelt-cli/src/serve/tests/dispatch.rs`
   - `crates/smelt-cli/src/serve/tests/http.rs`
   - `crates/smelt-cli/src/serve/tests/ssh_dispatch.rs`
   - `crates/smelt-cli/src/serve/tests/config.rs`
2. **Expected:** All files exist with substantive content (not stubs)

## Edge Cases

### Old import paths still resolve

1. Run `cargo build --workspace`
2. **Expected:** Clean build — all existing imports (e.g. `crate::serve::ssh::tests::MockSshClient`) still resolve via re-exports

## Failure Signals

- Any test failure in `cargo test --workspace` indicates a regression from the decomposition
- Compiler errors indicate broken imports or visibility changes
- `cargo doc` warnings indicate missing doc comments on new public items

## Requirements Proved By This UAT

- R044 (Large file decomposition) — all three target files under thresholds, all tests pass, public API preserved

## Not Proven By This UAT

- No runtime behavior is tested — this is a pure refactoring slice with no new features
- No live execution (Docker, Compose, K8s, SSH) is exercised

## Notes for Tester

This is a structural refactoring. If `cargo test --workspace` passes and `cargo build --workspace` compiles clean, the decomposition is correct. No manual testing of runtime behavior is needed — the existing test suite covers all functionality.
