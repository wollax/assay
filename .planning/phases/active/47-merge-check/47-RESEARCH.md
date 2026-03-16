# Phase 47: Merge Check — Research

**Completed:** 2026-03-16
**Confidence:** HIGH (all findings verified against git 2.50.1 CLI behavior and official docs)

---

## Standard Stack

**Zero new workspace dependencies.** All git operations use `std::process::Command` calling the `git` CLI binary. Follows the exact pattern established in `crates/assay-core/src/worktree.rs`.

| Concern | Solution | Notes |
|---------|----------|-------|
| Git operations | `std::process::Command` → `git` CLI | Same pattern as worktree module. No git2/gitoxide. |
| Conflict detection | `git merge-tree --write-tree` | Available since Git 2.38 (Oct 2022). Zero side effects. |
| File listing (clean) | `git diff-tree -r --name-status` | merge-tree only outputs tree OID for clean merges; diff-tree extracts file changes. |
| Fast-forward detect | `git merge-base --is-ancestor` | Exit 0 = ancestor (fast-forward possible), exit 1 = not ancestor. |
| Merge base | `git merge-base <base> <head>` | Returns common ancestor SHA. |
| Ahead/behind | `git rev-list --left-right --count <base>...<head>` | Tab-separated: `<ahead>\t<behind>`. Same approach as worktree status. |
| Ref resolution | `git rev-parse <ref>` | Resolves branches, tags, raw SHAs, relative refs to full SHA. |
| Serialization | `serde` + `schemars` (workspace deps) | For MCP tool response types in `assay-types`. |
| Error handling | `thiserror` via `AssayError` enum | Reuse existing `WorktreeGit` and `WorktreeGitFailed` variants, or add `MergeCheck`-specific variants. |

---

## Architecture Patterns

### Layer Responsibilities

```
assay-types    → MergeCheck, MergeConflict, ConflictType, FileChange enums/structs
assay-core     → merge_check module: conflict detection logic (calls git CLI)
assay-mcp      → merge_check tool method on AssayServer
```

### Core Module: `assay_core::merge`

New module file `crates/assay-core/src/merge.rs` containing pure functions:

```rust
/// Perform a merge check between two refs with zero side effects.
pub fn merge_check(
    project_root: &Path,
    base: &str,
    head: &str,
    max_conflicts: Option<u32>,
) -> Result<MergeCheck>
```

### Git Command Sequence

The merge_check function executes these git commands in order:

1. **Validate refs** (parallel): `git rev-parse <base>` and `git rev-parse <head>` — resolve to SHAs, report both errors if both fail
2. **Merge base**: `git merge-base <base_sha> <head_sha>` — get common ancestor
3. **Fast-forward**: `git merge-base --is-ancestor <base_sha> <head_sha>` — check if head descends from base
4. **Ahead/behind**: `git rev-list --left-right --count <head_sha>...<base_sha>` — commit divergence counts
5. **Merge tree**: `git merge-tree --write-tree <base_sha> <head_sha>` — the actual conflict check
6. **File list (clean only)**: `git diff-tree -r --name-status <base_sha> <merge_tree_oid>` — extract changed files

Steps 2-4 can be combined or run after step 1. Step 6 only runs when step 5 exits 0 (clean merge).

### Reuse `git_command` Pattern

The existing `worktree.rs` has a private `git_command` helper. Extract or duplicate this pattern for the merge module:

```rust
fn git_command(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| AssayError::WorktreeGit { ... })?;
    // check output.status.success(), return stdout or stderr error
}
```

Consider extracting to a shared `crate::git` helper module if this is the second consumer. Alternatively, duplicate (it's 20 lines) and refactor later.

---

## `git merge-tree --write-tree` Output Format

### Verified Output Structure (Confidence: HIGH)

**Clean merge (exit code 0):**
```
<40-char tree OID>\n
```
That's it. Just the tree SHA on one line. No file listing, no messages.

**Conflicted merge (exit code 1):**
```
<40-char tree OID>\n
<mode> <object-oid> <stage>\t<filename>\n
<mode> <object-oid> <stage>\t<filename>\n
...\n
\n
<informational messages>\n
```

Sections:
1. **Line 1**: Tree OID (always present, even with conflicts — the tree contains conflict markers)
2. **Conflicted file info**: Lines with format `<mode> <oid> <stage>\t<path>`. Stage 1=base, 2=ours, 3=theirs. Multiple lines per conflicted file (one per stage).
3. **Blank line separator**
4. **Informational messages**: Human-readable conflict descriptions like `CONFLICT (content): Merge conflict in file.txt`

### `-z` Flag Output (Machine Parseable)

With `-z`, NUL characters replace newlines:
- Tree OID terminated by NUL (not newline)
- Conflicted file entries: `<mode> <oid> <stage>\t<path>\0` (tab between stage and path, NUL terminates)
- Empty separator (double NUL)
- Info messages: `<num-paths>\0<path1>\0...<pathN>\0<conflict-type>\0<conflict-message>\n\0`

**Recommendation: Do NOT use `-z`.** The standard (non-`-z`) output is simpler to parse for our use case. The `-z` format is designed for `--stdin` batch mode and adds complexity. Standard output is sufficient because:
- We control the git invocation (no filenames with special chars in conflict paths)
- We parse structured data (conflict info section), not free-form text
- Conflict type is extractable from the informational messages via regex

### `--name-only` Flag

Simplifies conflicted file info to just filenames (no mode/oid/stage). Useful if we only need conflict file paths, but we lose the stage information needed for rich conflict entries.

**Recommendation: Use default output (no `--name-only`).** Parse the full `<mode> <oid> <stage>\t<path>` lines to build rich conflict entries with stage information.

### Exit Codes (Verified)

| Exit Code | Meaning |
|-----------|---------|
| 0 | Clean merge (no conflicts) |
| 1 | Merge has conflicts (stdout contains conflict info) |
| 128 | Error (invalid refs, no common ancestor, git error). Stderr has message, stdout is empty. |

**Critical distinction:** Exit code 1 for conflicts vs 128 for errors. When exit code is 1, stdout contains valid output. When exit code is 128, stdout is empty and stderr has the error. Some edge cases (like invalid refs) also produce exit code 1 with empty stdout and stderr message — differentiate by checking if stdout starts with a 40-char hex OID.

**Verified:** Invalid ref produces stderr `merge-tree: <ref> - not something we can merge` with empty stdout (but exit code varies by git version — observed exit 1 on git 2.50.1 for invalid refs, contradicting docs that say "something other than 0 or 1"). **Safest approach: check stdout for valid tree OID presence rather than relying solely on exit code.**

---

## Conflict Type Parsing

### Conflict Types from Informational Messages

The conflict type is embedded in the informational messages section. Pattern: `CONFLICT (<type>): <description>`.

Verified conflict type strings (from git source and testing):

| Git Output String | Suggested Enum Variant | Description |
|-------------------|----------------------|-------------|
| `CONFLICT (content)` | `Content` | Both sides modified same file differently |
| `CONFLICT (contents)` | `Content` | Alternate spelling (seen in `-z` output!) |
| `CONFLICT (rename/delete)` | `RenameDelete` | One side renamed, other deleted |
| `CONFLICT (rename/rename)` | `RenameRename` | Both sides renamed same file differently |
| `CONFLICT (modify/delete)` | `ModifyDelete` | One side modified, other deleted |
| `CONFLICT (add/add)` | `AddAdd` | Both sides added same filename |
| `CONFLICT (file/directory)` | `FileDirectory` | One side has file, other has directory at same path |
| `CONFLICT (binary)` | `Binary` | Binary file modified on both sides |
| `CONFLICT (submodule)` | `Submodule` | Submodule conflict |
| `Auto-merging` | (not a conflict) | Informational — git auto-resolved |

**Important: `-z` output uses `CONFLICT (contents)` (plural) while non-`-z` uses `CONFLICT (content)` (singular).** Handle both spellings.

### Parsing Strategy

For non-`-z` output, parse informational messages with regex:

```rust
// Extract conflict type from message line
let re = Regex::new(r"CONFLICT \(([^)]+)\)").unwrap();
// Match groups: "content", "rename/delete", "modify/delete", etc.
```

Map the extracted string to a `ConflictType` enum. Unknown types map to a `ConflictType::Other(String)` variant to be forward-compatible with future git versions.

### Correlating Conflicts with File Paths

Two data sources to correlate:
1. **Conflicted file info** section: gives `(mode, oid, stage, path)` tuples — tells you WHICH files conflict and at which stages
2. **Informational messages** section: gives conflict TYPE and human description — tells you WHAT KIND of conflict

Strategy:
- Parse conflicted file info to get the set of conflicted paths (group by path, stages present)
- Parse informational messages to extract conflict type per path
- Match by path to build rich `MergeConflict { path, conflict_type, stages }` entries

For simple cases (content conflicts), each file appears in both sections. For rename conflicts, the informational message mentions old and new paths.

---

## Fast-Forward Detection

**Command:** `git merge-base --is-ancestor <base> <head>`

| Exit Code | Meaning |
|-----------|---------|
| 0 | `base` IS an ancestor of `head` → fast-forward possible |
| 1 | `base` is NOT an ancestor of `head` → merge commit required |
| 128 | Error (invalid refs) |

**Note on direction:** "Fast-forward" means head is ahead of base — i.e., base is an ancestor of head. If base is `main` and head is `feature`, this checks whether `feature` can be fast-forwarded onto `main`.

---

## Ahead/Behind Counts

**Command:** `git rev-list --left-right --count <head>...<base>`

Output: `<ahead>\t<behind>` (tab-separated integers)

- `ahead` = commits in `head` not in `base` (left side of `...`)
- `behind` = commits in `base` not in `head` (right side of `...`)

This is the same approach used in `worktree.rs` (line ~419). Reuse the same parsing logic.

**Edge case:** If merge-base doesn't exist (unrelated histories), rev-list will fail. Handle by setting ahead/behind to None and including a warning.

---

## Ref Resolution & Validation

**Command:** `git rev-parse <ref>`

Resolves any valid git ref to a full SHA:
- Branch names: `main` → `271f2ed...`
- Tags (lightweight and annotated): `v1.0` → `04a3c66...`
- Raw SHAs: pass-through
- Relative refs: `HEAD~3`, `branch-a~0`

**Upfront validation approach:**
1. Run `git rev-parse <base>` and `git rev-parse <head>`
2. If either fails, collect both errors and return them together
3. On success, use resolved SHAs for all subsequent commands

**Note:** `git rev-parse --verify` is stricter but doesn't handle relative refs like `HEAD~3`. Use plain `git rev-parse` without `--verify` for maximum ref compatibility.

---

## Git Version Detection

### Minimum Required Version

`git merge-tree --write-tree` was introduced in **Git 2.38** (October 2022).

### Detection Strategy

```rust
fn check_git_version(project_root: &Path) -> Result<()> {
    let output = git_command(&["--version"], project_root)?;
    // output: "git version 2.50.1" or "git version 2.50.1 (Apple Git-155)"
    // Parse major.minor, check >= 2.38
}
```

**Version string format variations:**
- `git version 2.50.1`
- `git version 2.50.1 (Apple Git-155)` (macOS)
- `git version 2.38.0.windows.1` (Windows/Git for Windows)

Parse with: split on space, take element at index 2, split on `.`, compare first two segments.

**When to check:** Check once at tool invocation time (not at server startup). Cache the result if needed, but a single `git --version` call is fast (~2ms).

**Error message:** If version < 2.38, return a clear error: `"git merge-tree --write-tree requires Git 2.38+, found {version}. Please upgrade git."`.

---

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Git object manipulation | `git merge-tree --write-tree` (not manual three-way merge) |
| Conflict detection | Parse merge-tree output (not diff3 algorithm) |
| Rename detection | Built into merge-tree (not custom rename detection) |
| Merge base computation | `git merge-base` (not manual ancestor walk) |
| Ahead/behind counting | `git rev-list --left-right --count` (not manual graph traversal) |
| File change listing | `git diff-tree` (not tree comparison logic) |
| Ref resolution | `git rev-parse` (not manual ref lookup) |
| Version parsing | Simple string split (not semver crate — only need major.minor) |

---

## Common Pitfalls

### P1: Exit code 1 means TWO different things
Exit code 1 from `git merge-tree` means "conflicts detected" — BUT invalid refs can also produce exit code 1 in some git versions. **Always check stdout for a valid 40-char hex OID** as the first line. If stdout is empty or doesn't start with a hex OID, treat as an error regardless of exit code.

### P2: Clean merges have NO file listing
`git merge-tree --write-tree` only outputs the tree OID for clean merges. To get the list of changed files (required by CONTEXT.md Decision 3), use `git diff-tree -r --name-status <base_commit> <merge_tree_oid>` as a follow-up command.

### P3: Conflict type spelling varies
The `-z` output uses `CONFLICT (contents)` (plural) while non-`-z` output uses `CONFLICT (content)` (singular). If parsing conflict types, normalize both spellings to the same enum variant.

### P4: Unrelated histories = exit 128, not 1
When two refs share no common ancestor, `git merge-tree` exits with code 128 and `fatal: refusing to merge unrelated histories`. This is an error, not a conflict result. Handle separately from conflict exit code 1.

### P5: `git rev-parse --verify` rejects relative refs
`git rev-parse --verify HEAD~3` fails because `--verify` expects a single object name. Use `git rev-parse` without `--verify` to support the full range of ref formats (relative refs, reflog entries, etc.).

### P6: merge-base can fail for unrelated refs
`git merge-base <a> <b>` exits non-zero if there's no common ancestor. Handle this gracefully: set `merge_base_sha` to None (or empty) and `ahead`/`behind` to None.

### P7: diff-tree needs commit, not tree, for base
`git diff-tree -r --name-status <base> <tree_oid>` works when `<base>` is a commit (git auto-dereferences to its tree). No need to explicitly dereference with `^{tree}`.

### P8: Informational messages contain newlines
Some git conflict messages span multiple lines (e.g., rename conflict descriptions). When parsing the informational messages section, don't assume one conflict per line. Parse by `CONFLICT (` prefix to find conflict boundaries.

### P9: Stage numbers in conflicted file info
Stages: 1=common ancestor (base), 2=ours (first arg), 3=theirs (second arg). A modify/delete conflict may only have stages 1 and 2 (no stage 3 if the file was deleted in theirs).

---

## Code Examples

### Parsing merge-tree stdout (non-`-z`)

```rust
fn parse_merge_tree_output(stdout: &str, exit_code: i32) -> ParsedMergeTree {
    let mut lines = stdout.lines();

    // Line 1: tree OID (always present for valid merges)
    let tree_oid = lines.next().unwrap_or("").to_string();
    if tree_oid.len() != 40 || !tree_oid.chars().all(|c| c.is_ascii_hexdigit()) {
        // Not a valid merge-tree output — treat as error
        return ParsedMergeTree::Error;
    }

    if exit_code == 0 {
        return ParsedMergeTree::Clean { tree_oid };
    }

    // Remaining lines: conflicted file info, then blank line, then messages
    let mut conflict_entries = Vec::new();
    let mut messages = Vec::new();
    let mut in_messages = false;

    for line in lines {
        if line.is_empty() {
            in_messages = true;
            continue;
        }
        if in_messages {
            messages.push(line.to_string());
        } else {
            // Parse: "<mode> <oid> <stage>\t<path>"
            if let Some((meta, path)) = line.split_once('\t') {
                let parts: Vec<&str> = meta.split_whitespace().collect();
                if parts.len() == 3 {
                    conflict_entries.push(ConflictFileEntry {
                        mode: parts[0].to_string(),
                        oid: parts[1].to_string(),
                        stage: parts[2].parse().unwrap_or(0),
                        path: path.to_string(),
                    });
                }
            }
        }
    }

    ParsedMergeTree::Conflicted { tree_oid, conflict_entries, messages }
}
```

### Extracting conflict type from messages

```rust
fn extract_conflict_type(message: &str) -> ConflictType {
    if let Some(start) = message.find("CONFLICT (") {
        let after = &message[start + 10..];
        if let Some(end) = after.find(')') {
            return match &after[..end] {
                "content" | "contents" => ConflictType::Content,
                "rename/delete" => ConflictType::RenameDelete,
                "rename/rename" => ConflictType::RenameRename,
                "modify/delete" => ConflictType::ModifyDelete,
                "add/add" => ConflictType::AddAdd,
                "file/directory" => ConflictType::FileDirectory,
                "binary" => ConflictType::Binary,
                "submodule" => ConflictType::Submodule,
                other => ConflictType::Other(other.to_string()),
            };
        }
    }
    ConflictType::Unknown
}
```

### Git version check

```rust
fn check_git_version(project_root: &Path) -> Result<()> {
    let version_str = git_command(&["--version"], project_root)?;
    // "git version 2.50.1" or "git version 2.50.1 (Apple Git-155)"
    let parts: Vec<&str> = version_str.split_whitespace().collect();
    let version = parts.get(2).ok_or_else(|| /* error */)?;
    let segments: Vec<u32> = version.split('.')
        .take(2)
        .filter_map(|s| s.parse().ok())
        .collect();
    if segments.len() >= 2 && (segments[0] > 2 || (segments[0] == 2 && segments[1] >= 38)) {
        Ok(())
    } else {
        Err(AssayError::/* MergeCheckVersionError */{ found: version.to_string() })
    }
}
```

### diff-tree for clean merge file listing

```rust
fn list_changed_files(project_root: &Path, base_sha: &str, tree_oid: &str) -> Result<Vec<FileChange>> {
    let output = git_command(
        &["diff-tree", "-r", "--name-status", base_sha, tree_oid],
        project_root,
    )?;
    output.lines().filter_map(|line| {
        let (status, path) = line.split_once('\t')?;
        let change_type = match status {
            "A" => ChangeType::Added,
            "M" => ChangeType::Modified,
            "D" => ChangeType::Deleted,
            _ => ChangeType::Other(status.to_string()),
        };
        Some(FileChange { path: path.to_string(), change_type })
    }).collect()
}
```

---

## Performance Characteristics

- `git merge-tree --write-tree` operates on the object store, not the working tree. No file I/O to disk beyond git's object database.
- For large repos, performance is proportional to the number of changed files between the two branches, NOT the total repo size.
- The `--quiet` flag allows early exit on first conflict and skips creating merge objects — useful for a "has conflicts?" boolean check, but we need the full output so don't use `--quiet`.
- The `max_conflicts` cap (from CONTEXT.md Decision 1) is applied AFTER parsing, not during git execution. Git outputs all conflicts regardless.
- Total command sequence (6 git invocations) adds ~10-50ms overhead for a typical merge check. The merge-tree itself dominates for large changesets.

---

## Type Design Recommendations

```rust
// In assay-types

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MergeCheck {
    pub clean: bool,
    pub base_sha: String,
    pub head_sha: String,
    pub merge_base_sha: Option<String>,
    pub fast_forward: bool,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,       // populated for clean merges
    pub conflicts: Vec<MergeConflict>, // populated for conflicted merges
    pub truncated: Option<u32>,        // "and N more" when max_conflicts hit
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MergeConflict {
    pub path: String,
    pub conflict_type: ConflictType,
    pub message: String, // raw git conflict message
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ConflictType {
    Content,
    RenameDelete,
    RenameRename,
    ModifyDelete,
    AddAdd,
    FileDirectory,
    Binary,
    Submodule,
    Other(String),
}
```

---

## Error Variants Needed

Add to `AssayError`:

```rust
/// Git version too old for merge-tree --write-tree.
#[error("git merge-tree --write-tree requires Git 2.38+, found {version}")]
GitVersionTooOld { version: String },

/// Merge check ref resolution failed.
#[error("merge check failed: {message}")]
MergeCheckRefError { message: String },
```

Alternatively, reuse existing `WorktreeGitFailed` for git command failures (it's generic enough) and only add a version-specific variant.
