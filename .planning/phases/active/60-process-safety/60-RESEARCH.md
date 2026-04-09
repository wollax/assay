# Phase 60: Process Safety - Research

**Researched:** 2026-04-08
**Domain:** Rust process lifecycle, Unix signals, TOCTOU races, thread panic handling, ANSI terminal safety
**Confidence:** HIGH

## Summary

All five requirements in this phase are precisely-scoped bug fixes in existing Assay code. No new
architectural decisions are needed — each fix has a clear reference implementation or a small set of
well-understood options. The codebase already contains the correct patterns (process-group kill in
`gate/mod.rs`, stderr capture in `pr.rs`, atomic file write via tempfile-rename); the fixes are about
applying those patterns to the code paths that currently lack them.

The trickiest area is SAFE-05 (ANSI stripping). The existing `sanitize` function in `app.rs` replaces
the ESC byte (0x1B) with U+FFFD but does NOT strip the rest of the escape sequence (`[31m` etc.), so
CSI escape sequences partially leak through. A proper fix must consume the full sequence, not just the
initiating byte. `regex-lite` is already a workspace dependency (used in `assay-core`) and can be
added to `assay-tui` without adding a new workspace dep.

**Primary recommendation:** Fix each issue in isolation, in a single wave, leveraging existing
reference patterns already in the codebase. No new crates need to be added to the workspace unless
the planner prefers the `strip-ansi-escapes` crate for SAFE-05.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- SAFE-01: `kill_agent_subprocess` in `pipeline_checkpoint.rs:191` must use `killpg` instead of
  single-process `child.kill()`. Reference: `gate/mod.rs:794` — same pattern.
- SAFE-02: `pipeline.rs:1130-1191` TOCTOU race must be made atomic (CAS, lock, or filesystem-level
  atomic rename).
- SAFE-03: Pipeline crash error messages must include stderr content. Stderr is currently captured
  but not threaded into crash error paths.
- SAFE-04: `relay.join().expect("thread panicked")` sites must catch panics and log instead of
  crashing the host.
- SAFE-05: `app.rs:342` TextDelta text must be stripped of ANSI escape sequences and control
  characters before rendering.

### Claude's Discretion

- ANSI stripping approach (regex, byte scan, or `strip-ansi-escapes` crate)
- Specific atomic mechanism for TOCTOU fix (filesystem rename vs in-memory lock)
- Whether to add `pre_exec` for pgid in `kill_agent_subprocess` or restructure to share gate's existing pattern

### Deferred Ideas (OUT OF SCOPE)

None — all requirements are clear-cut bug fixes.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SAFE-01 | `kill_agent_subprocess` uses `killpg` for process group termination | Reference pattern at `gate/mod.rs:792-794`; libc already a unix dep in assay-core |
| SAFE-02 | Auto-promote path handles TOCTOU race between status check and promotion | `promote_spec` already atomic via tempfile-rename; gap is at call site in `pipeline.rs:1141-1145` |
| SAFE-03 | Pipeline crash error messages include stderr content | Stderr piped to `Stdio::inherit()` in streaming launch — needs piped capture + buffer in relay thread |
| SAFE-04 | Relay thread panics logged instead of silently swallowed | Production site at pipeline.rs:924 uses `unwrap_or(-1)` (silent swallow); test sites use `.expect()`; fix via `match join_result` + tracing::error! |
| SAFE-05 | TUI strips ANSI/control characters from TextDelta/TextBlock | Existing sanitize() replaces ESC byte only; full ANSI sequence (ESC + CSI payload) must be consumed |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| libc | 0.2 | `killpg`, `kill` POSIX signal APIs | Already workspace dep; already used in gate/mod.rs and pipeline.rs |
| tempfile | 3 | Atomic file write via NamedTempFile + persist | Already used in promote.rs for atomic spec write |
| regex-lite | 0.1 | Lightweight regex for ANSI sequence stripping | Already workspace dep in assay-core; no new crate needed |
| std::thread::JoinHandle | stdlib | `.join()` returns `Result<T, Box<dyn Any>>` | Panic payload captured as Err; no crate needed |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| strip-ansi-escapes | 0.2.1 | Dedicated ANSI stripper crate | Only if planner prefers zero-regex approach for SAFE-05 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| regex-lite for ANSI | Manual state machine | State machine has zero allocations, handles partial sequences; regex is simpler to read |
| regex-lite for ANSI | strip-ansi-escapes crate | New workspace dep, byte-stream oriented (not str), adds build complexity |
| Filesystem rename for TOCTOU | In-memory Mutex | Mutex only works within a single process; rename-based is inter-process safe |

**Installation:** No new crates needed if regex-lite is added to assay-tui's existing workspace deps.

## Architecture Patterns

### SAFE-01: killpg in kill_agent_subprocess

**What:** Change `libc::kill(pid as i32, signal)` to `libc::killpg(pid as libc::pid_t, signal)` in
both the SIGTERM step (line ~204) and the SIGKILL step (line ~253) of `kill_agent_subprocess`.

**Reference (gate/mod.rs:791-794):**
```rust
// SAFETY: child.id() returns a u32; process_group(0) set
// pgid == pid, so killpg sends SIGKILL to the entire group.
let pid = child.id() as libc::pid_t;
unsafe { libc::killpg(pid, libc::SIGKILL) };
```

**Key insight:** `launch_agent_streaming` already calls `cmd.process_group(0)` on Unix, which sets
the subprocess pgid == pid. So `killpg(pid, signal)` correctly kills the entire process group
(agent + any spawned tool subprocesses). The `kill_agent_subprocess` signature does not need to
change — `child.id()` still provides the pgid.

**Note on call sites:** The grep shows `kill_agent_subprocess` is currently only called from test
code. The production checkpoint-abort in `pipeline.rs:897-924` already uses
`libc::kill(-(pid as i32), SIGTERM)` inline (negative PID form, equivalent to killpg). SAFE-01
should fix the function itself so tests and any future callers are correct, and should also
standardize the inline call in pipeline.rs to use `killpg` for consistency.

### SAFE-02: TOCTOU Race in Auto-Promote

**What the race is:**
```
Thread 1: load_feature_spec() → status == InProgress  [reads file]
Thread 2: promote_spec(InProgress → Verified)          [writes file]
Thread 1: promote_spec(target=Some(Verified))          [reads again, old=Verified, writes Verified]
```

With the current code, this is mostly benign (writing Verified over Verified), but has a window
where a concurrent promotion with `target=None` could advance past Verified unexpectedly.

**Recommended fix:** Wrap the status guard and the promote call inside a
`promote_spec_if_still_in_progress` helper that re-reads status atomically and returns a typed
`AlreadyPromoted` variant if the file has already moved on. The caller in pipeline.rs treats
`AlreadyPromoted` as success (no-op with info log). This is fully filesystem-based — no Mutex
needed, and works even if multiple pipeline processes run concurrently.

Alternatively (simpler): rely on `promote_spec` to return an appropriate error when status is
already beyond InProgress, and add an `AlreadyTerminal`/`AlreadyAtTarget` match arm in pipeline.rs
that logs info instead of warning.

**The existing `promote_spec` error path** at line 174 is the right hook:
```rust
Err(e) => {
    // AlreadyTerminal or any other error — warn and continue.
    warn!(spec = %spec_name, error = %e, "auto-promote failed (non-fatal); continuing pipeline");
}
```
The TOCTOU fix should make this path distinguish "already promoted to target" from a genuine error.

### SAFE-03: Pipeline Crash Stderr Capture

**Root cause:** `launch_agent_streaming` at line 474 sets `stderr(Stdio::inherit())`, forwarding
stderr to the parent's stderr. The relay thread only reads `child.stdout`. When the process crashes,
no stderr content is available in the error path at pipeline.rs:962-974.

**Fix:** Change `stderr(Stdio::piped())` in the streaming spawn. Capture stderr concurrently in the
relay thread or a second reader thread. Buffer the last N bytes (e.g. 4096 bytes, trim if larger) and
make them available as part of `StreamingAgentHandle`. When the crash error is constructed at
pipeline.rs:962, include the captured stderr.

**Reference pattern (pr.rs:111-121):**
```rust
let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
return Err(AssayError::Io {
    source: io::Error::other(format!(
        "gh exited with status {}: {}",
        output.status,
        if stderr.is_empty() { format!("(no stderr; exit {})", output.status) } else { stderr }
    )),
});
```

**Implementation note:** Two concurrent read loops (stdout relay + stderr drainer) need to be on
separate threads to avoid deadlock. Or use a second JoinHandle that captures stderr before process
exit. `StreamingAgentHandle` needs a `stderr` field of type `Arc<Mutex<String>>` or the stderr
thread can be joined at crash-detect time.

### SAFE-04: Relay Thread Panic Logging

**Current production site (pipeline.rs:924):**
```rust
let exit_code = agent_handle.relay.join().unwrap_or(-1);
```
`unwrap_or(-1)` swallows panics silently — caller gets `-1` as if the process exited with signal.

**Fix pattern:**
```rust
let exit_code = match agent_handle.relay.join() {
    Ok(code) => code,
    Err(e) => {
        let msg = e.downcast_ref::<&str>().copied()
            .or_else(|| e.downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("(non-string panic payload)");
        tracing::error!(panic = %msg, "relay thread panicked");
        -1
    }
};
```

**Test sites:** The test `.expect("thread panicked")` calls at lines 1497, 1524, 1533, 1552 are
appropriate in tests (panics should surface) — they do NOT need to change.

**Where to apply:** pipeline.rs:924 (production relay join). Also check
`pipeline_streaming.rs` if it exists for any additional relay join sites.

### SAFE-05: TUI ANSI Stripping

**Current code flaw (app.rs:320-334):**
```rust
fn sanitize(s: String) -> String {
    if s.bytes().any(|b| b < 0x20 && b != b'\t') {
        s.chars().map(|c| {
            if (c as u32) < 0x20 && c != '\t' { '\u{FFFD}' } else { c }
        }).collect()
    } else { s }
}
```
ESC (0x1B = 27 < 32) → replaced with U+FFFD. But the rest of the CSI sequence
(`[31m`, `[0m`, etc.) consists of printable ASCII and passes through unchanged. Terminal sees
`<FFFD>[31m` — the ESC is gone but the `[31m` text still appears in the TUI.

**Fix with regex-lite (recommended — no new dep if added to assay-tui):**
```rust
// In assay-tui/Cargo.toml: add regex-lite.workspace = true
use regex_lite::Regex;
use std::sync::OnceLock;

fn ansi_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Matches: ESC [ ... final-byte (CSI sequences)
    //          ESC non-[ single char (other Fe sequences)
    RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]|\x1b[^[]").unwrap())
}

fn sanitize(s: String) -> String {
    // Strip full ANSI/CSI sequences first, then replace remaining control chars.
    let stripped = ansi_re().replace_all(&s, "");
    if stripped.bytes().any(|b| b < 0x20 && b != b'\t') {
        stripped.chars().map(|c| {
            if (c as u32) < 0x20 && c != '\t' { '\u{FFFD}' } else { c }
        }).collect()
    } else {
        stripped.into_owned()
    }
}
```

**Alternative — manual byte-scan state machine (zero deps, slightly more code):**
Scan bytes; when ESC is seen, enter a state that consumes the sequence without emitting. A full
ANSI-aware parser handles CSI (`ESC [`), OSC (`ESC ]`...`BEL`), and single-char sequences. This
is correct but ~30-40 lines.

**Alternative — `strip-ansi-escapes` crate:**
Operates on `&[u8]` via a `Write` adapter. Requires adding to workspace Cargo.toml and handling
UTF-8 round-trip. Adds a new dep for relatively small gain over regex-lite.

**Recommendation:** Use `regex-lite` (workspace dep, already used in assay-core) added to
`assay-tui/Cargo.toml`. Lazy static via `OnceLock`. Handles CSI + single-char Fe sequences.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Atomic file write | Custom open/write/rename | `tempfile::NamedTempFile::persist` | Handles cross-filesystem rename, cleanup on drop |
| Process group signal | `kill(-pid)` negative-PID hack | `libc::killpg(pgid, sig)` | POSIX-standard, self-documenting |
| Panic payload extraction | `std::panic::catch_unwind` wrapper | `JoinHandle::join()` Err branch | Thread panics are already captured by join; no extra wrapper needed |

## Common Pitfalls

### Pitfall 1: Forgetting to set `process_group(0)` before using `killpg`
**What goes wrong:** `killpg(pid, sig)` sends to process group `pid`. If the child was spawned
without `process_group(0)`, it inherits the parent's pgid, and `killpg` kills the wrong group.
**How to avoid:** Verify that all spawning paths (including the test spawn in
`kill_helper_terminates_long_running_process`) set `process_group(0)` before calling `killpg`.
The existing test spawns without `process_group(0)` — after the SAFE-01 fix, the test needs
updating to also set `process_group(0)`, otherwise `killpg(pid, sig)` will send the signal to the
test process group.
**Warning signs:** `killpg` test kills the test runner instead of the target child.

### Pitfall 2: Deadlock when adding stderr pipe to streaming launch
**What goes wrong:** If both stdout and stderr are piped, and the child generates enough output to
fill both pipe buffers, and only one is being read, the child blocks on write and the reader
deadlocks.
**How to avoid:** Stderr must be read concurrently with stdout. Spawn a dedicated stderr-reader
thread inside the relay closure, or use a second `JoinHandle`.
**Warning signs:** Relay thread hangs indefinitely on processes with verbose stderr.

### Pitfall 3: ANSI regex not covering OSC and DCS sequences
**What goes wrong:** A minimal regex (`ESC [ ... final`) covers CSI but misses OSC (`ESC ]` ...
`BEL`/`ST`), DCS, and other less-common sequences. These can be used for terminal injection too.
**How to avoid:** The regex should cover at minimum: CSI (`\x1b\[[0-9;]*[A-Za-z]`) and
single-char Fe sequences (`\x1b[^[]`). OSC sequences terminated by BEL are rarer in agent output
but can be added: `\x1b\][^\x07]*\x07`.
**Warning signs:** TUI shows garbage like `]0;title` window-title escape leakage.

### Pitfall 4: TOCTOU fix introducing silent no-op on genuine promotion failure
**What goes wrong:** If the "already promoted" case is too broadly caught, a genuine promote
failure (file permission error, corrupt TOML) is silently swallowed.
**How to avoid:** Only treat the "status is no longer InProgress" case as benign. All other errors
should still warn. This requires `promote_spec` to return a typed error or the caller to re-read
and inspect the actual current status.

### Pitfall 5: relay.join() Err arm unreachable — testing panic path
**What goes wrong:** It's hard to write a unit test that makes the relay thread panic to verify
the logging path.
**How to avoid:** The logging path can be integration-tested by spawning a thread that panics
and verifying log output via `tracing-test`, OR the fix can simply be code-reviewed as obviously
correct. Do not weaken the test to just assert compilation.

## Code Examples

### Verified pattern: killpg (gate/mod.rs:791-794)
```rust
// SAFETY: child.id() returns a u32; process_group(0) set
// pgid == pid, so killpg sends SIGKILL to the entire group.
let pid = child.id() as libc::pid_t;
unsafe { libc::killpg(pid, libc::SIGKILL) };
```
Source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/gate/mod.rs:791`

### Verified pattern: stderr capture in process output (pr.rs:110-124)
```rust
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    return Err(AssayError::Io {
        source: io::Error::other(format!(
            "gh exited with status {}: {}",
            output.status,
            if stderr.is_empty() { format!("(no stderr; exit {})", output.status) } else { stderr }
        )),
    });
}
```
Source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/pr.rs:110`

### Verified pattern: JoinHandle panic extraction
```rust
// stdlib: JoinHandle<T>::join() -> Result<T, Box<dyn Any + Send>>
match handle.join() {
    Ok(exit_code) => exit_code,
    Err(e) => {
        let msg = e.downcast_ref::<&str>().copied()
            .or_else(|| e.downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("(non-string panic payload)");
        tracing::error!(panic = %msg, "relay thread panicked; treating as exit -1");
        -1
    }
}
```
Source: Rust stdlib docs — confirmed behavior of `std::thread::JoinHandle::join`.

### Verified pattern: process_group(0) at spawn (pipeline.rs:479-482)
```rust
#[cfg(unix)]
{
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}
```
Source: `/Users/wollax/Git/personal/assay/crates/assay-core/src/pipeline.rs:478`

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `child.kill()` / `kill(pid)` single process | `killpg(pgid, sig)` process group | Kills entire agent process tree (agent + tools) |
| Stderr inherited (not captured) | Stderr piped and buffered | Crash messages include what the agent actually printed |
| `.expect("thread panicked")` | `match join() { Err => log + recover }` | Host process survives relay thread panics |
| ESC-byte replacement | Full CSI/Fe sequence stripping | No partial escape sequence leakage in TUI |
| Read-then-write without guard | Atomic promote with idempotency check | No double-promotion window |

## Open Questions

1. **SAFE-03: How much stderr to buffer**
   - What we know: stderr from a crashed agent could be arbitrarily large (stack traces, etc.)
   - What's unclear: whether there's an existing cap or convention for this in the codebase
   - Recommendation: Cap at 4096 bytes (trim head, keep tail — most useful part is the end).

2. **SAFE-02: Whether promote_spec needs a new error variant**
   - What we know: Current error type is `AssayError::Io` which does not distinguish "already at target" from IO errors
   - What's unclear: whether to add a typed `AlreadyAtStatus` variant or just re-read the file and check
   - Recommendation: Re-read status after failed promote and if current == target, log info and treat as success. No new error variant needed.

3. **SAFE-01: The test `kill_helper_terminates_long_running_process` spawns without `process_group(0)`**
   - What we know: After switching to `killpg`, the test child must also set `process_group(0)` or `killpg` will signal the wrong group (the test runner's pgid)
   - Recommendation: Update the test spawn to add `process_group(0)` when fixing the production code.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | `Cargo.toml` workspace (no separate test config) |
| Quick run command | `cargo test -p assay-core pipeline_checkpoint 2>/dev/null` |
| Full suite command | `just test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SAFE-01 | `kill_agent_subprocess` kills process group | unit | `cargo test -p assay-core kill_helper_terminates_long_running_process` | ✅ (needs update for pgid) |
| SAFE-01 | SIGKILL fallback also uses killpg | unit | `cargo test -p assay-core kill_helper` | ✅ (same test) |
| SAFE-02 | Auto-promote TOCTOU race — noop on double call | unit | `cargo test -p assay-core test_auto_promote_already_verified_is_noop` | ❌ Wave 0 |
| SAFE-03 | Crash error includes stderr content | unit | `cargo test -p assay-core launch_agent_crash_includes_stderr` | ❌ Wave 0 |
| SAFE-04 | Relay panic logged, host survives | unit | `cargo test -p assay-core relay_panic_is_logged` | ❌ Wave 0 |
| SAFE-05 | sanitize strips full ANSI sequences | unit | `cargo test -p assay-tui sanitize_strips_ansi` | ❌ Wave 0 |
| SAFE-05 | sanitize strips CSI color codes | unit | `cargo test -p assay-tui sanitize_strips_csi_color` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p assay-core -p assay-tui 2>/dev/null`
- **Per wave merge:** `just test`
- **Phase gate:** `just ready` (fmt-check + lint + test + deny) green before `/kata:verify-work`

### Wave 0 Gaps
- [ ] Test for SAFE-02: `test_auto_promote_already_verified_is_noop` in `pipeline.rs` or a dedicated test module
- [ ] Test for SAFE-03: `launch_agent_crash_includes_stderr` in `pipeline.rs` test module — spawn a child that writes to stderr and exits non-zero; assert crash error contains the stderr text
- [ ] Test for SAFE-04: `relay_panic_is_logged` — needs `tracing-test` + a relay thread that panics; assert log contains panic message and function returns -1
- [ ] Test for SAFE-05: `sanitize_strips_ansi` in `assay-tui/src/app.rs` test module — assert `\x1b[31mred\x1b[0m` → `red`, `\x1b[?25l` → `` (stripped), plain text passes through

## Sources

### Primary (HIGH confidence)
- Direct codebase inspection — all findings are from reading the actual source files
- `/Users/wollax/Git/personal/assay/crates/assay-core/src/gate/mod.rs:791-794` — killpg reference pattern
- `/Users/wollax/Git/personal/assay/crates/assay-core/src/pipeline_checkpoint.rs:191-266` — kill_agent_subprocess current impl
- `/Users/wollax/Git/personal/assay/crates/assay-core/src/pipeline.rs:394-608` — launch_agent_streaming, relay thread, stderr(inherit) 
- `/Users/wollax/Git/personal/assay/crates/assay-core/src/pipeline.rs:897-974` — checkpoint abort, crash error path
- `/Users/wollax/Git/personal/assay/crates/assay-core/src/spec/promote.rs` — promote_spec, atomic tempfile-rename
- `/Users/wollax/Git/personal/assay/crates/assay-tui/src/app.rs:315-370` — existing sanitize(), TextDelta handling
- `/Users/wollax/Git/personal/assay/crates/assay-core/src/pr.rs:110-124` — stderr capture reference
- `Cargo.toml` workspace — confirmed libc, regex-lite, tempfile, tracing deps available

### Secondary (MEDIUM confidence)
- Rust stdlib docs on `std::thread::JoinHandle::join` returning `Result<T, Box<dyn Any + Send>>` — panic payload extraction is standard Rust
- `cargo search strip-ansi-escapes` — version 0.2.1 exists if needed

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps verified in actual Cargo.toml files
- Architecture: HIGH — patterns verified in actual source code; reference implementations located
- Pitfalls: HIGH — SAFE-01 test pgid issue is a concrete code observation; deadlock risk is well-understood
- ANSI regex: HIGH — ESC=0x1B<0x20 confirmed; CSI sequence format is ANSI X3.64 standard

**Research date:** 2026-04-08
**Valid until:** 2026-05-08 (stable Rust APIs; no fast-moving deps)
