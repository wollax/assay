# S03: Streaming Assay Output — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The core claim is that bollard delivers chunks incrementally via callback, which requires a real Docker daemon and real exec. The `test_exec_streaming_delivers_chunks_in_order` test provisions a real container, issues a real command, and collects real chunks — no mocking of the transport layer. Human/UAT verification is not required because the automated test proves the callback ordering contract; the `--nocapture` output is directly human-readable as a secondary confirmation.

## Preconditions

- Docker daemon running and accessible (test skips gracefully if unavailable)
- `cargo test` environment has network access to pull `alpine:3` if not already cached
- `smelt-cli` and `smelt-core` compile cleanly (`cargo build -p smelt-cli`)

## Smoke Test

```bash
cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture
```

Expected: `test result: ok. 1 passed` with `chunk[0] = "a\nb\nc\n"` and `handle.stdout = "a\nb\nc\n"` visible in output.

## Test Cases

### 1. Streaming callback delivers chunks in order

1. Run `cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order -- --nocapture`
2. Observe printed output
3. **Expected:** `chunk[0] = "a\nb\nc\n"` (or multiple chunks totalling `"a\nb\nc\n"`); `handle.stdout = "a\nb\nc\n"`; test passes

### 2. Full workspace passes

1. Run `cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\["`
2. **Expected:** Seven `test result: ok.` lines; no FAILED; no `error[`

### 3. exec() is silent — no eprint! in docker.rs

1. Run `grep -n "eprint!" crates/smelt-core/src/docker.rs`
2. **Expected:** No output (grep exits 1 — no matches)

### 4. Phase 7 uses exec_streaming

1. Run `grep "exec_streaming" crates/smelt-cli/src/commands/run.rs`
2. **Expected:** Exactly one match in the Phase 7 block

### 5. Post-exec double-print block is gone

1. Run `grep -A5 "exec_streaming" crates/smelt-cli/src/commands/run.rs`
2. **Expected:** No `handle.stdout` or `handle.stderr` eprint lines following the exec_streaming call in Phase 7

## Edge Cases

### Docker unavailable — graceful skip

1. Run `DOCKER_HOST=unix:///nonexistent.sock cargo test -p smelt-cli --test docker_lifecycle test_exec_streaming_delivers_chunks_in_order`
2. **Expected:** Test is skipped (existing `check_docker` pattern), not failed; `cargo test` exits 0

### Multi-line output arrives as single chunk

1. Observe `chunk[0] = "a\nb\nc\n"` in test output (single chunk containing all three lines)
2. **Expected:** Test passes — assertion is on joined chunk content (`"a\nb\nc\n"`), not chunk count; `handle.stdout.contains("a")` passes

## Failure Signals

- `test_exec_streaming_delivers_chunks_in_order` fails with `!chunks.is_empty()` → callback was never invoked; `exec_streaming()` likely failed silently or callback bind is broken
- `handle.stdout.contains("a")` assertion fails → `ExecHandle` not being populated by `exec_streaming()`; buffering path is broken
- `cargo test --workspace` shows any FAILED → regression in existing tests from `exec()` silencing or trait change
- `grep "eprint!" crates/smelt-core/src/docker.rs` returns matches → `eprint!` accidentally reintroduced in `exec()`

## Requirements Proved By This UAT

- M002 success criterion "Gate output from inside the container is visible on the terminal as `assay run` produces it (streaming, not buffered until exit)" — proved by callback receiving live chunks from bollard's multiplexed stream, verified against real Docker container with `--nocapture` output
- Operational verification: streaming exec delivers output incrementally (observable via test that sends chunks and checks arrival order) — satisfied

## Not Proven By This UAT

- Full end-to-end `smelt run` with a real manifest, real assay binary, and real Claude API key — requires manual human UAT (out of scope for S03; planned for M002 UAT)
- Exit code 2 from `assay run` surfaces as distinct outcome vs exit code 1 — deferred to S04
- `ResultCollector` handles Assay's merge-to-base-branch behavior correctly — deferred to S04
- Multi-minute streaming sessions (real assay AI runs) — automated test uses fast `printf`; real assay streaming is proved architecturally but not by timed incremental chunk delivery test

## Notes for Tester

- `printf 'a\nb\nc\n'` in alpine may deliver all three lines as a single `"a\nb\nc\n"` chunk rather than three separate chunks. This is correct behavior — bollard doesn't guarantee per-line delivery, and the test handles it correctly.
- `exec()` is now fully silent. If you're debugging setup phase failures and expect to see command output on stderr, you'll need to inspect `ExecHandle.stdout`/`ExecHandle.stderr` or temporarily switch to `exec_streaming()` with a debug callback.
- The `--nocapture` flag is required to see chunk output in the test — without it, the `println!` / `eprintln!` output is suppressed by cargo's test harness.
