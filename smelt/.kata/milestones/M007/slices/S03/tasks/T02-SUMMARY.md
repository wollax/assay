---
id: T02
parent: S03
milestone: M007
provides:
  - "`commands/serve.rs` calls `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)` — crash recovery active on every real startup"
  - "`examples/server.toml` annotated with persistence and restart-recovery behavior comment"
key_files:
  - crates/smelt-cli/src/commands/serve.rs
  - examples/server.toml
key_decisions:
  - "No new import needed — `ServerState` was already imported via `use crate::serve::{...}`; `load_or_new` is a method on the same type"
patterns_established:
  - "Persistence wiring pattern: `load_or_new(config.queue_dir.clone(), config.max_concurrent)` replaces bare `new()` — one-line change activates full crash-recovery loop"
observability_surfaces:
  - "tracing::info!(\"load_or_new: loaded N jobs from {queue_dir}, M remapped to Queued\") fires on every `smelt serve` startup — grep `.smelt/serve.log` or stderr to confirm recovery ran"
  - "`grep \"load_or_new\" crates/smelt-cli/src/commands/serve.rs` — confirms wiring is present"
duration: 5min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Wire `load_or_new` into `serve.rs` + update `examples/server.toml`

**`serve.rs` now calls `ServerState::load_or_new` on startup — crash-recovery is live; `examples/server.toml` explains persistence behavior**

## What Happened

Single-line change in `commands/serve.rs`: replaced `ServerState::new(config.max_concurrent)` with `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)`. No import changes required — `ServerState` was already in scope. `config.queue_dir` was already available two lines earlier (used in `create_dir_all`).

Added a four-line comment block above `queue_dir` in `examples/server.toml` explaining the automatic persistence loop, state file location, and restart-recovery behavior for operators.

## Verification

- `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs` → prints the wiring line ✓
- `grep "ServerState::new" crates/smelt-cli/src/commands/serve.rs` (excluding `load_or_new`/`new_with_persistence` lines) → no output (exit 1) — confirms `new()` no longer called ✓
- `cargo check -p smelt-cli` → exits 0, zero warnings ✓
- `cargo test -p smelt-cli` → 52 passed, 0 failed ✓ (includes `test_load_or_new_restart_recovery` and `test_load_or_new_missing_file` from T01)

## Diagnostics

After any `smelt serve` startup, grep `.smelt/serve.log` or stderr for `"load_or_new: loaded"` to confirm whether crash-recovery ran and how many jobs were recovered. If `n=0` on a restart where jobs were expected, verify `queue_dir` in `server.toml` matches the path used in the previous run.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/commands/serve.rs` — replaced `ServerState::new(config.max_concurrent)` with `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)`
- `examples/server.toml` — added persistence/restart-recovery comment block above `queue_dir`
