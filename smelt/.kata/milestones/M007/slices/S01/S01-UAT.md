# S01: Serialize queue types + migrate Instant to SystemTime — UAT

**Milestone:** M007
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All verification is contractual — the slice's correctness is fully proved by `cargo test -p smelt-cli` passing all 46 tests (including 19 serve tests). No live runtime, no HTTP server startup, and no human UI interaction are required for the goals of this slice. The serde derives and timing helpers are passive type-level changes that either compile and test-pass or they don't.

## Preconditions

- Rust toolchain installed (`cargo --version`)
- Working directory: `/Users/wollax/Git/personal/smelt`

## Smoke Test

```bash
cargo test -p smelt-cli 2>&1 | grep "^test result"
```

Expected: All lines show `ok. N passed; 0 failed`.

## Test Cases

### 1. All 46 smelt-cli tests pass

```bash
cargo test -p smelt-cli 2>&1 | grep -E "^test result"
```

**Expected:** Each test result line shows `0 failed`.

### 2. Exactly 19 serve tests are registered

```bash
cargo test -p smelt-cli -- --list 2>&1 | grep -c "serve::tests"
```

**Expected:** Output is `19`.

### 3. No Instant-related compile warnings

```bash
cargo check -p smelt-cli 2>&1 | grep -i "instant\|unused import"
```

**Expected:** No output (zero warnings about `Instant` or unused imports in serve files).

### 4. QueuedJob carries serde derives

```bash
grep -A2 "struct QueuedJob" crates/smelt-cli/src/serve/types.rs
```

**Expected:** Output includes `#[derive(` line with both `Serialize` and `Deserialize`.

### 5. Timing fields are u64, not Instant

```bash
grep "queued_at\|started_at" crates/smelt-cli/src/serve/types.rs
```

**Expected:** `queued_at: u64` and `started_at: Option<u64>` — no `Instant` reference.

### 6. Helper functions present in types.rs

```bash
grep "pub fn now_epoch\|pub fn elapsed_secs_since" crates/smelt-cli/src/serve/types.rs
```

**Expected:** Both function signatures appear.

## Edge Cases

### Clock skew guard

```bash
grep "max(0.0)\|\.max(" crates/smelt-cli/src/serve/types.rs
```

**Expected:** `.max(0.0)` guard is present in `elapsed_secs_since()`.

### No std::time::Instant in callsite files

```bash
grep "std::time::Instant\|use std::time::{.*Instant" \
  crates/smelt-cli/src/serve/http_api.rs \
  crates/smelt-cli/src/serve/tui.rs \
  crates/smelt-cli/src/serve/tests.rs
```

**Expected:** No output.

## Failure Signals

- `cargo test -p smelt-cli` shows any `FAILED` lines → a test regression was introduced
- `cargo check -p smelt-cli` shows warnings about unused `Instant` imports → a callsite was missed
- `grep "Instant" types.rs queue.rs dispatch.rs` returns output → `Instant` was not fully removed from core serve files
- Test count from `--list | grep -c serve::tests` is not 19 → tests were accidentally deleted

## Requirements Proved By This UAT

- R028 (partial) — `QueuedJob` and all queue types are now `Serialize + Deserialize` with `u64` timing fields; the blocking prerequisite for TOML state persistence is removed. Full proof of R028 (crash recovery end-to-end) requires S02 + S03.

## Not Proven By This UAT

- That `queue_dir/.smelt-queue-state.toml` is actually written (S02).
- That `smelt serve` recovers jobs after restart (S03).
- That HTTP `elapsed_secs` values are accurate over longer time spans (this would require a live runtime test with real wall-clock elapsed time).
- Live TUI rendering with real jobs — `test_tui_render_no_panic` only confirms no panic, not display accuracy.

## Notes for Tester

All checks in this UAT run in under 60 seconds. They are deterministic and require no external services, no Docker daemon, and no network access. If any check fails, run `cargo check -p smelt-cli` first to get the full compiler error context before investigating test output.
