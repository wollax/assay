# Phase 46 Research: Worktree Fixes

## Standard Stack

No external crates needed. All changes use:
- `std::fs::canonicalize` — resolves symlinks and `..` segments to absolute paths
- `std::path::{Path, PathBuf}` — `parent()`, `file_name()`, `join()`
- Existing `git_command()` helper in `crates/assay-core/src/worktree.rs`
- Existing `AssayError` variants (no new error variants needed)

## Architecture Patterns

### WFIX-01: Path Canonicalization in `resolve_worktree_dir()`

**Current code** (`crates/assay-core/src/worktree.rs:189-213`):
Returns raw strings from config/env/default without canonicalization. Relative paths are joined to `project_root` but `..` segments are preserved (e.g., `/home/user/myproject/../myproject-worktrees`).

**Fix pattern — canonicalize-what-exists-append-leaf:**
```rust
fn canonicalize_best_effort(path: PathBuf) -> PathBuf {
    // If the full path exists, canonicalize it directly
    if path.exists() {
        return std::fs::canonicalize(&path).unwrap_or(path);
    }
    // Otherwise, canonicalize the parent and append the leaf
    if let (Some(parent), Some(leaf)) = (path.parent(), path.file_name()) {
        if let Ok(canonical_parent) = std::fs::canonicalize(parent) {
            return canonical_parent.join(leaf);
        }
    }
    // Fallback: return as-is (no existing ancestor to canonicalize)
    path
}
```

Apply this at the end of `resolve_worktree_dir()` before returning the final `PathBuf`.

**macOS edge case — `/tmp` symlink:** On macOS, `/tmp` → `/private/tmp` and `/var` → `/private/var`. This means `TempDir::new()` returns paths under `/var/folders/...` but `canonicalize()` resolves them to `/private/var/folders/...`. The `git worktree list --porcelain` output also uses the canonical path. This is exactly the mismatch being fixed — after this change, both sides will use canonical paths.

**Confidence:** HIGH — `canonicalize()` is the standard Rust API for this, and the parent+leaf pattern handles non-existent directories gracefully without side effects.

### WFIX-02: Default Branch Detection Returns `Result`

**Current code** (`crates/assay-core/src/worktree.rs:47-56`):
```rust
fn detect_default_branch(project_root: &Path) -> String {
    // ...falls back to "main" on failure
}
```

**Fix pattern:**
```rust
fn detect_default_branch(project_root: &Path) -> Result<String> {
    git_command(&["symbolic-ref", "refs/remotes/origin/HEAD"], project_root)
        .ok()
        .and_then(|output| {
            output
                .strip_prefix("refs/remotes/origin/")
                .map(|s| s.to_string())
        })
        .ok_or_else(|| AssayError::WorktreeGitFailed {
            cmd: "git symbolic-ref refs/remotes/origin/HEAD".to_string(),
            stderr: "Could not detect default branch. Run `git remote set-head origin --auto` \
                     or set `init.defaultBranch` in git config, or pass base_branch explicitly."
                .to_string(),
            exit_code: None,
        })
}
```

**Single callsite** (`crates/assay-core/src/worktree.rs:262-264`):
```rust
let base = base_branch
    .map(|s| s.to_string())
    .unwrap_or_else(|| detect_default_branch(project_root));
```
Changes to:
```rust
let base = match base_branch {
    Some(b) => b.to_string(),
    None => detect_default_branch(project_root)?,
};
```

**Git CLI behavior verified:**
- `git symbolic-ref refs/remotes/origin/HEAD` returns exit code 128 with `fatal: ref refs/remotes/origin/HEAD is not a symbolic ref` when no remote is configured or remote HEAD is not set.
- When configured: returns `refs/remotes/origin/main` (or whatever the default branch is).

**Error variant choice:** Reuse `WorktreeGitFailed` with a custom stderr message rather than adding a new variant. The error message contains the actionable guidance. This avoids growing the error enum for a single use case.

**Confidence:** HIGH — single callsite, clear transformation from infallible to fallible.

### WFIX-03: Prune Warnings via `WorktreeListResult`

**Current code** (`crates/assay-core/src/worktree.rs:294-296`):
```rust
pub fn list(project_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let _ = git_command(&["worktree", "prune"], project_root);
```

**New wrapper struct** (define in `crates/assay-core/src/worktree.rs`, NOT in `assay-types`):
```rust
/// Result of listing worktrees, including any non-fatal warnings.
pub struct WorktreeListResult {
    pub entries: Vec<WorktreeInfo>,
    pub warnings: Vec<String>,
}
```

This struct is internal to `assay-core` — it does not need `Serialize`/`Deserialize`/`JsonSchema` because callers destructure it and handle entries/warnings separately. Do NOT put it in `assay-types`.

**Fix pattern:**
```rust
pub fn list(project_root: &Path) -> Result<WorktreeListResult> {
    let mut warnings = Vec::new();

    // Prune stale entries — best-effort, failures become warnings
    if let Err(e) = git_command(&["worktree", "prune"], project_root) {
        warnings.push(format!("git worktree prune failed: {e}"));
    }

    // ... rest unchanged ...

    Ok(WorktreeListResult { entries, warnings })
}
```

**Confidence:** HIGH — straightforward wrapper struct, captures existing swallowed error.

## Don't Hand-Roll

- **Path canonicalization:** Use `std::fs::canonicalize()`. Do not implement manual symlink resolution or `..` collapsing.
- **Git CLI parsing:** Continue using the existing `git_command` + `parse_worktree_list` helpers. Do not use `git2` or other git libraries.
- **Error types:** Reuse `AssayError::WorktreeGitFailed` for the detection error. Do not create a new `DefaultBranchDetectionFailed` variant.

## Common Pitfalls

### Pitfall 1: `canonicalize()` Fails on Non-Existent Paths
`std::fs::canonicalize()` returns `Err` if the path does not exist. The worktree base directory may not exist yet (it's created by `create()`). The parent+leaf pattern handles this.

### Pitfall 2: macOS `/tmp` Symlink in Tests
`TempDir::new()` on macOS creates directories under `/var/folders/...` which canonicalizes to `/private/var/folders/...`. Existing tests already handle this — see `test_detect_main_worktree_from_linked` (line 886-888) which canonicalizes both sides before comparison. New tests for WFIX-01 must do the same.

### Pitfall 3: Existing `resolve_worktree_dir` Tests Use Non-Existent Paths
The current unit tests (lines 555-624) use fake paths like `/home/user/myproject` that don't exist on disk. After adding canonicalization, these tests need updating since `canonicalize()` will fail on non-existent parents, causing the fallback to return the path as-is. This is correct behavior — the tests just need to assert the un-canonicalized path (which is what happens when neither the path nor its parent exists).

### Pitfall 4: `WorktreeListResult` Breaks Three Callsites
All three `list()` callsites currently expect `Vec<WorktreeInfo>`. They must be updated to destructure the wrapper.

### Pitfall 5: Integration Test Assertions After WFIX-03
The integration test `test_create_list_status_cleanup` (line 699) calls `list(&root)` and asserts on the return. After WFIX-03, this needs `let result = list(&root)?; let entries = result.entries;`.

## Code Examples

### WFIX-01: Updated `resolve_worktree_dir` (full function)

```rust
pub fn resolve_worktree_dir(
    cli_override: Option<&str>,
    config: &Config,
    project_root: &Path,
) -> PathBuf {
    let raw = cli_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("ASSAY_WORKTREE_DIR").ok())
        .or_else(|| {
            config
                .worktree
                .as_ref()
                .map(|w| &w.base_dir)
                .filter(|d| !d.is_empty())
                .cloned()
        })
        .unwrap_or_else(|| format!("../{}-worktrees", config.project_name));

    let path = Path::new(&raw);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };

    // Canonicalize to resolve symlinks and `..` segments.
    // If the full path exists, canonicalize directly.
    // If not, canonicalize the parent and append the leaf.
    // Fallback: return as-is (no existing ancestor).
    if resolved.exists() {
        std::fs::canonicalize(&resolved).unwrap_or(resolved)
    } else if let (Some(parent), Some(leaf)) = (resolved.parent(), resolved.file_name()) {
        std::fs::canonicalize(parent)
            .map(|p| p.join(leaf))
            .unwrap_or(resolved)
    } else {
        resolved
    }
}
```

### WFIX-02: Updated `detect_default_branch` and Callsite

```rust
fn detect_default_branch(project_root: &Path) -> Result<String> {
    git_command(&["symbolic-ref", "refs/remotes/origin/HEAD"], project_root)
        .ok()
        .and_then(|output| {
            output
                .strip_prefix("refs/remotes/origin/")
                .map(|s| s.to_string())
        })
        .ok_or_else(|| AssayError::WorktreeGitFailed {
            cmd: "git symbolic-ref refs/remotes/origin/HEAD".to_string(),
            stderr: "Could not detect default branch. Run `git remote set-head origin --auto` \
                     or set `init.defaultBranch` in git config, or pass base_branch explicitly."
                .to_string(),
            exit_code: None,
        })
}
```

Callsite in `create()`:
```rust
let base = match base_branch {
    Some(b) => b.to_string(),
    None => detect_default_branch(project_root)?,
};
```

### WFIX-03: `WorktreeListResult` and Updated `list()`

```rust
/// Result of listing worktrees, including any non-fatal warnings.
pub struct WorktreeListResult {
    /// The worktree entries found.
    pub entries: Vec<WorktreeInfo>,
    /// Non-fatal warnings (e.g., prune failures).
    pub warnings: Vec<String>,
}

pub fn list(project_root: &Path) -> Result<WorktreeListResult> {
    let mut warnings = Vec::new();

    if let Err(e) = git_command(&["worktree", "prune"], project_root) {
        warnings.push(format!("git worktree prune failed: {e}"));
    }

    let output = git_command(&["worktree", "list", "--porcelain"], project_root)?;
    let raw = parse_worktree_list(&output);

    let mut entries: Vec<WorktreeInfo> = raw
        .into_iter()
        .filter_map(|wt| {
            let branch = wt.branch.as_deref()?;
            let slug = branch.strip_prefix("assay/")?;
            let base_branch = read_metadata(&wt.path).map(|m| m.base_branch);
            Some(WorktreeInfo {
                spec_slug: slug.to_string(),
                path: wt.path,
                branch: branch.to_string(),
                base_branch,
            })
        })
        .collect();

    entries.sort_by(|a, b| a.spec_slug.cmp(&b.spec_slug));
    Ok(WorktreeListResult { entries, warnings })
}
```

## Callsite Inventory

### `resolve_worktree_dir()` — 4 callsites (WFIX-01: no signature change, transparent fix)

| File | Line | Context |
|------|------|---------|
| `crates/assay-cli/src/commands/worktree.rs` | 135 | `resolve_dirs()` helper — used by create, status, cleanup |
| `crates/assay-mcp/src/server.rs` | 1981 | `worktree_create` MCP tool |
| `crates/assay-mcp/src/server.rs` | 2043 | `worktree_status` MCP tool |
| `crates/assay-mcp/src/server.rs` | 2085 | `worktree_cleanup` MCP tool |

**Impact:** None — signature is unchanged. All callers get canonical paths automatically.

### `detect_default_branch()` — 1 callsite (WFIX-02: signature change String → Result<String>)

| File | Line | Context |
|------|------|---------|
| `crates/assay-core/src/worktree.rs` | 264 | Inside `create()`, called when `base_branch` is `None` |

**Impact:** Only the `create()` function's `unwrap_or_else` changes to `?` propagation. `create()` already returns `Result`, so the error naturally propagates to CLI/MCP callers.

### `list()` — 3 callsites (WFIX-03: return type change Vec → WorktreeListResult)

| File | Line | Context | Required Change |
|------|------|---------|-----------------|
| `crates/assay-cli/src/commands/worktree.rs` | 173 | `handle_worktree_list` | Destructure: `let result = list(&root)?; let entries = result.entries;` — warnings ignored (CLI list doesn't show warnings currently) |
| `crates/assay-cli/src/commands/worktree.rs` | 366 | `handle_worktree_cleanup_all` | Destructure: `let result = list(root)?; let entries = result.entries;` — warnings ignored |
| `crates/assay-mcp/src/server.rs` | 2019 | `worktree_list` MCP tool | Destructure and include warnings in response. Wrap in a `WorktreeListResponse` struct with `entries` + `warnings` fields, matching the MCP convention. |

**MCP response pattern for warnings** (consistent with existing tools):
```rust
#[derive(Serialize)]
struct WorktreeListResponse {
    entries: Vec<WorktreeInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<String>,
}
```

## Test Inventory

### Existing Tests in `crates/assay-core/src/worktree.rs`

**Unit tests (`mod tests`):**
- `test_parse_worktree_list_normal` — parsing porcelain output
- `test_parse_worktree_list_empty` — empty input
- `test_parse_worktree_list_bare` — bare repo entry
- `test_parse_worktree_list_detached` — detached HEAD entry
- `test_resolve_worktree_dir_default` — default path generation
- `test_resolve_worktree_dir_config` — config override
- `test_resolve_worktree_dir_env_overrides_config` — env var precedence
- `test_resolve_worktree_dir_cli_overrides_all` — CLI precedence
- `test_resolve_worktree_dir_relative_resolved_against_root` — relative path resolution

**Integration tests (`mod integration_tests`):**
- `test_create_list_status_cleanup` — full lifecycle (needs WFIX-03 update)
- `test_create_nonexistent_spec_returns_spec_not_found`
- `test_create_duplicate_returns_worktree_exists`
- `test_cleanup_dirty_without_force_returns_worktree_dirty`
- `test_create_directory_based_spec`
- `test_status_missing_metadata_returns_none_ahead_behind`
- `test_read_write_metadata_roundtrip`
- `test_status_nonexistent_returns_not_found`
- `test_cleanup_nonexistent_returns_not_found`
- `test_detect_main_worktree_from_linked` — already uses `canonicalize()` for comparison
- `test_detect_main_worktree_from_main_returns_none`

### Test Infrastructure
- **Real git repos:** Integration tests use `tempfile::TempDir` + actual `git init` — no mocking.
- **Serial execution:** `resolve_worktree_dir` tests use `#[serial]` from `serial_test` crate because they manipulate environment variables.
- **Spec fixtures:** `setup_repo()` creates a full repo with an `auth-flow.toml` spec and initial commit.

### Tests Affected by Changes

**WFIX-01:** The 5 `resolve_worktree_dir` unit tests use non-existent paths (`/home/user/myproject`). Since `canonicalize()` will fail on these, the function falls back to the un-canonicalized path — these tests continue to pass without changes. Add a new integration test that uses a real `TempDir` to verify canonicalization happens.

**WFIX-02:** No existing test covers `detect_default_branch()` directly (it's private). Add integration tests:
1. `create()` without `base_branch` in a repo without remote → should error with actionable message
2. `create()` with explicit `base_branch` → should succeed regardless of remote state (bypass path)

**WFIX-03:** Update `test_create_list_status_cleanup` to destructure `WorktreeListResult`. Add:
1. A unit test that `list()` returns warnings when prune would fail (hard to trigger in integration tests since prune rarely fails in clean temp repos — may need to verify the struct shape only)

### New Tests Needed

| WFIX | Test | Type |
|------|------|------|
| 01 | `resolve_worktree_dir` with real `TempDir` + relative path resolves to canonical | Integration |
| 01 | `resolve_worktree_dir` with symlinked parent resolves through symlink | Integration |
| 02 | `create()` without `base_branch` in repo without remote returns actionable error | Integration |
| 02 | `create()` with explicit `base_branch` succeeds even without remote | Integration (already passes, just needs explicit assertion) |
| 03 | `list()` returns `WorktreeListResult` with empty warnings on success | Integration (update existing test) |
