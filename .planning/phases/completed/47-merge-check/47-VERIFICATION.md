# Phase 47 Verification Report

**Phase:** 47 — Merge Check
**Goal:** Standalone conflict detection via `git merge-tree --write-tree` with zero side effects
**Status:** passed
**Score:** 16/16

---

## Plan 01: Core Domain

### 1. Types exist with serde + schemars derives
**PASS**

`crates/assay-types/src/merge.rs` defines all required types:
- `MergeCheck` — `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]`
- `MergeConflict` — same derives
- `ConflictType` — same derives
- `FileChange` — same derives
- `ChangeType` — same derives

All re-exported from `crates/assay-types/src/lib.rs` line 42:
`pub use merge::{ChangeType, ConflictType, FileChange, MergeCheck, MergeConflict};`

### 2. merge_check() function signature
**PASS**

`crates/assay-core/src/merge.rs` line 227:
```rust
pub fn merge_check(
    project_root: &Path,
    base: &str,
    head: &str,
    max_conflicts: Option<u32>,
) -> Result<MergeCheck>
```

### 3. git merge-tree --write-tree is used (no index or working tree mutation)
**PASS**

`crates/assay-core/src/merge.rs` lines 281–284:
```rust
let (mt_stdout, mt_stderr, mt_exit) = git_raw(
    &["merge-tree", "--write-tree", &base_sha, &head_sha],
    project_root,
)?;
```
No `git checkout`, `git merge`, or index-mutating commands are used anywhere in the module.

### 4. Exit code 1 disambiguated by checking stdout for valid 40-char hex OID (P1)
**PASS**

`is_valid_tree_oid()` at line 68 checks `first_line.len() >= 40 && first_line[..40].chars().all(|c| c.is_ascii_hexdigit())`.

Used at lines 296–308: if stdout does not begin with a valid OID, the output is treated as an error regardless of exit code. The clean/conflict distinction is then made purely on `mt_exit == Some(0)` only after OID validation passes.

### 5. Clean merges include file list via follow-up git diff-tree (P2)
**PASS**

Lines 313–320: on `clean == true`, the tree OID is extracted from stdout and passed to `git diff-tree -r --name-status <base_sha> <tree_oid>`. Results are parsed into `Vec<FileChange>` and included in the returned `MergeCheck`.

### 6. Conflict type parsing handles both 'content' and 'contents' (P3)
**PASS**

`parse_conflict_type()` at line 78:
```rust
"content" | "contents" => ConflictType::Content,
```
Covered by unit test `test_parse_conflict_type_content`.

### 7. Unrelated histories (exit 128) produce a clear error (P4)
**PASS**

Lines 287–293:
```rust
if mt_exit == Some(128) {
    return Err(AssayError::WorktreeGitFailed {
        cmd: format!("git merge-tree --write-tree {base_sha} {head_sha}"),
        stderr: mt_stderr,
        exit_code: mt_exit,
    });
}
```
Returns an `Err` immediately — never reaches conflict parsing.

### 8. Ref resolution uses git rev-parse without --verify (P5)
**PASS**

Lines 237 and 244 call `git_command(&["rev-parse", base], ...)` and `git_command(&["rev-parse", head], ...)` — `--verify` is absent, supporting relative refs like `HEAD~3`, `@{upstream}`, etc.

### 9. MergeCheckRefError variant with actionable message
**PASS**

`crates/assay-core/src/error.rs` lines 343–348:
```rust
/// One or both git refs failed to resolve.
#[error("merge check ref error: {message}")]
MergeCheckRefError {
    /// Actionable message describing which ref(s) failed and why.
    message: String,
},
```
Both failed-ref errors are accumulated and joined with `"; "` before being returned as this variant.

### 10. All unit tests pass
**PASS**

`cargo test --workspace -- merge` reports **16 passed, 0 failed**.

Tests verified:
- `test_parse_conflict_type_content` — both spellings
- `test_parse_conflict_type_variants` — all named variants
- `test_parse_conflict_type_unknown` — `Other(...)` fallback
- `test_parse_change_type` — A/M/D and invalid inputs
- `test_parse_ahead_behind` — valid and malformed inputs
- `test_is_valid_tree_oid` — valid OID, short, invalid chars, empty, error message
- `test_parse_file_changes` — 3-line output
- `test_parse_conflicts_from_stdout` — content conflict
- `test_parse_conflicts_modify_delete` — modify/delete conflict
- `test_extract_path_merge_conflict_in` — "Merge conflict in <path>" pattern
- `test_extract_path_deleted_in` — "<path> deleted in ..." pattern
- MCP handler tests: `merge_check_invalid_ref_returns_domain_error`, `merge_check_self_merge_is_clean`

---

## Plan 02: MCP Tool

### 1. merge_check MCP tool exists on AssayServer
**PASS**

`crates/assay-mcp/src/server.rs` line 2149: `pub async fn merge_check(...)` decorated with `#[tool(...)]`. Listed in module doc comment at line 18.

### 2. Accepts base (required), head (required), max_conflicts (optional, default 20)
**PASS**

`MergeCheckParams` at lines 280–300:
- `base: String` — required
- `head: String` — required
- `max_conflicts: Option<u32>` — optional, `#[serde(default)]`; default of 20 applied in core at `max_conflicts.unwrap_or(20)`

### 3. Returns serialized MergeCheck JSON on success
**PASS**

Lines 2165–2170: `serde_json::to_string(&result)` → `Content::text(json)` → `CallToolResult::success(...)`.

Verified by MCP handler test `merge_check_self_merge_is_clean` which asserts `json["clean"] == true`.

### 4. Returns domain_error on ref resolution failure, git version error, or other AssayError
**PASS**

Lines 2161–2162:
```rust
Err(e) => return Ok(domain_error(&e)),
```
All `AssayError` variants — including `MergeCheckRefError`, `GitVersionTooOld`, and `WorktreeGitFailed` — are routed through `domain_error()`.

Verified by MCP handler test `merge_check_invalid_ref_returns_domain_error` which asserts `result.is_error == true` and that the error text contains the invalid ref name.

### 5. Tool description documents read-only, zero side effects
**PASS**

`#[tool(description = "...")]` at line 2147:
> "Uses `git merge-tree --write-tree` — read-only with zero side effects (no index mutation, no working tree changes)."

### 6. Works without requiring an Assay project context
**PASS**

The handler calls only `resolve_cwd()` (→ `std::env::current_dir()`). It does not call `load_config()`. No `.assay/` directory or config file is required.

---

## Summary

All 16 must-haves verified against actual source code. Tests run green. Phase 47 is complete.
