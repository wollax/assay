# Phase 36: Correctness & Robustness — Research

## Standard Stack

No new external dependencies required. All work uses existing crate capabilities:

- **Git operations**: `std::process::Command` shelling out to `git` CLI (established pattern in `crates/assay-core/src/worktree.rs`)
- **Truncation**: `truncate_head_tail()` in `crates/assay-core/src/gate/mod.rs` (private, 32 KiB budget already defined as `STREAM_BUDGET`)
- **Serialization**: `serde` + `schemars` (existing derive patterns on all types)
- **Async**: `tokio::task::spawn_blocking` for git operations in MCP handlers (existing pattern)

## Architecture Patterns

### File Locations and Key Structures

| Component | File | Key Types/Functions |
|-----------|------|-------------------|
| Worktree types | `crates/assay-types/src/worktree.rs` | `WorktreeInfo`, `WorktreeStatus`, `WorktreeConfig` |
| Worktree logic | `crates/assay-core/src/worktree.rs` | `create()`, `list()`, `status()`, `cleanup()`, `git_command()` |
| Session types | `crates/assay-types/src/session.rs` | `AgentSession` |
| Session logic | `crates/assay-core/src/gate/session.rs` | `create_session()`, `report_evaluation()`, `build_finalized_record()`, `finalize_session()`, `finalize_as_timed_out()` |
| Truncation | `crates/assay-core/src/gate/mod.rs:661-715` | `TruncationResult` (struct), `truncate_head_tail()` (fn) |
| MCP server | `crates/assay-mcp/src/server.rs` | `AssayServer`, all handler methods, response structs |
| Error types | `crates/assay-core/src/error.rs` | `AssayError` (non-exhaustive enum) |
| Schema tests | `crates/assay-types/tests/schema_snapshots.rs` | Snapshot tests for `AgentSession` schema |
| Roundtrip tests | `crates/assay-types/tests/schema_roundtrip.rs` | Validation tests for `WorktreeStatus` |

### FIX-01: Worktree Status — Current vs Target

**Current behavior** (`crates/assay-core/src/worktree.rs:225-266`):
- `status()` takes `(worktree_path, spec_slug)` — no base branch info available
- Ahead/behind computed against `@{upstream}` (line 243): `git rev-list --left-right --count HEAD...@{upstream}`
- Falls back to `(0, 0)` when no upstream is configured
- `WorktreeStatus` has `ahead: usize` and `behind: usize` (non-nullable, line 61-63)

**Problem**: Assay-created branches (`assay/<slug>`) are local-only — they have no upstream. So ahead/behind is always `(0, 0)`. The useful comparison is against the base branch the worktree was created from.

**What needs to change**:

1. **Metadata file**: Store base branch at worktree creation time. Currently `create()` returns `WorktreeInfo` with `base_branch: Some(base)` (line 191) but this is only in the return value — not persisted anywhere. Need a `metadata.json` file inside the worktree (e.g., `.assay-worktree.json` or in the worktree's `.git` area).

   Recommended location: `<worktree_path>/.assay/worktree.json` — keeps it with the worktree, easy to read. Alternative: `<project_root>/.assay/worktrees/<slug>.json` — survives worktree deletion but requires project root access.

   **Confidence: HIGH** — `<worktree_path>/.assay/worktree.json` is cleanest since the worktree path is always available to `status()`.

2. **`WorktreeStatus` type change**: `ahead` and `behind` must become `Option<usize>` to represent "base ref not found" as null rather than misleading zeros. This is a **breaking schema change** affecting `crates/assay-types/src/worktree.rs`, roundtrip tests, and snapshot tests.

3. **`status()` signature change**: Must accept base branch info (either read metadata internally, or receive it as a parameter). Adding a `warnings: Vec<String>` return channel for non-blocking issues.

4. **`WorktreeStatusParams`**: Add optional `fetch: bool` parameter (defaults to false). When true, run `git fetch origin` before computing ahead/behind.

5. **Ahead/behind git command**: Change from `HEAD...@{upstream}` to `HEAD...<base_ref>` where base_ref resolution tries `origin/<base>` first, falls back to `refs/heads/<base>`.

   ```
   git rev-list --left-right --count HEAD...origin/<base_branch>
   ```
   If that ref doesn't exist:
   ```
   git rev-list --left-right --count HEAD...refs/heads/<base_branch>
   ```
   If neither exists: return `None` for both + warning.

### FIX-02: Gate Session Error Messages — Current vs Target

**Current behavior** (`crates/assay-mcp/src/server.rs`):

`gate_report` (line 733-737):
```rust
let Some(session) = sessions.get_mut(&p.session_id) else {
    return Ok(CallToolResult::error(vec![Content::text(format!(
        "session '{}' not found (expired or already finalized)",
        p.session_id
    ))]));
};
```

`gate_finalize` (line 794-798):
```rust
let Some(session) = session else {
    return Ok(CallToolResult::error(vec![Content::text(format!(
        "session '{}' not found (expired or already finalized)",
        session_id
    ))]));
};
```

**Problem**: Both handlers produce the same generic error for two distinct failure modes:
1. Session timed out (auto-finalized after `SESSION_TIMEOUT_SECS = 1800`)
2. Session never existed or was already manually finalized

No recovery hints are provided. Agent gets a dead-end error.

**What needs to change**:

1. **Track timed-out sessions**: When the timeout task fires (line 666-694), store the session ID in a separate set (e.g., `timed_out_sessions: Arc<Mutex<HashSet<String>>>` on `AssayServer`). This allows distinguishing "timed out" from "never existed".

2. **Error messages with recovery hints**:
   - Timeout: `"Session '{id}' timed out after {elapsed}s (timeout: {timeout}s). Use gate_run to start a new session."`
   - Not found: `"Session '{id}' not found. Use gate_run to start a new session."`

3. **Both `gate_report` and `gate_finalize`** use the same error format — extract a shared helper function.

4. **Timeout metadata**: The timeout task must record the elapsed time and configured timeout. Currently `SESSION_TIMEOUT_SECS` is a constant (1800). To report elapsed time, either store the session creation timestamp (already in `AgentSession.created_at`) or just use the constant since timeout fires at that exact interval.

   **Confidence: HIGH** — Use `Utc::now() - session.created_at` for elapsed, and `SESSION_TIMEOUT_SECS` for the configured timeout.

### FIX-03: Diff Capture — Current vs Target

**Current behavior**: No diff is captured at `gate_run` time. `AgentSession` has no diff-related fields.

**What needs to change**:

1. **New fields on `AgentSession`** (`crates/assay-types/src/session.rs`):
   ```rust
   /// Git diff captured at session creation time (`git diff HEAD`).
   /// None when working tree is clean or diff capture failed.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub diff: Option<String>,

   /// Whether the diff was truncated to fit the 32 KiB budget.
   #[serde(default, skip_serializing_if = "std::ops::Not::not")]
   pub diff_truncated: bool,

   /// Original diff size in bytes before truncation.
   /// Only present when truncation occurred.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub diff_bytes_original: Option<usize>,
   ```

   **Confidence: HIGH** — `diff` is cleaner than `diff_content` or `diff_text`. Store `diff_bytes_original` to match the existing `original_bytes` pattern on `GateResult`.

2. **Capture point**: In the `gate_run` MCP handler, after evaluating criteria and before creating the session. Run `git diff HEAD` in the working directory.

3. **Truncation**: Reuse `truncate_head_tail()` from `crates/assay-core/src/gate/mod.rs`. **Problem**: This function is currently `fn` (private to the module). It must be made `pub(crate)` or extracted to a shared utility.

   **Confidence: HIGH** — Make `truncate_head_tail` and `TruncationResult` `pub(crate)` visibility. The function is well-tested (8+ unit tests) and its API is stable.

4. **Git diff command**: `git diff HEAD` run in the working directory. Use the existing `git_command()` helper from `worktree.rs` or create a parallel helper in the MCP server. Since `git_command()` is private to `worktree.rs`, either:
   - Extract `git_command()` to a shared module in `assay-core` (e.g., `git.rs`)
   - Inline a simple `Command::new("git")` call in the gate_run handler

   **Recommendation**: Extract to `assay-core::git::run()` — there are now multiple consumers. **Confidence: MEDIUM** — inline in MCP handler is simpler for this phase; extraction could be deferred.

5. **Error handling for diff capture**: If `git diff HEAD` fails (not a git repo, git not installed, etc.), log a warning and continue without diff. Do NOT fail the gate run.

6. **Clean worktree**: If `git diff HEAD` returns empty string, set `diff: None` (not `Some("")`).

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| String truncation with byte budgets | `truncate_head_tail()` from `crates/assay-core/src/gate/mod.rs` — already handles UTF-8 boundary safety, head/tail splitting, and marker formatting |
| Session timeout tracking | Extend existing `AssayServer.sessions` + new `timed_out_sessions` set — don't build a separate timeout registry |
| Git CLI invocation | Follow existing `Command::new("git")` pattern from `crates/assay-core/src/worktree.rs:19-43` |
| Schema snapshot tests | Use `insta::assert_snapshot!` with `schemars::schema_for!()` — existing pattern in `crates/assay-types/tests/schema_snapshots.rs` |

## Common Pitfalls

### FIX-01 Pitfalls

1. **`WorktreeStatus.ahead/behind` type change breaks schema snapshots**: The `agent-session` snapshot and `schema_roundtrip` tests reference the current non-nullable `usize` fields. Changing to `Option<usize>` requires updating snapshots with `cargo insta review` or `UPDATE_SNAPSHOTS=1`.

2. **Metadata file not written during `create()`**: If `create()` doesn't write metadata, `status()` has no way to know the base branch. Must ensure metadata write happens atomically with worktree creation — write metadata AFTER `git worktree add` succeeds, but BEFORE returning `Ok`.

3. **`git fetch` in `status()` blocks the MCP handler**: The fetch parameter triggers a network operation. Must run in `spawn_blocking` from the MCP handler level (already the pattern for worktree operations).

4. **Remote ref format**: `origin/<branch>` vs `refs/remotes/origin/<branch>` — `git rev-list` accepts both, but use the short form for consistency with how `detect_default_branch()` already works.

### FIX-02 Pitfalls

1. **`timed_out_sessions` set grows unbounded**: Need a cleanup strategy — e.g., only keep the last N session IDs, or entries with timestamps that auto-expire. A simple `HashMap<String, DateTime<Utc>>` with a capacity cap works.

2. **Race between timeout task and manual finalize**: The timeout task removes the session from `sessions` map. If `gate_finalize` runs concurrently, one will find `None`. The current code already handles this correctly (timeout task checks `sessions.remove()` returns `Some`). Just need to ensure the timed_out set is populated BEFORE removing from sessions.

3. **Error message consistency**: Both `gate_report` and `gate_finalize` must produce identical error format. Extract to a shared function: `fn session_lookup_error(id: &str, timed_out: &HashSet<String>) -> CallToolResult`.

### FIX-03 Pitfalls

1. **`truncate_head_tail` visibility**: Currently private (`fn`, not `pub`). Must change to `pub(crate)` along with `TruncationResult`. This is a non-breaking change within the workspace.

2. **Large diffs can block `spawn_blocking`**: `git diff HEAD` on a repo with large binary changes can produce megabytes of output. The 32 KiB truncation only applies AFTER the full output is captured. Consider using `--stat` as a secondary fallback if raw diff exceeds some threshold? **No** — the CONTEXT.md specifies `git diff HEAD` with truncation. Just capture and truncate.

3. **Diff capture must NOT use `worktree.rs::git_command()`**: That function returns `WorktreeGit` / `WorktreeGitFailed` errors which are semantically wrong for diff capture. Either inline the `Command` call or create a generic git helper.

4. **`AgentSession` schema snapshot will change**: Adding `diff`, `diff_truncated`, `diff_bytes_original` fields changes the JSON Schema. Must update snapshot file `crates/assay-types/tests/snapshots/schema_snapshots__agent-session-schema.snap`.

5. **Empty diff vs no diff**: `git diff HEAD` on a clean worktree exits 0 with empty stdout. Must map `""` to `None`, not `Some("")`.

## Code Examples

### Ahead/behind against base branch ref

```rust
fn ahead_behind_base(
    worktree_path: &Path,
    base_branch: &str,
) -> Result<(usize, usize), String> {
    // Try remote-tracking ref first, fall back to local
    let base_ref = format!("origin/{base_branch}");
    let fallback_ref = format!("refs/heads/{base_branch}");

    let ref_to_use = if git_command(&["rev-parse", "--verify", &base_ref], worktree_path).is_ok() {
        base_ref
    } else if git_command(&["rev-parse", "--verify", &fallback_ref], worktree_path).is_ok() {
        fallback_ref
    } else {
        return Err(format!("base branch '{base_branch}' not found as remote or local ref"));
    };

    let output = git_command(
        &["rev-list", "--left-right", "--count", &format!("HEAD...{ref_to_use}")],
        worktree_path,
    )?;

    let parts: Vec<&str> = output.split('\t').collect();
    if parts.len() == 2 {
        Ok((
            parts[0].parse::<usize>().unwrap_or(0),
            parts[1].parse::<usize>().unwrap_or(0),
        ))
    } else {
        Err("unexpected rev-list output format".to_string())
    }
}
```

### Worktree metadata file

```rust
// At creation time, write metadata:
#[derive(Serialize, Deserialize)]
struct WorktreeMetadata {
    base_branch: String,
    created_at: DateTime<Utc>,
}

// Write to: <worktree_path>/.assay/worktree.json
// Read in status() to get base_branch
```

### Session lookup with timeout detection

```rust
fn session_not_found_error(
    session_id: &str,
    timed_out: &HashMap<String, TimedOutInfo>,
) -> CallToolResult {
    if let Some(info) = timed_out.get(session_id) {
        CallToolResult::error(vec![Content::text(format!(
            "Session '{}' timed out after {}s (timeout: {}s). \
             Use gate_run to start a new session.",
            session_id, info.elapsed_secs, info.timeout_secs
        ))])
    } else {
        CallToolResult::error(vec![Content::text(format!(
            "Session '{}' not found. Use gate_run to start a new session.",
            session_id
        ))])
    }
}
```

### Diff capture at gate_run time

```rust
// In gate_run handler, after eval completes:
let (diff, diff_truncated, diff_bytes_original) = {
    let output = std::process::Command::new("git")
        .args(["diff", "HEAD"])
        .current_dir(&working_dir)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let raw = String::from_utf8_lossy(&out.stdout).to_string();
            if raw.is_empty() {
                (None, false, None)
            } else {
                let result = assay_core::gate::truncate_head_tail(&raw, DIFF_BUDGET);
                let bytes_orig = if result.truncated {
                    Some(result.original_bytes)
                } else {
                    None
                };
                (Some(result.output), result.truncated, bytes_orig)
            }
        }
        Ok(_) | Err(_) => {
            // Git diff failed — warn but don't block gate run
            tracing::warn!("git diff HEAD failed, continuing without diff");
            (None, false, None)
        }
    }
};
```

### Making truncation public

```rust
// In crates/assay-core/src/gate/mod.rs:
// Change visibility from private to pub(crate)

/// Result of applying head+tail truncation to a string.
pub(crate) struct TruncationResult { ... }

/// Truncate output using a head+tail strategy with a byte budget.
pub(crate) fn truncate_head_tail(input: &str, budget: usize) -> TruncationResult { ... }
```

## Verification Checklist

- [ ] `WorktreeStatus.ahead` and `.behind` are `Option<usize>`, not `usize`
- [ ] Worktree metadata file written at creation, read at status time
- [ ] `git rev-list` uses base branch ref, not `@{upstream}`
- [ ] `WorktreeStatusParams` has `fetch: Option<bool>` parameter
- [ ] `WorktreeStatus` has `warnings: Vec<String>` field (or MCP response wraps with warnings)
- [ ] `gate_report` and `gate_finalize` distinguish timeout vs not-found errors
- [ ] Error messages include recovery hints suggesting `gate_run`
- [ ] Timeout errors include elapsed time and configured timeout
- [ ] `AgentSession` has `diff: Option<String>`, `diff_truncated: bool`, `diff_bytes_original: Option<usize>`
- [ ] `truncate_head_tail()` is `pub(crate)` accessible from MCP crate (or re-exported)
- [ ] Empty `git diff HEAD` output maps to `None`, not `Some("")`
- [ ] Schema snapshot tests updated for both `AgentSession` and `WorktreeStatus`
- [ ] Schema roundtrip tests updated for modified types

---

*Researched: 2026-03-11*
*Phase: 36-correctness-robustness*
