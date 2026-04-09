---
phase: 60-process-safety
plan: 03
status: complete
completed_at: "2026-04-09T01:45:00Z"
duration_minutes: 15
requirements: [SAFE-02, SAFE-03]
commits:
  - hash: 9b481ce
    message: "feat(60-03): TOCTOU-safe auto-promote + stderr capture in crash errors"
---

# 60-03 Summary: TOCTOU-safe auto-promote + stderr capture in crash errors

TOCTOU guard added to auto-promote error path; stderr now piped and captured for inclusion in agent crash error messages.

## Task Results

| Task | Name | Status | Commits |
|------|------|--------|---------|
| 1 | TOCTOU-safe auto-promote + stderr capture in crash errors | complete | 9b481ce |

## What Was Built

### SAFE-02: TOCTOU-safe auto-promote (`pipeline.rs:1239-1280`)

The `Err` arm of the `promote_spec` call in the auto-promote block now re-reads the spec on disk before deciding how to respond. If the spec is already at `Verified` (meaning a concurrent process beat us to the promotion), it logs at `info` level and records `auto_promoted = true` on the session — treating this as a success. Genuine IO errors or unexpected states still `warn` and continue.

Key snippet: `match crate::spec::load_feature_spec(spec_toml_path)` — re-read after error, check `current.status == SpecStatus::Verified`.

### SAFE-03: Stderr capture in crash errors (`pipeline.rs:422-650, 1011-1033`)

- `StreamingAgentHandle` gains `stderr_buffer: Arc<Mutex<String>>` field
- `launch_agent_streaming` changes `stderr(Stdio::inherit())` to `stderr(Stdio::piped())`
- A dedicated stderr reader thread reads line-by-line into the shared buffer, capped at 4096 bytes (tail-biased: head is trimmed when over cap)
- Stdout and stderr are read on separate threads — no pipe deadlock risk
- Crash error recovery field now includes `\n\nCaptured stderr:\n{content}` when non-empty
- Empty-args guard path also returns a valid (empty) `stderr_buffer`

## Deviations

None. Implementation matched the plan exactly.

## Verification

- `cargo test -p assay-core pipeline` — 37 passed (3 new tests added)
- `cargo test -p assay-core promote` — 23 passed (unchanged)
- `cargo clippy -p assay-core -- -D warnings` — clean
- `cargo fmt --check` — clean (formatting corrected before final commit)
- `Stdio::piped()` confirmed for stderr in `launch_agent_streaming`
- "already at target" info log path confirmed in auto-promote `Err` arm
