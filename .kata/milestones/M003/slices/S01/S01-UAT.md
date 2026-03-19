# S01: AI Conflict Resolution — UAT

**Milestone:** M003
**Written:** 2026-03-17

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The core lifecycle mechanics (two-phase merge, handler invocation, panic recovery, CLI/MCP routing) are proven by automated integration tests with real git repos. The remaining gap — that a real `claude -p` invocation actually resolves a genuine conflict — requires a live runtime because the AI's output is non-deterministic and the subprocess path is only exercised manually. This UAT closes that gap.

## Preconditions

- `assay` CLI built in release mode: `cargo build --release -p assay-cli`
- `claude` CLI available in PATH (Claude Code, logged in with a valid session)
- A test git repository with two branches that have a merge conflict in at least one file (see setup steps below)
- Working directory is the repo root

## Smoke Test

Run:
```sh
assay run manifest.toml --conflict-resolution auto --json
```
Expected: output contains `"status": "Merged"` for the conflicting session, not `"ConflictSkipped"`.

## Test Cases

### 1. Real Claude resolves a merge conflict end-to-end

Setup:
```sh
git init /tmp/uat-conflict-test && cd /tmp/uat-conflict-test
echo 'fn greet() { println!("Hello"); }' > src/lib.rs
git add . && git commit -m "init"
git checkout -b session-a
echo 'fn greet() { println!("Hello, World!"); }' > src/lib.rs
git add . && git commit -m "session-a changes"
git checkout main
git checkout -b session-b
echo 'fn greet() { println!("Hi there!"); }' > src/lib.rs
git add . && git commit -m "session-b changes"
git checkout main
```

Create `manifest.toml` (multi-session with overlapping file changes) pointing to the two branches, then:

```sh
assay run manifest.toml --conflict-resolution auto --json
```

1. Observe that the CLI invokes the orchestrator with two sessions
2. After both sessions complete and their branches are ready for merge, the merge runner encounters the conflict
3. Claude is invoked with the conflicted `src/lib.rs` contents
4. **Expected:** `MergeReport.results` shows `Merged` for the conflicting session with a valid `merge_sha`. The resolved `src/lib.rs` contains no conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`). `git log --oneline --graph` shows a proper merge commit with 2 parents.

### 2. Failure mode: claude not in PATH

1. Temporarily remove `claude` from PATH: `export PATH=/usr/bin:/bin`
2. Run: `assay run manifest.toml --conflict-resolution auto --json`
3. **Expected:** `MergeReport.results` shows `ConflictSkipped` for the conflicting session with `error` field containing "claude CLI not found". Repository is in clean state (no dangling `MERGE_HEAD`). Exit code is non-zero.

### 3. --conflict-resolution skip leaves conflicts unresolved

1. Run: `assay run manifest.toml --conflict-resolution skip --json`
2. **Expected:** `MergeReport.results` shows `ConflictSkipped` for the conflicting session. Conflict was not resolved. Repository is clean (merge --abort ran). No `claude` subprocess was spawned.

### 4. CLI help shows the flag

1. Run: `assay run --help`
2. **Expected:** Output includes `--conflict-resolution <CONFLICT_RESOLUTION>` with a description mentioning `auto` and `skip`.

### 5. MCP orchestrate_run routes correctly

1. Start assay MCP server
2. Call `orchestrate_run` with `conflict_resolution: "auto"` in the parameters
3. **Expected:** Server accepts the parameter without error. If a conflict arises during the run, Claude is invoked. `orchestrate_status` shows the run completed.

## Edge Cases

### Claude produces malformed JSON response

1. Set `claude` in PATH to a script that outputs garbage: `echo '#!/bin/sh\necho "not json"' > /tmp/fake-claude && chmod +x /tmp/fake-claude && export PATH=/tmp:$PATH`
2. Run: `assay run manifest.toml --conflict-resolution auto --json`
3. **Expected:** `ConflictSkipped` with a parse error message. Repository is clean. No crash.

### Claude times out

1. Set `claude` to a script that hangs: `echo '#!/bin/sh\nsleep 200' > /tmp/fake-claude && chmod +x /tmp/fake-claude`
2. Set `timeout_secs` in ConflictResolutionConfig to 5 (requires code change or config extension)
3. **Expected:** After ~5 seconds, `ConflictSkipped` with a timeout message. Repository is clean. Process was killed.

### Resolved file still contains conflict markers

1. Set `claude` to a script that returns a JSON response where the resolved file still contains `<<<<<<<` markers
2. Run with `--conflict-resolution auto`
3. **Expected:** Currently no validation — the bad resolution is committed and the merge succeeds (this is a known gap addressed in S02 via post-resolution validation). The audit trail (S02) will make this visible.

## Failure Signals

- `ConflictSkipped` status in MergeReport when `Merged` was expected
- `MERGE_HEAD` file present in `.git/` after run completes (indicates cleanup failure)
- Conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) present in source files after merge
- `just ready` test failure after the slice lands on main
- `error` field in `MergeSessionResult` containing "panic" (indicates unwind-safe handler failure)

## Requirements Proved By This UAT

- R026 (AI conflict resolution) — live Claude invocation successfully resolves a real merge conflict in a multi-session orchestration run, producing a clean merge commit with no conflict markers. CLI `--conflict-resolution auto` routes to the handler. MCP `conflict_resolution: "auto"` routes correctly. Failure modes (not found, malformed response) produce clean fallback with descriptive errors.

## Not Proven By This UAT

- Post-resolution validation command (R028) — not yet implemented; S02 concern
- Conflict resolution audit trail in MergeReport (R029) — not yet implemented; S02 concern
- `orchestrate_status` showing resolution details — S02 concern
- Non-deterministic AI resolution quality — Claude may produce subtly broken merges (duplicate definitions, broken imports). S02's validation command mitigates but doesn't eliminate this. Production use requires code review of AI-resolved conflicts.
- Concurrent conflict resolution — the merge runner is sequential; no concurrent resolution scenarios tested.
- Large files (>100KB) with many conflict hunks — prompt construction includes full file contents; very large conflicts may exceed Claude's context window. Not tested.

## Notes for Tester

- The most valuable UAT step is #1 (real Claude end-to-end). If Claude resolves the conflict correctly, that proves the happy path. The failure modes (steps 2-3) are fast and low-risk.
- `--json` flag on `assay run` produces structured output — easier to inspect `status` and `error` fields than parsing human-readable output.
- If `assay run` is not yet wired to a real orchestrator (i.e., you're testing on a manifest with sessions that have already-completed branches), you can also trigger the merge phase directly if assay exposes a separate merge command or MCP tool.
- For step 5 (MCP), the `mcp` gateway tool in Kata can be used to call `orchestrate_run` directly without a CLI wrapper.
