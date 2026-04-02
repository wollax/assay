# S02: Teardown error handling + SSH DRY cleanup

**Goal:** Container teardown failures produce visible warnings instead of silent `let _ =` discards; error chains are preserved via `.context()` instead of `anyhow!("{e}")`; SSH argument builders share a common helper eliminating ~90% duplicated logic.
**Demo:** `cargo test --workspace` passes with 0 failures; `rg 'let _ =' phases.rs` shows zero silent teardown discards (only monitor phase-transition `let _ =` remains where appropriate); `rg 'anyhow!.*\{e\}' phases.rs` returns zero hits; `build_ssh_args` and `build_scp_args` both delegate to a shared `build_common_ssh_args`.

## Must-Haves

- All 6 duplicated teardown blocks in `phases.rs` (lines ~117–207) replaced with a `warn_teardown()` helper that logs warnings on failure instead of silently discarding
- `let _ = provider.teardown(...)` replaced with logged warning on error
- `let _ = monitor.set_phase(...)` on teardown paths replaced with logged warning on error
- `let _ = monitor.cleanup()` on teardown paths replaced with logged warning on error
- All 5× `monitor.write().map_err(|e| anyhow::anyhow!("{e}"))` replaced with `.context("...")`
- `build_common_ssh_args()` extracted from `build_ssh_args`/`build_scp_args` with a `port_flag` parameter
- `build_ssh_args` and `build_scp_args` delegate to `build_common_ssh_args` — no duplicated flag logic
- All existing tests pass unchanged (286+ tests, 0 failures)
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings

## Proof Level

- This slice proves: contract (pure refactoring — behavior changes only in error paths, verified by existing tests + code inspection)
- Real runtime required: no (no Docker/K8s/SSH needed — all changes are internal code quality)
- Human/UAT required: no (mechanical refactoring verified by test suite)

## Verification

- `cargo test --workspace` passes with 0 failures, 286+ tests
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` zero warnings
- `rg 'anyhow!.*\{e\}' crates/smelt-cli/src/commands/run/phases.rs` returns zero hits
- `rg 'let _ = provider\.teardown' crates/smelt-cli/src/commands/run/phases.rs` returns zero hits
- `build_ssh_args` and `build_scp_args` bodies are ≤5 lines each (delegation only)
- All existing SSH arg tests in `mock.rs` pass without modification

## Observability / Diagnostics

- Runtime signals: `eprintln!` warnings on teardown failures (provider.teardown, monitor.set_phase, monitor.cleanup) — replacing silent discards
- Inspection surfaces: stderr output during `smelt run` failure paths — previously invisible teardown errors are now visible
- Failure visibility: error chain preserved on `monitor.write()` failures — `.context()` retains the original error type and message instead of stringifying
- Redaction constraints: none (teardown and SSH args contain no secrets)

## Integration Closure

- Upstream surfaces consumed: `phases.rs` (run lifecycle), `client.rs` (SSH subprocess), `mock.rs` (SSH arg tests)
- New wiring introduced in this slice: none (pure refactoring — no new call sites or interfaces)
- What remains before the milestone is truly usable end-to-end: S03 documents `[auth]` in examples/server.toml and README.md

## Tasks

- [x] **T01: Extract warn_teardown helper and replace silent let _ = in phases.rs** `est:30m`
  - Why: 6 identical teardown blocks silently discard errors — the core R052 fix
  - Files: `crates/smelt-cli/src/commands/run/phases.rs`
  - Do: Extract `warn_teardown()` async helper that calls provider.teardown + monitor.set_phase(TearingDown) + monitor.cleanup with eprintln! warnings on each failure; replace all 6 duplicated teardown blocks with calls to this helper; replace all 5× `anyhow!("{e}")` with `.context("failed to write monitor state")` or similar; keep `let _ =` on non-teardown monitor.set_phase calls (GatesFailed, Complete, Failed, etc.) since those are best-effort status updates, not teardown
  - Verify: `cargo test --workspace` all pass; `rg 'let _ = provider\.teardown' phases.rs` returns 0; `rg 'anyhow!.*\{e\}' phases.rs` returns 0; `cargo check`
  - Done when: zero silent teardown discards; error chains preserved; all tests pass

- [x] **T02: Extract build_common_ssh_args and deduplicate SSH arg builders** `est:20m`
  - Why: `build_ssh_args` and `build_scp_args` share ~90% identical logic — R053 fix
  - Files: `crates/smelt-cli/src/serve/ssh/client.rs`
  - Do: Extract `build_common_ssh_args(worker, timeout_secs, port_flag, tool_name, extra_args) -> Vec<String>` containing all shared logic (BatchMode, StrictHostKeyChecking, ConnectTimeout, port handling, key_env resolution with tracing); `build_ssh_args` calls it with `"-p"` and `"SSH"`; `build_scp_args` calls it with `"-P"` and `"SCP"`; each delegating function should be ≤5 lines
  - Verify: `cargo test --workspace` all pass; existing SSH arg tests in `mock.rs` pass without modification; `cargo clippy --workspace` clean; `cargo doc --workspace --no-deps` zero warnings
  - Done when: `build_ssh_args`/`build_scp_args` are pure delegation; common helper is documented; all tests pass

## Files Likely Touched

- `crates/smelt-cli/src/commands/run/phases.rs`
- `crates/smelt-cli/src/serve/ssh/client.rs`
