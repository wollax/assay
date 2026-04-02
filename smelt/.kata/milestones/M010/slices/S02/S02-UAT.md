# S02: Teardown error handling + SSH DRY cleanup — UAT

**Milestone:** M010
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Pure refactoring slice — no new runtime behavior, no new UI, no new endpoints. All changes are internal code structure verified by the existing test suite and static code inspection.

## Preconditions

- Rust toolchain installed
- Repository cloned and on the S02 branch (or main after merge)

## Smoke Test

Run `cargo test --workspace` — all 155+ tests pass with 0 failures.

## Test Cases

### 1. No silent teardown discards in phases.rs

1. Run: `rg 'let _ = provider\.teardown' crates/smelt-cli/src/commands/run/phases.rs`
2. **Expected:** Zero matches — all teardown calls go through `warn_teardown()` helper

### 2. No lossy error conversions in phases.rs

1. Run: `rg 'anyhow!.*\{e\}' crates/smelt-cli/src/commands/run/phases.rs`
2. **Expected:** Zero matches — all error conversions use `.context()`

### 3. SSH arg builders are pure delegation

1. Run: `rg -A5 'pub fn build_ssh_args' crates/smelt-cli/src/serve/ssh/client.rs`
2. Run: `rg -A5 'pub fn build_scp_args' crates/smelt-cli/src/serve/ssh/client.rs`
3. **Expected:** Each function body is a single line calling `Self::build_common_ssh_args(...)`

### 4. Common SSH helper exists and is documented

1. Run: `rg -B3 'fn build_common_ssh_args' crates/smelt-cli/src/serve/ssh/client.rs`
2. **Expected:** Function exists with a `///` doc comment

### 5. Full workspace verification

1. Run: `cargo clippy --workspace`
2. Run: `cargo doc --workspace --no-deps`
3. **Expected:** Both exit cleanly with zero warnings

## Edge Cases

### warn_teardown helper is called from all early-return error paths

1. Run: `rg 'warn_teardown' crates/smelt-cli/src/commands/run/phases.rs`
2. **Expected:** 6 callsites + 1 function definition (7 total matches)

### Non-teardown let _ = preserved where appropriate

1. Run: `rg 'let _ =' crates/smelt-cli/src/commands/run/phases.rs`
2. **Expected:** Only the outcome match block (GatesFailed/Complete/Failed/Timeout/Cancelled phase transitions) — these are best-effort status updates, not teardown error paths

## Failure Signals

- Any `cargo test` failure
- `cargo clippy` or `cargo doc` warnings
- `rg 'let _ = provider\.teardown'` returning matches
- `rg 'anyhow!.*\{e\}'` returning matches in phases.rs

## Requirements Proved By This UAT

- R052 — Teardown error visibility: all silent `let _ =` replaced with logged warnings; error chains preserved via `.context()`
- R053 — SSH argument builder DRY cleanup: common helper extracted; both builders are pure delegation

## Not Proven By This UAT

- Runtime observation of teardown warning messages during an actual `smelt run` failure — would require triggering a real Docker teardown error (impractical to simulate reliably)
- SSH subprocess behavior — only arg construction is tested, not actual SSH/SCP execution

## Notes for Tester

This is a mechanical refactoring slice. If `cargo test --workspace` passes and the `rg` checks show zero hits, the slice is correct. No runtime environment (Docker, K8s, SSH) is needed.
