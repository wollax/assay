//! Merge check: conflict detection between git refs with zero side effects.
//!
//! Uses `git merge-tree --write-tree` to detect conflicts without mutating
//! the index or working tree. Follows the same `std::process::Command` pattern
//! as the `worktree` module.

use std::path::Path;
use std::process::Command;

use assay_types::{
    ChangeType, ConflictMarker, ConflictScan, ConflictType, FileChange, MarkerType, MergeCheck,
    MergeConflict, MergeExecuteResult,
};

use crate::error::{AssayError, Result};

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Execute a git command and return (stdout, stderr, exit_code) without checking success.
///
/// Used when we need to inspect the exit code ourselves (e.g., merge-tree).
pub(crate) fn git_raw(args: &[&str], cwd: &Path) -> Result<(String, String, Option<i32>)> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| AssayError::WorktreeGit {
            cmd: format!("git {}", args.join(" ")),
            source: e,
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_string();
    Ok((stdout, stderr, output.status.code()))
}

/// Execute a git command and return stdout on success.
pub(crate) fn git_command(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| AssayError::WorktreeGit {
            cmd: format!("git {}", args.join(" ")),
            source: e,
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr)
            .trim_end()
            .to_string();
        Err(AssayError::WorktreeGitFailed {
            cmd: format!("git {}", args.join(" ")),
            stderr,
            exit_code: output.status.code(),
        })
    }
}

/// Check whether stdout starts with a valid 40-character hex OID.
fn is_valid_tree_oid(s: &str) -> bool {
    let first_line = s.lines().next().unwrap_or("");
    first_line.len() >= 40 && first_line[..40].chars().all(|c| c.is_ascii_hexdigit())
}

/// Parse a conflict type string from git merge-tree output.
///
/// Handles both "content" and "contents" spellings (git uses both).
fn parse_conflict_type(s: &str) -> ConflictType {
    match s {
        "content" | "contents" => ConflictType::Content,
        "rename/delete" => ConflictType::RenameDelete,
        "rename/rename" => ConflictType::RenameRename,
        "modify/delete" => ConflictType::ModifyDelete,
        "add/add" => ConflictType::AddAdd,
        "file/directory" => ConflictType::FileDirectory,
        "binary" => ConflictType::Binary,
        "submodule" => ConflictType::Submodule,
        other => ConflictType::Other(other.to_string()),
    }
}

/// Parse a change type letter from `git diff-tree --name-status`.
///
/// Renames (`R100`) are treated as modified (the new path) and copies (`C100`)
/// as added, since those are the most useful representations for merge output.
fn parse_change_type(s: &str) -> Option<ChangeType> {
    match s {
        "A" => Some(ChangeType::Added),
        "M" => Some(ChangeType::Modified),
        "D" => Some(ChangeType::Deleted),
        _ if s.starts_with('R') => Some(ChangeType::Modified),
        _ if s.starts_with('C') => Some(ChangeType::Added),
        _ => None,
    }
}

/// Parse ahead/behind from `git rev-list --left-right --count` output.
///
/// Expected format: `<ahead>\t<behind>`.
fn parse_ahead_behind(s: &str) -> (u32, u32) {
    let parts: Vec<&str> = s.split('\t').collect();
    if parts.len() == 2 {
        let ahead = parts[0].parse::<u32>().unwrap_or(0);
        let behind = parts[1].parse::<u32>().unwrap_or(0);
        (ahead, behind)
    } else {
        (0, 0)
    }
}

/// Parse conflict information from merge-tree informational messages.
///
/// Looks for lines matching `CONFLICT (<type>): <message>` and extracts
/// the conflict type and a file path from the message.
fn parse_conflicts(stdout: &str) -> Vec<MergeConflict> {
    let mut conflicts = Vec::new();

    // The informational messages section comes after a blank line separator
    // following the conflicted file info section.
    let mut in_messages = false;
    let mut past_first_line = false;

    for line in stdout.lines() {
        if !past_first_line {
            past_first_line = true;
            continue; // skip the tree OID line
        }

        if line.is_empty() {
            in_messages = true;
            continue;
        }

        if !in_messages {
            continue;
        }

        // Parse CONFLICT (<type>): <message>
        if let Some(start) = line.find("CONFLICT (") {
            let after = &line[start + 10..];
            if let Some(end) = after.find(')') {
                let type_str = &after[..end];
                let conflict_type = parse_conflict_type(type_str);

                // Extract message after "): "
                let message = if after.len() > end + 2 {
                    after[end + 2..].trim().to_string()
                } else {
                    line.to_string()
                };

                // Extract file path from message
                let path = extract_path_from_message(&message);

                conflicts.push(MergeConflict {
                    path,
                    conflict_type,
                    message: line[start..].to_string(),
                });
            }
        }
    }

    conflicts
}

/// Extract a file path from a conflict message.
///
/// Common patterns:
/// - "Merge conflict in <path>"
/// - "<path> deleted in ... and modified in ..."
fn extract_path_from_message(message: &str) -> String {
    // Pattern: "Merge conflict in <path>"
    if let Some(idx) = message.find("Merge conflict in ") {
        return message[idx + 18..].trim().to_string();
    }

    // Pattern: "<path> deleted in ..."
    if let Some(idx) = message.find(" deleted in ") {
        return message[..idx].trim().to_string();
    }

    // Pattern: "<path> renamed to ... in ..."
    if let Some(idx) = message.find(" renamed to ") {
        return message[..idx].trim().to_string();
    }

    // Fallback: use the whole message
    message.to_string()
}

/// Parse file changes from `git diff-tree -r --name-status` output.
///
/// Renames and copies emit three tab-separated fields (`R100\told\tnew`);
/// the new (last) path is used as the `FileChange.path`.
fn parse_file_changes(output: &str) -> Vec<FileChange> {
    output
        .lines()
        .filter_map(|line| {
            let (status, rest) = line.split_once('\t')?;
            let change_type = parse_change_type(status)?;
            // R/C lines have a third field: old-path\tnew-path — use the last path.
            let path = rest.rsplit_once('\t').map_or(rest, |(_, new)| new);
            Some(FileChange {
                path: path.to_string(),
                change_type,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform a merge check between two refs with zero side effects.
///
/// Uses `git merge-tree --write-tree` for conflict detection without mutating
/// the working tree or index. Returns a `MergeCheck` with conflict details,
/// file changes, and divergence metadata.
///
/// # Arguments
///
/// * `project_root` - Path to the git repository root.
/// * `base` - Base ref (branch, tag, SHA, or relative ref).
/// * `head` - Head ref (branch, tag, SHA, or relative ref).
/// * `max_conflicts` - Maximum number of conflicts to return (default: 20).
///   When exceeded, `truncated` is set to `true`.
pub fn merge_check(
    project_root: &Path,
    base: &str,
    head: &str,
    max_conflicts: Option<u32>,
) -> Result<MergeCheck> {
    let max = max_conflicts.unwrap_or(20) as usize;

    // Step 1: Resolve refs (no --verify, so relative refs like HEAD~3 work)
    let mut ref_errors = Vec::new();
    let base_sha = match git_command(&["rev-parse", base], project_root) {
        Ok(sha) => Some(sha),
        Err(e) => {
            ref_errors.push(format!("base ref '{base}': {e}"));
            None
        }
    };
    let head_sha = match git_command(&["rev-parse", head], project_root) {
        Ok(sha) => Some(sha),
        Err(e) => {
            ref_errors.push(format!("head ref '{head}': {e}"));
            None
        }
    };

    if !ref_errors.is_empty() {
        return Err(AssayError::MergeCheckRefError {
            message: ref_errors.join("; "),
        });
    }

    let base_sha = base_sha.unwrap();
    let head_sha = head_sha.unwrap();

    // Step 2: Merge base (may fail for unrelated histories, hence .ok())
    let merge_base_sha = git_command(&["merge-base", &base_sha, &head_sha], project_root).ok();

    // Step 3: Fast-forward detection
    let (_, _, ff_exit) = git_raw(
        &["merge-base", "--is-ancestor", &base_sha, &head_sha],
        project_root,
    )?;
    let fast_forward = ff_exit == Some(0);

    // Step 4: Ahead/behind (three-dot notation)
    let rev_range = format!("{head_sha}...{base_sha}");
    let (ahead, behind) = git_command(
        &["rev-list", "--left-right", "--count", &rev_range],
        project_root,
    )
    .map(|output| parse_ahead_behind(&output))
    .unwrap_or((0, 0));

    // Step 5: Merge tree (exit code 1 can mean conflicts or invalid refs)
    let (mt_stdout, mt_stderr, mt_exit) = git_raw(
        &["merge-tree", "--write-tree", &base_sha, &head_sha],
        project_root,
    )?;

    // Exit 128 = unrelated histories or fatal error
    if mt_exit == Some(128) {
        return Err(AssayError::WorktreeGitFailed {
            cmd: format!("git merge-tree --write-tree {base_sha} {head_sha}"),
            stderr: mt_stderr,
            exit_code: mt_exit,
        });
    }

    // Check stdout for valid tree OID to disambiguate exit code 1
    if !is_valid_tree_oid(&mt_stdout) {
        // Invalid output — treat as error regardless of exit code
        let msg = if mt_stderr.is_empty() {
            mt_stdout.clone()
        } else {
            mt_stderr.clone()
        };
        return Err(AssayError::WorktreeGitFailed {
            cmd: format!("git merge-tree --write-tree {base_sha} {head_sha}"),
            stderr: msg,
            exit_code: mt_exit,
        });
    }

    let clean = mt_exit == Some(0);

    if clean {
        // Step 6: Clean merge — get file list via diff-tree
        let tree_oid = mt_stdout.lines().next().unwrap_or("");
        let files = git_command(
            &["diff-tree", "-r", "--name-status", &base_sha, tree_oid],
            project_root,
        )
        .map(|output| parse_file_changes(&output))?;

        Ok(MergeCheck {
            clean: true,
            base_sha,
            head_sha,
            merge_base_sha,
            fast_forward,
            ahead,
            behind,
            files,
            conflicts: Vec::new(),
            truncated: false,
        })
    } else {
        // Conflicted merge — parse conflicts
        let all_conflicts = parse_conflicts(&mt_stdout);
        let truncated = all_conflicts.len() > max;
        let conflicts: Vec<MergeConflict> = all_conflicts.into_iter().take(max).collect();

        Ok(MergeCheck {
            clean: false,
            base_sha,
            head_sha,
            merge_base_sha,
            fast_forward,
            ahead,
            behind,
            files: Vec::new(),
            conflicts,
            truncated,
        })
    }
}

// ---------------------------------------------------------------------------
// Conflict marker scanning
// ---------------------------------------------------------------------------

/// Scan a string for standard git conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`).
///
/// Returns a `ConflictScan` with the markers found and their line numbers.
/// The `file` field on each marker is set to the provided `file_path`.
pub fn scan_conflict_markers(content: &str, file_path: &str) -> ConflictScan {
    let mut markers = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        let marker_type = if trimmed.starts_with("<<<<<<<") {
            Some(MarkerType::Ours)
        } else if trimmed.starts_with("=======") {
            Some(MarkerType::Separator)
        } else if trimmed.starts_with(">>>>>>>") {
            Some(MarkerType::Theirs)
        } else {
            None
        };

        if let Some(mt) = marker_type {
            markers.push(ConflictMarker {
                file: file_path.to_string(),
                line: (idx + 1) as u32,
                marker_type: mt,
            });
        }
    }

    let has_markers = !markers.is_empty();
    ConflictScan {
        has_markers,
        markers,
        truncated: false,
    }
}

/// Scan multiple files for conflict markers, capping at 100 files.
///
/// Reads each file relative to `dir` and scans for conflict markers.
/// If more than 100 files are provided, only the first 100 are scanned
/// and `truncated` is set to `true`.
pub fn scan_files_for_markers(dir: &Path, files: &[String]) -> ConflictScan {
    const MAX_FILES: usize = 100;
    let truncated = files.len() > MAX_FILES;
    let scan_files = if truncated {
        &files[..MAX_FILES]
    } else {
        files
    };

    let mut all_markers = Vec::new();

    for file_path in scan_files {
        let full_path = dir.join(file_path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let scan = scan_conflict_markers(&content, file_path);
            all_markers.extend(scan.markers);
        }
    }

    let has_markers = !all_markers.is_empty();
    ConflictScan {
        has_markers,
        markers: all_markers,
        truncated,
    }
}

// ---------------------------------------------------------------------------
// Merge execution
// ---------------------------------------------------------------------------

/// Execute `git merge --no-ff` and return a structured result.
///
/// On success, returns the merge commit SHA and changed files.
/// On conflict, runs `git merge --abort` and returns conflict details.
/// Checks for an in-progress merge before starting.
///
/// # Arguments
///
/// * `project_root` - Path to the git repository root.
/// * `branch` - Branch name to merge into the current HEAD.
/// * `message` - Commit message for the merge commit.
pub fn merge_execute(
    project_root: &Path,
    branch: &str,
    message: &str,
) -> Result<MergeExecuteResult> {
    // Check for in-progress merge
    let git_dir = project_root.join(".git");
    let merge_head = if git_dir.is_file() {
        // Worktree: read the gitdir path from the .git file
        let content = std::fs::read_to_string(&git_dir).map_err(|e| AssayError::Io {
            operation: "reading .git file".to_string(),
            path: git_dir.clone(),
            source: e,
        })?;
        let gitdir = content
            .trim()
            .strip_prefix("gitdir: ")
            .unwrap_or(content.trim());
        Path::new(gitdir).join("MERGE_HEAD")
    } else {
        git_dir.join("MERGE_HEAD")
    };

    if merge_head.exists() {
        return Err(AssayError::MergeExecuteError {
            branch: branch.to_string(),
            conflicting_files: Vec::new(),
            message: "a merge is already in progress (MERGE_HEAD exists)".to_string(),
        });
    }

    // Execute the merge
    let (_stdout, stderr, exit_code) =
        git_raw(&["merge", "--no-ff", "-m", message, branch], project_root)?;

    if exit_code == Some(0) {
        // Success — get the merge commit SHA
        let merge_sha = git_command(&["rev-parse", "HEAD"], project_root)?;

        // Get changed files via diff-tree against parents
        let files_output = git_command(
            &["diff-tree", "-r", "--name-status", "HEAD^1", "HEAD"],
            project_root,
        )
        .unwrap_or_default();

        let files_changed = parse_file_changes(&files_output);

        Ok(MergeExecuteResult {
            merge_sha: Some(merge_sha),
            files_changed,
            was_conflict: false,
            conflict_details: None,
        })
    } else {
        // Conflict — collect details before aborting
        // List conflicting files from git status
        let conflicting_files: Vec<String> =
            git_command(&["diff", "--name-only", "--diff-filter=U"], project_root)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();

        // Scan for conflict markers in the conflicting files
        let conflict_scan = scan_files_for_markers(project_root, &conflicting_files);

        // Abort the merge to leave the repo clean
        let (_, abort_stderr, abort_exit) = git_raw(&["merge", "--abort"], project_root)?;

        if abort_exit != Some(0) {
            return Err(AssayError::MergeExecuteError {
                branch: branch.to_string(),
                conflicting_files,
                message: format!(
                    "merge conflict detected and git merge --abort failed: {stderr}; abort stderr: {abort_stderr}"
                ),
            });
        }

        Ok(MergeExecuteResult {
            merge_sha: None,
            files_changed: Vec::new(),
            was_conflict: true,
            conflict_details: Some(conflict_scan),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_conflict_type_content() {
        assert_eq!(parse_conflict_type("content"), ConflictType::Content);
        assert_eq!(parse_conflict_type("contents"), ConflictType::Content);
    }

    #[test]
    fn test_parse_conflict_type_variants() {
        assert_eq!(
            parse_conflict_type("rename/delete"),
            ConflictType::RenameDelete
        );
        assert_eq!(
            parse_conflict_type("rename/rename"),
            ConflictType::RenameRename
        );
        assert_eq!(
            parse_conflict_type("modify/delete"),
            ConflictType::ModifyDelete
        );
        assert_eq!(parse_conflict_type("add/add"), ConflictType::AddAdd);
        assert_eq!(
            parse_conflict_type("file/directory"),
            ConflictType::FileDirectory
        );
        assert_eq!(parse_conflict_type("binary"), ConflictType::Binary);
        assert_eq!(parse_conflict_type("submodule"), ConflictType::Submodule);
    }

    #[test]
    fn test_parse_conflict_type_unknown() {
        assert_eq!(
            parse_conflict_type("something-new"),
            ConflictType::Other("something-new".to_string())
        );
    }

    #[test]
    fn test_parse_change_type() {
        assert_eq!(parse_change_type("A"), Some(ChangeType::Added));
        assert_eq!(parse_change_type("M"), Some(ChangeType::Modified));
        assert_eq!(parse_change_type("D"), Some(ChangeType::Deleted));
        assert_eq!(parse_change_type("X"), None);
        assert_eq!(parse_change_type(""), None);
    }

    #[test]
    fn test_parse_ahead_behind() {
        assert_eq!(parse_ahead_behind("5\t3"), (5, 3));
        assert_eq!(parse_ahead_behind("0\t0"), (0, 0));
        assert_eq!(parse_ahead_behind("12\t0"), (12, 0));
        assert_eq!(parse_ahead_behind("0\t7"), (0, 7));
    }

    #[test]
    fn test_parse_ahead_behind_malformed() {
        assert_eq!(parse_ahead_behind(""), (0, 0));
        assert_eq!(parse_ahead_behind("5"), (0, 0));
        assert_eq!(parse_ahead_behind("abc\tdef"), (0, 0));
    }

    #[test]
    fn test_is_valid_tree_oid() {
        // Valid 40-char hex OID
        assert!(is_valid_tree_oid(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
        ));
        // Valid with trailing content
        assert!(is_valid_tree_oid(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2\nmore lines"
        ));
        // Too short
        assert!(!is_valid_tree_oid("a1b2c3d4"));
        // Invalid chars
        assert!(!is_valid_tree_oid(
            "g1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
        ));
        // Empty
        assert!(!is_valid_tree_oid(""));
        // Error message
        assert!(!is_valid_tree_oid("merge-tree: invalid ref"));
    }

    #[test]
    fn test_parse_file_changes() {
        let output = "A\tfile1.rs\nM\tfile2.rs\nD\tfile3.rs";
        let changes = parse_file_changes(output);
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].path, "file1.rs");
        assert_eq!(changes[0].change_type, ChangeType::Added);
        assert_eq!(changes[1].path, "file2.rs");
        assert_eq!(changes[1].change_type, ChangeType::Modified);
        assert_eq!(changes[2].path, "file3.rs");
        assert_eq!(changes[2].change_type, ChangeType::Deleted);
    }

    #[test]
    fn test_parse_conflicts_from_stdout() {
        let stdout = "\
a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
100644 abc123 2\tfile.rs
100644 def456 3\tfile.rs

CONFLICT (content): Merge conflict in file.rs";

        let conflicts = parse_conflicts(stdout);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].path, "file.rs");
        assert_eq!(conflicts[0].conflict_type, ConflictType::Content);
        assert!(conflicts[0].message.contains("CONFLICT (content)"));
    }

    #[test]
    fn test_parse_conflicts_modify_delete() {
        let stdout = "\
a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
100644 abc123 1\tremoved.rs
100644 def456 2\tremoved.rs

CONFLICT (modify/delete): removed.rs deleted in HEAD and modified in feature";

        let conflicts = parse_conflicts(stdout);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].path, "removed.rs");
        assert_eq!(conflicts[0].conflict_type, ConflictType::ModifyDelete);
    }

    #[test]
    fn test_extract_path_merge_conflict_in() {
        assert_eq!(
            extract_path_from_message("Merge conflict in src/main.rs"),
            "src/main.rs"
        );
    }

    #[test]
    fn test_extract_path_deleted_in() {
        assert_eq!(
            extract_path_from_message("file.rs deleted in HEAD and modified in feature"),
            "file.rs"
        );
    }

    #[test]
    fn test_extract_path_renamed() {
        assert_eq!(
            extract_path_from_message("foo.rs renamed to bar.rs in HEAD"),
            "foo.rs"
        );
    }

    #[test]
    fn test_extract_path_fallback() {
        assert_eq!(
            extract_path_from_message("some unknown conflict message"),
            "some unknown conflict message"
        );
    }

    #[test]
    fn test_parse_conflicts_empty() {
        let stdout = "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let conflicts = parse_conflicts(stdout);
        assert!(
            conflicts.is_empty(),
            "OID-only input should produce no conflicts"
        );
    }

    #[test]
    fn test_parse_conflicts_multiple() {
        let stdout = "\
a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
100644 abc123 2\tfile1.rs
100644 def456 3\tfile1.rs
100644 abc123 2\tfile2.rs
100644 def456 3\tfile2.rs
100644 abc123 2\tfile3.rs
100644 def456 3\tfile3.rs

CONFLICT (content): Merge conflict in file1.rs
CONFLICT (content): Merge conflict in file2.rs
CONFLICT (modify/delete): file3.rs deleted in HEAD and modified in feature";

        let conflicts = parse_conflicts(stdout);
        assert_eq!(conflicts.len(), 3, "should parse all 3 CONFLICT lines");
        assert_eq!(conflicts[0].path, "file1.rs");
        assert_eq!(conflicts[1].path, "file2.rs");
        assert_eq!(conflicts[2].path, "file3.rs");
        assert_eq!(conflicts[2].conflict_type, ConflictType::ModifyDelete);
    }

    #[test]
    fn test_truncation_logic() {
        let stdout = "\
a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
100644 abc123 2\tfile1.rs
100644 abc123 2\tfile2.rs
100644 abc123 2\tfile3.rs

CONFLICT (content): Merge conflict in file1.rs
CONFLICT (content): Merge conflict in file2.rs
CONFLICT (content): Merge conflict in file3.rs";

        let all_conflicts = parse_conflicts(stdout);
        assert_eq!(all_conflicts.len(), 3);

        // Simulate max_conflicts = 2
        let max = 2_usize;
        let truncated = all_conflicts.len() > max;
        let limited: Vec<_> = all_conflicts.into_iter().take(max).collect();

        assert!(truncated, "3 conflicts with max 2 should be truncated");
        assert_eq!(limited.len(), 2, "should only keep 2 conflicts");
        assert_eq!(limited[0].path, "file1.rs");
        assert_eq!(limited[1].path, "file2.rs");

        // Simulate max_conflicts = 5 (not exceeded)
        let all_again = parse_conflicts(stdout);
        let max2 = 5_usize;
        let truncated2 = all_again.len() > max2;
        assert!(
            !truncated2,
            "3 conflicts with max 5 should not be truncated"
        );
    }

    #[test]
    fn test_parse_file_changes_empty() {
        let changes = parse_file_changes("");
        assert!(
            changes.is_empty(),
            "empty input should produce no file changes"
        );
    }

    #[test]
    fn test_parse_file_changes_rename_status() {
        // R100 status from git renames maps to Modified, using the new path.
        let output = "R100\told.rs\tnew.rs\nM\tother.rs";
        let changes = parse_file_changes(output);
        assert_eq!(changes.len(), 2, "R100 should be parsed as Modified");
        assert_eq!(changes[0].path, "new.rs");
        assert_eq!(changes[0].change_type, ChangeType::Modified);
        assert_eq!(changes[1].path, "other.rs");
        assert_eq!(changes[1].change_type, ChangeType::Modified);
    }

    #[test]
    fn test_is_valid_tree_oid_39_chars() {
        assert!(
            !is_valid_tree_oid("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b"),
            "39 hex chars should be invalid (boundary test)"
        );
    }

    // ── Conflict marker scanning tests ───────────────────────────────

    #[test]
    fn test_scan_conflict_markers_no_markers() {
        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        let scan = scan_conflict_markers(content, "main.rs");
        assert!(!scan.has_markers);
        assert!(scan.markers.is_empty());
        assert!(!scan.truncated);
    }

    #[test]
    fn test_scan_conflict_markers_full_conflict() {
        let content = "line 1\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> feature\nline 7\n";
        let scan = scan_conflict_markers(content, "file.rs");
        assert!(scan.has_markers);
        assert_eq!(scan.markers.len(), 3);
        assert_eq!(scan.markers[0].marker_type, MarkerType::Ours);
        assert_eq!(scan.markers[0].line, 2);
        assert_eq!(scan.markers[1].marker_type, MarkerType::Separator);
        assert_eq!(scan.markers[1].line, 4);
        assert_eq!(scan.markers[2].marker_type, MarkerType::Theirs);
        assert_eq!(scan.markers[2].line, 6);
        assert_eq!(scan.markers[0].file, "file.rs");
    }

    #[test]
    fn test_scan_files_for_markers_real_files() {
        let dir = tempfile::tempdir().unwrap();

        // File with conflict markers
        std::fs::write(
            dir.path().join("conflict.rs"),
            "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n",
        )
        .unwrap();

        // Clean file
        std::fs::write(dir.path().join("clean.rs"), "fn main() {}\n").unwrap();

        let files = vec!["conflict.rs".to_string(), "clean.rs".to_string()];
        let scan = scan_files_for_markers(dir.path(), &files);

        assert!(scan.has_markers);
        assert_eq!(scan.markers.len(), 3); // 3 markers in conflict.rs
        assert!(!scan.truncated);
    }

    #[test]
    fn test_scan_files_for_markers_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec!["nonexistent.rs".to_string()];
        let scan = scan_files_for_markers(dir.path(), &files);
        assert!(!scan.has_markers);
        assert!(scan.markers.is_empty());
    }

    // ── Merge execution integration tests ────────────────────────────

    /// Helper: create a new git repo in a temp dir with an initial commit.
    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();

        // Init and configure
        Command::new("git")
            .args(["init"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(p)
            .output()
            .unwrap();

        // Initial commit on main
        std::fs::write(p.join("readme.md"), "# hello\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(p)
            .output()
            .unwrap();

        dir
    }

    #[test]
    fn test_merge_execute_clean_merge() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Create a feature branch with a new file
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("feature.rs"), "fn feature() {}\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add feature"])
            .current_dir(p)
            .output()
            .unwrap();

        // Switch back to main
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(p)
            .output()
            .unwrap();

        // Merge
        let result = merge_execute(p, "feature", "merge feature").unwrap();

        assert!(!result.was_conflict);
        assert!(result.merge_sha.is_some());
        assert!(!result.files_changed.is_empty());
        assert!(result.conflict_details.is_none());

        // Verify the merge SHA matches HEAD
        let head = git_command(&["rev-parse", "HEAD"], p).unwrap();
        assert_eq!(result.merge_sha.unwrap(), head);

        // Verify the files_changed contains feature.rs
        assert!(
            result.files_changed.iter().any(|f| f.path == "feature.rs"),
            "files_changed should include feature.rs, got: {:?}",
            result.files_changed
        );
    }

    #[test]
    fn test_merge_execute_conflict_returns_details_and_aborts() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Create diverging branches
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("readme.md"), "# feature version\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature change"])
            .current_dir(p)
            .output()
            .unwrap();

        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("readme.md"), "# main version\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "main change"])
            .current_dir(p)
            .output()
            .unwrap();

        // Attempt merge — should conflict
        let result = merge_execute(p, "feature", "merge feature").unwrap();

        assert!(result.was_conflict);
        assert!(result.merge_sha.is_none());
        assert!(result.files_changed.is_empty());
        assert!(result.conflict_details.is_some());

        // Verify repo is clean (abort worked)
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(p)
            .output()
            .unwrap();
        let status_str = String::from_utf8_lossy(&status.stdout);
        assert!(
            status_str.trim().is_empty(),
            "repo should be clean after abort, got: {status_str}"
        );

        // Verify MERGE_HEAD is gone
        assert!(
            !p.join(".git/MERGE_HEAD").exists(),
            "MERGE_HEAD should be removed after abort"
        );
    }

    #[test]
    fn test_merge_execute_detects_in_progress_merge() {
        let dir = setup_git_repo();
        let p = dir.path();

        // Simulate an in-progress merge by creating MERGE_HEAD
        std::fs::write(
            p.join(".git/MERGE_HEAD"),
            "0000000000000000000000000000000000000000\n",
        )
        .unwrap();

        let err = merge_execute(p, "feature", "merge").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("already in progress"),
            "expected in-progress error, got: {msg}"
        );
    }
}
