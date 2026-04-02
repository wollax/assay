# S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix — UAT

**Milestone:** M012
**Written:** 2026-03-27

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All verification criteria are mechanically checkable via `rg` and `cargo` commands. No Docker execution required — existing integration tests exercise the subscriber initialization. The output format change (bare-message vs full-format) is verifiable by inspection of the subscriber init code and confirmed by test pass.

## Preconditions

- Rust toolchain installed (`cargo`, `rustc`)
- `smelt-cli` built with the current codebase
- No Docker connection required (tests skip gracefully when unavailable)

## Smoke Test

Run `cargo test --workspace` and confirm 298 tests pass with 0 failures. This single command exercises the subscriber init (T01), verifies integration test assertions on stderr substrings still pass (T02), and confirms no regressions.

## Test Cases

### 1. eprintln! count verification

1. Run: `rg 'eprintln!' crates/smelt-cli/src/ --count-matches`
2. **Expected:** Exactly two results: `crates/smelt-cli/src/main.rs:1` and `crates/smelt-cli/src/serve/tui.rs:1`. No other files.

### 2. Flaky test timeout verification

1. Run: `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs`
2. **Expected:** No output (exit code 1 — no matches). The only `from_secs` on or near line 813 should be `from_secs(30)`.

### 3. Clippy clean

1. Run: `cargo clippy --workspace -- -D warnings`
2. **Expected:** `Finished` with 0 warnings. No `warning:` lines in output.

### 4. Doc build clean

1. Run: `cargo doc --workspace --no-deps`
2. **Expected:** `Finished` with 0 warnings. No `warning:` lines in output.

### 5. Full test suite

1. Run: `cargo test --workspace`
2. **Expected:** All test results show `ok. N passed; 0 failed`. Sum across all crates: 298+ tests, 0 failures.

## Edge Cases

### Default format produces bare messages

1. Build `smelt-cli`: `cargo build -p smelt-cli`
2. Run: `./target/debug/smelt run --dry-run examples/job-manifest.toml 2>&1 | head -5`
3. **Expected:** Output lines contain only the message text — no timestamps, no log levels (e.g. `INFO`), no module targets. Lines look like: `Dry-run mode: skipping container provision.`

### SMELT_LOG activates full format

1. Run: `SMELT_LOG=debug ./target/debug/smelt run --dry-run examples/job-manifest.toml 2>&1 | head -5`
2. **Expected:** Output lines include timestamps (`2026-03-27T...`), log levels (`INFO`, `DEBUG`), and module targets (`smelt_cli::commands::run`).

## Failure Signals

- `rg 'eprintln!'` returns more than 2 results — migration incomplete or new eprintln! added
- `from_secs(10)` found in docker_lifecycle.rs — timeout not updated
- `cargo clippy` reports warnings — tracing macro usage introduced a lint
- `cargo test --workspace` shows failures — message text changed broke integration test assertions
- `cargo doc` reports broken links — new tracing imports or file restructuring broke intra-doc links

## Requirements Proved By This UAT

- R061 — Proven: `from_secs(10)` absent from docker_lifecycle.rs confirms flaky timeout resolved; test passes in the suite
- R062 — Proven: exactly 2 `eprintln!` remain; integration test suite passes with stderr substring assertions intact; `SMELT_LOG=debug` surfaces full-format tracing output

## Not Proven By This UAT

- Live manual visual verification of bare-message format during a real `smelt run` with Docker — the format is correct by code inspection and integration test coverage, but a human hasn't watched it stream in a terminal session
- Performance impact of structured tracing vs raw eprintln! — no benchmarks run (not a concern at this scale)
- Behavior under very high log volume — not relevant for this codebase

## Notes for Tester

- Docker tests are skipped when Docker is unavailable — this is expected and does not indicate a failure
- The `cargo test --workspace` count may vary slightly if ignored tests are included in the total; look for `0 failed` not a specific number
- `examples/job-manifest.toml` or any valid manifest can be used for the dry-run format verification
