# Phase 23 Verification

**Status:** passed
**Score:** 32/32 must-haves verified

## Must-Have Verification

### TPROT Requirements

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| TPROT-07 | Guard daemon polls session file at configurable interval | PASS | `daemon.rs:59-60` — `tokio::time::interval(Duration::from_secs(self.config.poll_interval_secs))` in `tokio::select!` loop |
| TPROT-08 | Soft threshold triggers gentle pruning without session reload | PASS | `daemon.rs:190-226` — `handle_soft_threshold` calls `prune_session` with tier from circuit breaker; no session reload |
| TPROT-09 | Hard threshold triggers full prune + team-protect + optional session reload | PASS | `daemon.rs:229-271` — `handle_hard_threshold` forces at least `Standard` tier, saves checkpoint, calls `prune_session` |
| TPROT-10 | Token-based thresholds alongside file-size thresholds | PASS | `thresholds.rs:20-46` — `evaluate_thresholds` checks both `context_pct` and `file_size_bytes` against config; `daemon.rs:138-165` estimates token percentage |
| TPROT-11 | Reactive overflow recovery with file system watcher (kqueue on macOS, inotify on Linux) | PASS | `watcher.rs` uses `notify::RecommendedWatcher`; `Cargo.toml` specifies `notify = { version = "7", features = ["macos_kqueue"] }`; watcher events handled in `daemon.rs:89-103` via `tokio::select!` |
| TPROT-12 | Circuit breaker prevents infinite recovery loops | PASS | `circuit_breaker.rs:16-105` — sliding window, `should_trip()`, `trip()`, returns error `GuardCircuitBreakerTripped` |
| TPROT-13 | Escalating prescriptions on repeated recoveries | PASS | `circuit_breaker.rs:77-83` — `current_tier()`: 0-1 = Gentle, 2 = Standard, 3+ = Aggressive |

### Plan 01 Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | GuardConfig is optional field on Config | PASS | `assay-types/src/lib.rs:123-124` — `pub guard: Option<GuardConfig>` |
| 2 | GuardConfig has deny_unknown_fields | PASS | `assay-types/src/lib.rs:165` — `#[serde(deny_unknown_fields)]` |
| 3 | Defaults: soft 60%, hard 80%, poll 5s, max_recoveries 3, recovery_window 600s | PASS | `assay-types/src/lib.rs:203-216` — default functions return 0.6, 0.8, 5, 3, 600 |
| 4 | PID file prevents double-starts | PASS | `pid.rs:44-47` — `create_pid_file` checks `check_running` first, returns `GuardAlreadyRunning` |
| 5 | Stale PID files cleaned up automatically | PASS | `pid.rs:33-38` — `check_running` removes PID file when process is dead; test `stale_pid_is_cleaned_up` |
| 6 | Threshold evaluation returns None/Soft/Hard | PASS | `thresholds.rs:6-14` — `enum ThresholdLevel { None, Soft, Hard }` |
| 7 | Error variants GuardAlreadyRunning, GuardNotRunning, GuardCircuitBreakerTripped | PASS | `error.rs:202-220` — all three variants present |

### Plan 02 Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | CircuitBreaker tracks recovery timestamps in sliding window | PASS | `circuit_breaker.rs:22` — `recoveries: VecDeque<Instant>` with `prune_old()` |
| 2 | Trips after max_recoveries in window | PASS | `circuit_breaker.rs:53-55` — `should_trip` checks `recovery_count() >= max_recoveries`; test `max_recoveries_trips` |
| 3 | Escalation: gentle -> standard -> aggressive | PASS | `circuit_breaker.rs:77-83`; tests `escalation_gentle`, `escalation_standard`, `escalation_aggressive` |
| 4 | Resets after quiet period | PASS | `circuit_breaker.rs:89-94` — `reset_if_quiet` clears tripped when window empty; test `reset_after_quiet` |

### Plan 03 Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Event loop multiplexes polling timer, watcher events, and shutdown signals via tokio::select! | PASS | `daemon.rs:76-116` — `tokio::select!` with poll_interval, watcher.rx, sigint, sigterm |
| 2 | File system watcher uses notify crate (kqueue/inotify) | PASS | `watcher.rs:14` — `use notify::{..., RecommendedWatcher, ...}`; Cargo.toml `features = ["macos_kqueue"]` |
| 3 | Watcher events filtered to target session file only | PASS | `watcher.rs:70-81` — filters by filename match, rejects `.tmp` and `~` suffixes; test `watcher_ignores_unrelated_files` |
| 4 | Soft threshold triggers gentle prune with checkpoint | PASS | `daemon.rs:204,209-223` — `try_save_checkpoint("guard-soft")` then `prune_session` |
| 5 | Hard threshold triggers full prune with team-protect | PASS | `daemon.rs:248,253-268` — `try_save_checkpoint("guard-hard")` then `prune_session` with forced Standard+ tier |
| 6 | Ctrl+C triggers graceful shutdown with final checkpoint | PASS | `daemon.rs:105-108` — SIGINT calls `graceful_shutdown()` which saves checkpoint |
| 7 | Daemon re-evaluates thresholds after each prune | PASS | `daemon.rs:177,183` — calls `re_evaluate_after_prune()` after both soft and hard handling |
| 8 | Public API: start_guard, stop_guard, guard_status | PASS | `guard/mod.rs:16,28,47` — all three public functions exported |

### Plan 04 Must-Haves

| # | Must-Have | Status | Evidence |
|---|-----------|--------|----------|
| 1 | `assay context guard start` launches daemon | PASS | `main.rs:406` — `GuardCommand::Start`; handler calls `start_guard` |
| 2 | `assay context guard start --session <path>` overrides auto-discovery | PASS | `main.rs:408-409` — `#[arg(long)] session: Option<String>`; `main.rs:2431-2437` resolves from arg or auto-discovers |
| 3 | `assay context guard stop` sends SIGTERM | PASS | `guard/mod.rs:36` — `libc::kill(pid_i32, libc::SIGTERM)` |
| 4 | `assay context guard status` shows Running(pid) or Stopped | PASS | `main.rs:2511-2525` — prints PID or "not running" based on `guard_status()` |
| 5 | `assay context guard logs` displays log file with --level filter | PASS | `main.rs:2416-2417,2528-2539` — reads `guard.log`, filters by level rank |
| 6 | Exit code 2 on circuit breaker trip | PASS | `main.rs:2471-2473` — `GuardCircuitBreakerTripped => Ok(2)` |
| 7 | `just ready` passes | PASS | All checks passed (fmt, clippy, test, deny) |

## Quality Gate
- `just ready`: **PASS** (fmt-check ok, clippy ok, 318+ tests ok, cargo-deny ok)

## Test Coverage Summary

Guard-specific tests found:
- `circuit_breaker::tests` — 9 tests (new/trip/escalation/reset/prune)
- `thresholds::tests` — 7 tests (none/soft/hard for both pct and bytes)
- `pid::tests` — 7 tests (create/check/remove/stale/corrupt/double-start)
- `watcher::tests` — 3 tests (create/detect-modification/ignore-unrelated)
- `daemon::tests` — 1 test (construction)
- `guard::tests` — 3 tests (status/stop/guard_status)
- `config::tests` — 7 tests (validation edge cases)

Total guard-related tests: 37

## Gaps
None.
