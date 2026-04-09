---
phase: 60-process-safety
verified: 2026-04-08T22:45:00Z
status: passed
score: 5/5 must-haves verified
---

# Phase 60: Process Safety Verification Report

**Phase Goal:** Fix process lifecycle and output safety issues
**Verified:** 2026-04-08T22:45:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #   | Truth                                                                 | Status     | Evidence                                                                                    |
| --- | --------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------- |
| 1   | `kill_agent_subprocess` uses `killpg` for process group termination   | ✓ VERIFIED | `libc::killpg` for both SIGTERM and SIGKILL in `pipeline_checkpoint.rs` lines 206, 258      |
| 2   | Inline checkpoint-abort path also uses `killpg`                       | ✓ VERIFIED | `libc::killpg(agent_handle.pid, libc::SIGTERM)` in `pipeline.rs` line 950                   |
| 3   | Auto-promote treats "already at target status" as info, not warn/error | ✓ VERIFIED | `info!` macro at `pipeline.rs` line 1250–1253; genuine failures use `warn!`                 |
| 4   | Relay thread panics are logged instead of silently swallowed          | ✓ VERIFIED | `relay.join()` Err arm extracts payload string and calls `tracing::error!` at line 973      |
| 5   | TUI strips ANSI/control characters from TextDelta/TextBlock content   | ✓ VERIFIED | `sanitize()` in `app.rs` line 2306 called via `push_line` on every TextDelta flush          |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact                                            | Expected                                          | Status     | Details                                                                         |
| --------------------------------------------------- | ------------------------------------------------- | ---------- | ------------------------------------------------------------------------------- |
| `crates/assay-tui/src/app.rs`                       | sanitize function with ANSI_RE regex               | ✓ VERIFIED | `pub(crate) fn sanitize` at line 2306; `OnceLock<regex_lite::Regex>` at 2307   |
| `crates/assay-tui/Cargo.toml`                       | regex-lite workspace dep                          | ✓ VERIFIED | `regex-lite.workspace = true` present                                           |
| `crates/assay-core/src/pipeline_checkpoint.rs`      | `kill_agent_subprocess` with `killpg`             | ✓ VERIFIED | `libc::killpg` for SIGTERM (line 206) and SIGKILL (line 258)                   |
| `crates/assay-core/src/pipeline.rs` (inline abort)  | Inline checkpoint-abort using `killpg`            | ✓ VERIFIED | `libc::killpg(agent_handle.pid as libc::pid_t, libc::SIGTERM)` at line 950     |
| `crates/assay-core/src/pipeline.rs` (auto-promote)  | TOCTOU-safe auto-promote with `info!` on conflict | ✓ VERIFIED | Re-reads on-disk spec; uses `info!` when already Verified, `warn!` otherwise   |
| `crates/assay-core/src/pipeline.rs` (relay join)    | Relay panic logged with payload                   | ✓ VERIFIED | `downcast_ref` extracts panic string; `tracing::error!(panic = %msg, ...)` logged |
| `crates/assay-core/src/pipeline.rs` (StreamingAgentHandle) | `stderr_buffer` Arc<Mutex<String>> field  | ✓ VERIFIED | Field declared at line 431; populated by concurrent reader thread               |
| `crates/assay-core/src/pipeline.rs` (stderr Stdio)  | `Stdio::piped()` for stderr in streaming launch   | ✓ VERIFIED | `.stderr(Stdio::piped())` at line 479                                           |
| `crates/assay-core/src/pipeline.rs` (stderr thread) | Concurrent stderr reader thread                   | ✓ VERIFIED | `std::thread::spawn` on stderr pipe at line 524; writes to `stderr_buffer`     |
| `crates/assay-core/src/pipeline.rs` (crash error)   | Crash PipelineError includes captured stderr      | ✓ VERIFIED | `stderr_section` appended to `recovery` field at line 1035                     |

### Key Link Verification

| From                                      | To                                  | Via                             | Status     | Details                                                                      |
| ----------------------------------------- | ----------------------------------- | ------------------------------- | ---------- | ---------------------------------------------------------------------------- |
| `handle_agent_event` → `push_line`        | `sanitize`                          | Direct call in closure          | ✓ WIRED    | `lines.push(sanitize(line))` at app.rs line 317                              |
| `launch_agent_streaming` stderr pipe      | `stderr_buffer`                     | Concurrent reader thread        | ✓ WIRED    | Thread at line 524 populates Arc<Mutex<String>>; handle exposes it           |
| Pipeline crash path                       | `stderr_buffer` content             | `agent_handle.stderr_buffer.lock()` | ✓ WIRED | Content included in `recovery` field of returned `PipelineError`            |
| `kill_agent_subprocess` SIGTERM → SIGKILL | process group                       | `libc::killpg`                  | ✓ WIRED    | Both signals sent via `killpg` (not `kill`); test at checkpoint.rs line 549  |
| Auto-promote TOCTOU guard                 | "already Verified" → `info!` branch | Re-read spec.toml on Err        | ✓ WIRED    | `load_feature_spec` re-read inside Err arm; status comparison triggers `info!` |
| Relay thread panic                        | `tracing::error!`                   | `join()` Err arm + downcast     | ✓ WIRED    | Payload extracted via `downcast_ref::<&str>` and `downcast_ref::<String>`   |

### Requirements Coverage

| Requirement | Source Plan | Description                                                          | Status     | Evidence                                                         |
| ----------- | ----------- | -------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------- |
| SAFE-01     | 60-02       | `kill_agent_subprocess` uses `killpg` for process group termination  | ✓ SATISFIED | `libc::killpg` for SIGTERM and SIGKILL in `pipeline_checkpoint.rs` |
| SAFE-02     | 60-03       | Auto-promote TOCTOU race handled between status check and promotion  | ✓ SATISFIED | Re-read on-disk spec in Err arm; `info!` when already Verified   |
| SAFE-03     | 60-03       | Pipeline crash error messages include stderr content                 | ✓ SATISFIED | `captured_stderr` appended to `recovery` in `PipelineError`     |
| SAFE-04     | 60-02       | Relay thread panics logged instead of silently swallowed             | ✓ SATISFIED | `relay.join()` Err arm logs `tracing::error!(panic = %msg, ...)`|
| SAFE-05     | 60-01       | TUI strips ANSI/control characters from TextDelta/TextBlock content  | ✓ SATISFIED | `sanitize()` in `app.rs` using `regex_lite`; 8 unit tests cover all cases |

### Anti-Patterns Found

No anti-patterns found. No TODOs, stubs, empty implementations, or placeholder returns in modified files.

### Human Verification Required

None. All five safety requirements are fully verifiable via static analysis and test presence.

### Gaps Summary

No gaps. All five SAFE-01 through SAFE-05 requirements are implemented, substantive, and wired:

- SAFE-01: Both `kill_agent_subprocess` in `pipeline_checkpoint.rs` and the inline checkpoint-abort in `pipeline.rs` use `libc::killpg` (process group) rather than `libc::kill` (single PID) for SIGTERM and SIGKILL. Test spawns child with `process_group(0)`.
- SAFE-02: The auto-promote Err arm re-reads `spec.toml` from disk to distinguish "already Verified by concurrent process" (logged at `info!`) from genuine I/O failure (logged at `warn!`). A unit test validates the guard logic.
- SAFE-03: After the relay thread joins, `stderr_buffer` is locked and its content appended to the `recovery` field of `PipelineError` when the agent crashes.
- SAFE-04: `relay.join()` matches the `Err` arm, extracts the panic payload via `downcast_ref`, and emits `tracing::error!(panic = %msg, "relay thread panicked")` instead of calling `unwrap_or(-1)` silently.
- SAFE-05: `sanitize()` uses a `OnceLock`-cached `regex_lite::Regex` to strip full CSI sequences and single-char Fe sequences; remaining control chars are replaced with U+FFFD; plain text and tabs pass through. Eight unit tests cover all cases. Called via `push_line` on every line pushed from TextDelta events.

---

_Verified: 2026-04-08T22:45:00Z_
_Verifier: Claude (kata-verifier)_
