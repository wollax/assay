# Phase 28: Worktree Manager — Research

**Completed:** 2026-03-09
**Confidence:** HIGH (all findings verified against codebase and git CLI behavior)

---

## Standard Stack

**Zero new workspace dependencies.** All git operations use `std::process::Command` calling the `git` CLI binary.

| Concern | Solution | Notes |
|---------|----------|-------|
| Git operations | `std::process::Command` → `git` CLI | No git2/gitoxide. Already used in gate evaluation (`assay_core::gate::evaluate`). |
| Config parsing | `toml` (workspace dep, already used) | Add `[worktree]` section to existing `Config` struct in `assay-types`. |
| Serialization | `serde` + `serde_json` (workspace deps) | For `--json` output and MCP tool responses. |
| Error handling | `thiserror` via `AssayError` enum | Add `WorktreeError`-category variants to existing `AssayError`. |
| CLI parsing | `clap` derive (workspace dep) | New `WorktreeCommand` sub-enum in `commands/worktree.rs`. |
| MCP tools | `rmcp` via `#[tool]` + `#[tool_router]` macros | Add methods to existing `AssayServer` in `assay-mcp`. |
| Interactive prompts | `std::io::stdin().read_line()` | No dialoguer/inquire — raw stdin for confirmation prompts. Zero dep constraint. |
| Temp dir for tests | `tempfile` (workspace dep) | For integration tests with real git repos. |

---

## Architecture Patterns

### Layer Responsibilities

```
assay-types    → WorktreeConfig, WorktreeInfo, WorktreeStatus (serializable structs)
assay-core     → worktree module: create/list/status/cleanup logic (calls git CLI)
assay-cli      → commands/worktree.rs: clap subcommands, human + JSON output
assay-mcp      → worktree_create, worktree_status, worktree_cleanup tools
```

### Core Module: `assay_core::worktree`

Single module file (`crates/assay-core/src/worktree/mod.rs` or `worktree.rs`) containing pure functions that:
1. Accept paths/config as arguments (no global state)
2. Shell out to `git` via `Command`
3. Return `Result<T, AssayError>` using the existing error type

**Key functions:**

```rust
pub fn create(project_root: &Path, spec_slug: &str, base_branch: &str, worktree_base: &Path) -> Result<WorktreeInfo>
pub fn list(project_root: &Path, worktree_base: &Path) -> Result<Vec<WorktreeInfo>>
pub fn status(project_root: &Path, worktree_path: &Path) -> Result<WorktreeStatus>
pub fn cleanup(project_root: &Path, worktree_path: &Path, delete_branch: bool) -> Result<()>
```

### Config Extension

Add to `assay_types::Config`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub worktree: Option<WorktreeConfig>,
```

```rust
pub struct WorktreeConfig {
    /// Base directory for worktrees. Relative paths resolved from project root.
    /// Default: "../<project-name>-worktrees/"
    #[serde(default = "default_worktree_base_dir")]
    pub base_dir: String,
}
```

**Config precedence (already decided):** CLI flag > env var `ASSAY_WORKTREE_DIR` > `config.toml` `[worktree].base_dir` > default sibling directory.

### CLI Command Structure

Following existing pattern (flat file per subcommand group):

```
commands/worktree.rs  — WorktreeCommand enum + pub(crate) fn handle()
```

```rust
#[derive(Subcommand)]
pub(crate) enum WorktreeCommand {
    Create { name: String, #[arg(long)] base: Option<String>, #[arg(long)] worktree_dir: Option<String>, #[arg(long)] json: bool },
    List { #[arg(long)] json: bool, #[arg(long)] worktree_dir: Option<String> },
    Status { name: String, #[arg(long)] json: bool, #[arg(long)] worktree_dir: Option<String> },
    Cleanup { name: Option<String>, #[arg(long)] all: bool, #[arg(long)] force: bool, #[arg(long)] json: bool, #[arg(long)] worktree_dir: Option<String> },
}
```

Wire into `main.rs` as:

```rust
Worktree { #[command(subcommand)] command: commands::worktree::WorktreeCommand },
```

### MCP Tool Surface

**Recommendation: 1:1 mirror** (3 tools: `worktree_create`, `worktree_status`, `worktree_cleanup`).

Rationale: `worktree_list` is less useful for agents (they know which spec they're working on). Status + create + cleanup cover the agent workflow. If list is needed, `worktree_status` without a specific spec could return all.

However, adding `worktree_list` as a 4th tool is low-cost and consistent with CLI. **Planner's call.**

---

## Git Worktree Commands Reference

### Create

```bash
git worktree add -b assay/<spec-slug> <path> <base-branch>
```

- Creates worktree at `<path>`, creates and checks out branch `assay/<spec-slug>` from `<base-branch>`.
- Fails if branch already exists (use `-B` to force-reset, but we should error instead for safety).
- Fails if path already exists.
- Exit code 0 on success, 128 on failure.

### List

```bash
git worktree list --porcelain
```

Porcelain output format (one record per worktree, blank line separator):

```
worktree /absolute/path
HEAD <sha>
branch refs/heads/<name>

worktree /another/path
HEAD <sha>
branch refs/heads/<other>
```

Bare worktrees show `bare` instead of `branch`. Detached worktrees show `detached` instead of `branch`.

**Filtering strategy:** Parse all worktrees, filter to those whose path starts with the configured worktree base directory AND whose branch matches `refs/heads/assay/`.

### Status (dirty/ahead/behind)

No single git command. Compose from:

```bash
# In the worktree directory:
git -C <worktree-path> status --porcelain    # empty = clean
git -C <worktree-path> rev-list --left-right --count HEAD...origin/<base>  # "ahead\tbehind"
git -C <worktree-path> rev-parse --abbrev-ref HEAD  # branch name
```

### Remove/Cleanup

```bash
git worktree remove <path>           # fails if dirty
git worktree remove --force <path>   # removes even if dirty
git branch -d assay/<spec-slug>      # delete the branch after removal
git branch -D assay/<spec-slug>      # force-delete (unmerged commits)
```

**Sequence for cleanup:**
1. Check if worktree is dirty (status --porcelain)
2. If dirty and interactive: prompt for confirmation
3. If dirty and non-interactive (no TTY or `--force`): fail with error (unless `--force`)
4. `git worktree remove [--force] <path>`
5. `git branch -D assay/<spec-slug>` (decision: always force-delete since worktree is being cleaned up)

### Default Branch Detection

```bash
git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's|refs/remotes/origin/||'
```

Falls back to `main` if the above fails (e.g., no remote configured).

---

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Git repository interaction | `std::process::Command` calling `git` CLI — do NOT implement git plumbing |
| Branch existence check | `git rev-parse --verify refs/heads/assay/<slug>` — do NOT scan .git/refs |
| Worktree enumeration | `git worktree list --porcelain` — do NOT walk filesystem |
| Dirty working tree check | `git -C <path> status --porcelain` — do NOT diff files manually |
| Interactive TTY detection | `std::io::stdin().is_terminal()` (stable since Rust 1.70, `IsTerminal` trait) — do NOT use libc/atty |
| Slug generation | Derive from spec filename stem (already the pattern in `load_spec_entry`) — do NOT parse spec title |
| Config resolution precedence | Build a simple `resolve_worktree_dir()` function with the 4-level fallback — do NOT thread it through every function |

---

## Common Pitfalls

### P1: Worktree path must be outside the main repo
Git worktrees cannot be nested inside the main working tree. The sibling directory default (`../project-worktrees/`) handles this correctly. **Verify** the resolved path is not under the project root.

### P2: Worktree creation in a bare repo vs normal repo
This project assumes a normal (non-bare) repo. No special handling needed, but the error message should be clear if git fails.

### P3: Branch name conflicts
`git worktree add -b assay/<slug>` fails if the branch exists. Two scenarios:
- Worktree already exists for this spec → detect and return idempotent success (or error, planner decides)
- Branch exists but worktree was manually deleted → `git worktree prune` first, then retry

### P4: Stale worktree entries
If a worktree directory is deleted outside of `git worktree remove`, git keeps a stale entry. Run `git worktree prune` before `list` operations to get accurate results.

### P5: Race conditions with concurrent worktree operations
Git's worktree lockfile (`<path>/.git` is a file, not a directory, in worktrees) provides some protection. For this phase (single-user CLI), no extra locking needed.

### P6: Relative vs absolute paths
`git worktree add` requires an absolute or relative-to-cwd path. Always canonicalize the worktree base directory before passing to git. Use `std::fs::canonicalize()` for existing paths, manual join for paths that don't exist yet.

### P7: Spec resolution from within a worktree (ORCH-07)
When running gates inside a worktree, specs must resolve from the parent project's `.assay/specs/` directory. The worktree shares the same `.git` but has its own working tree. **Solution:** The worktree has its own copy of the project files (it's a checkout), so `.assay/specs/` exists in the worktree. However, specs should come from the _parent project_ (the main worktree) to ensure consistency. This means `gate_run` needs to detect it's in a worktree and resolve specs from the main worktree's path.

**Detection:** Check if `.git` is a file (not directory) — worktrees have a `.git` file containing `gitdir: <path>`. Parse this to find the main repo, then resolve specs from there.

### P8: Non-interactive mode detection
`--force` bypasses prompts. For MCP tools and piped stdin, detect non-interactive mode via `std::io::stdin().is_terminal()` and fail safely (refuse destructive operations without `--force`).

### P9: Cleanup of worktree with uncommitted changes
The CONTEXT.md decision says: interactive confirmation prompt for dirty worktrees, fail safely in non-interactive mode. `git worktree remove` without `--force` already fails for dirty worktrees — leverage this and add our own prompt layer on top.

---

## Code Examples

### Shelling out to git

```rust
use std::process::Command;
use std::path::Path;

fn git_command(args: &[&str], cwd: &Path) -> Result<String, AssayError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| AssayError::WorktreeGit {
            cmd: format!("git {}", args.join(" ")),
            source: e,
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(AssayError::WorktreeGitFailed {
            cmd: format!("git {}", args.join(" ")),
            stderr,
            exit_code: output.status.code(),
        })
    }
}
```

### Parsing worktree list --porcelain

```rust
struct RawWorktree {
    path: PathBuf,
    head: String,
    branch: Option<String>,
}

fn parse_worktree_list(porcelain: &str) -> Vec<RawWorktree> {
    porcelain
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .filter_map(|block| {
            let mut path = None;
            let mut head = None;
            let mut branch = None;
            for line in block.lines() {
                if let Some(p) = line.strip_prefix("worktree ") {
                    path = Some(PathBuf::from(p));
                } else if let Some(h) = line.strip_prefix("HEAD ") {
                    head = Some(h.to_string());
                } else if let Some(b) = line.strip_prefix("branch ") {
                    branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
                }
            }
            Some(RawWorktree { path: path?, head: head?, branch })
        })
        .collect()
}
```

### Worktree create flow

```rust
pub fn create(
    project_root: &Path,
    spec_slug: &str,
    base_branch: &str,
    worktree_base: &Path,
) -> Result<WorktreeInfo> {
    let branch_name = format!("assay/{spec_slug}");
    let worktree_path = worktree_base.join(spec_slug);

    // Check if worktree already exists
    if worktree_path.exists() {
        // Either idempotent return or error — planner decides
    }

    // Ensure base directory exists
    std::fs::create_dir_all(worktree_base).map_err(|e| AssayError::io("creating worktree base dir", worktree_base, e))?;

    // Create worktree with new branch
    git_command(
        &["worktree", "add", "-b", &branch_name, &worktree_path.display().to_string(), base_branch],
        project_root,
    )?;

    Ok(WorktreeInfo {
        spec_slug: spec_slug.to_string(),
        path: worktree_path,
        branch: branch_name,
        base_branch: base_branch.to_string(),
    })
}
```

### Detecting worktree context (ORCH-07)

```rust
fn detect_main_worktree(cwd: &Path) -> Option<PathBuf> {
    let dot_git = cwd.join(".git");
    if dot_git.is_file() {
        // This is a linked worktree — .git is a file containing "gitdir: <path>"
        let content = std::fs::read_to_string(&dot_git).ok()?;
        let gitdir = content.strip_prefix("gitdir: ")?.trim();
        // gitdir points to .git/worktrees/<name> in the main repo
        let gitdir_path = if Path::new(gitdir).is_absolute() {
            PathBuf::from(gitdir)
        } else {
            cwd.join(gitdir).canonicalize().ok()?
        };
        // Navigate up: .git/worktrees/<name> → .git → parent (main worktree)
        let main_git_dir = gitdir_path.parent()?.parent()?;
        Some(main_git_dir.parent()?.to_path_buf())
    } else {
        None // Already in main worktree
    }
}
```

### TTY detection for interactive prompts

```rust
use std::io::IsTerminal;

fn confirm_cleanup(spec_slug: &str, is_dirty: bool) -> bool {
    if !std::io::stdin().is_terminal() {
        return false; // Non-interactive: refuse
    }
    if !is_dirty {
        return true; // Clean worktree: no confirmation needed
    }
    eprint!("Worktree '{spec_slug}' has uncommitted changes. Remove anyway? [y/N] ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap_or(0);
    matches!(input.trim(), "y" | "Y" | "yes" | "YES")
}
```

---

## Error Variants to Add

Add to `AssayError` in `crates/assay-core/src/error.rs`:

```rust
/// A git command failed to spawn (git not found, permission denied).
#[error("git command `{cmd}` failed to execute: {source}")]
WorktreeGit {
    cmd: String,
    source: std::io::Error,
},

/// A git command exited with non-zero status.
#[error("git command `{cmd}` failed (exit {exit_code:?}):\n{stderr}")]
WorktreeGitFailed {
    cmd: String,
    stderr: String,
    exit_code: Option<i32>,
},

/// Worktree already exists for this spec.
#[error("worktree already exists for spec `{spec_slug}` at {path}")]
WorktreeExists {
    spec_slug: String,
    path: PathBuf,
},

/// Worktree not found for the given spec.
#[error("no worktree found for spec `{spec_slug}`")]
WorktreeNotFound {
    spec_slug: String,
},

/// Worktree has uncommitted changes and cleanup was refused.
#[error("worktree `{spec_slug}` has uncommitted changes; use --force to override")]
WorktreeDirty {
    spec_slug: String,
},
```

---

## Types to Add (assay-types)

```rust
/// Configuration for worktree management.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorktreeConfig {
    /// Base directory for worktrees.
    /// Relative paths resolved from project root.
    /// Default: "../<project-name>-worktrees/"
    #[serde(default = "default_worktree_base_dir")]
    pub base_dir: String,
}

/// Information about a single worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInfo {
    pub spec_slug: String,
    pub path: PathBuf,
    pub branch: String,
}

/// Status of a worktree including dirty state and ahead/behind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeStatus {
    pub spec_slug: String,
    pub path: PathBuf,
    pub branch: String,
    pub head: String,
    pub dirty: bool,
    pub ahead: usize,
    pub behind: usize,
}
```

Note: The default for `base_dir` depends on `project_name`, so it cannot be a simple `fn default()`. Resolution strategy: when `base_dir` is empty/unset, compute it at resolution time using the project name from config.

---

## Testing Strategy

### Unit Tests (assay-core::worktree)
- `parse_worktree_list` with various porcelain outputs (normal, detached, bare, empty)
- Slug derivation from spec names
- Config resolution precedence (mock env vars)

### Integration Tests (assay-cli, requires git)
- Create a temp git repo with `git init`, add a spec, run `assay worktree create`
- Verify worktree exists at expected path with correct branch
- `assay worktree list` returns the created worktree
- `assay worktree status` shows clean/dirty correctly
- `assay worktree cleanup` removes worktree and branch
- `assay worktree cleanup --all` removes all worktrees
- Non-interactive cleanup of dirty worktree fails without `--force`
- Duplicate create handling (error or idempotent, depending on planner decision)

### MCP Integration Tests
- Follow existing pattern in `crates/assay-mcp/tests/mcp_handlers.rs`
- Test `worktree_create`, `worktree_status`, `worktree_cleanup` via `Parameters` struct

---

## Discretionary Recommendations

### Spec slug derivation
**Recommendation:** Use filename stem, identical to existing `load_spec_entry` slug parameter. The slug IS the spec identifier throughout the codebase. **Confidence: HIGH.**

### Worktree directory naming
**Recommendation:** `<worktree_base>/<spec-slug>/` — simple, one directory per spec. No prefixing or date stamping. **Confidence: HIGH.**

### Duplicate worktree handling
**Recommendation:** Return idempotent success if the worktree already exists at the expected path with the correct branch. Return error if a different worktree exists for the same spec. This matches the agent workflow where retries are common. **Confidence: MEDIUM** — error-on-duplicate is simpler and equally valid.

### `worktree list` display format
**Recommendation:** Compact table, matching `gate history` style:
```
  Spec          Branch              Path                              Status
  ─────         ──────              ────                              ──────
  auth-flow     assay/auth-flow     ../project-worktrees/auth-flow    clean
  payments      assay/payments      ../project-worktrees/payments     dirty
```
**Confidence: HIGH.**

### MCP tool granularity
**Recommendation:** 4 individual tools: `worktree_create`, `worktree_list`, `worktree_status`, `worktree_cleanup`. 1:1 with CLI subcommands. Agents benefit from explicit, single-purpose tools. **Confidence: HIGH.**

### Git ref pruning strategy
**Recommendation:** Run `git worktree prune` before `list` operations (cheap, idempotent). On individual cleanup, delete the branch. On bulk cleanup (`--all`), delete all branches. No separate prune command needed. **Confidence: HIGH.**

---

*Phase: 28-worktree-manager*
*Research completed: 2026-03-09*
