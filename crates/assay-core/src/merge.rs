//! Merge check: conflict detection between git refs with zero side effects.
//!
//! Uses `git merge-tree --write-tree` to detect conflicts without mutating
//! the index or working tree. Follows the same `std::process::Command` pattern
//! as the `worktree` module.

use std::path::Path;
use std::process::Command;

use assay_types::{ChangeType, ConflictType, FileChange, MergeCheck, MergeConflict};

use crate::error::{AssayError, Result};

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Execute a git command and return (stdout, exit_code) without checking success.
///
/// Used when we need to inspect the exit code ourselves (e.g., merge-tree).
fn git_raw(args: &[&str], cwd: &Path) -> Result<(String, String, Option<i32>)> {
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
fn git_command(args: &[&str], cwd: &Path) -> Result<String> {
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
/// Handles both "content" and "contents" spellings (P3 pitfall).
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
fn parse_change_type(s: &str) -> Option<ChangeType> {
    match s {
        "A" => Some(ChangeType::Added),
        "M" => Some(ChangeType::Modified),
        "D" => Some(ChangeType::Deleted),
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
fn parse_file_changes(output: &str) -> Vec<FileChange> {
    output
        .lines()
        .filter_map(|line| {
            let (status, path) = line.split_once('\t')?;
            let change_type = parse_change_type(status)?;
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

    // Step 1: Resolve refs (P5: no --verify, supports relative refs)
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

    // Step 2: Merge base (P6: may fail for unrelated histories)
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

    // Step 5: Merge tree (P1: disambiguate exit code 1)
    let (mt_stdout, mt_stderr, mt_exit) = git_raw(
        &["merge-tree", "--write-tree", &base_sha, &head_sha],
        project_root,
    )?;

    // P4: Exit 128 = unrelated histories or fatal error
    if mt_exit == Some(128) {
        return Err(AssayError::WorktreeGitFailed {
            cmd: format!("git merge-tree --write-tree {base_sha} {head_sha}"),
            stderr: mt_stderr,
            exit_code: mt_exit,
        });
    }

    // P1: Check stdout for valid tree OID to disambiguate exit code 1
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
        // Step 6: Clean merge — get file list via diff-tree (P2)
        let tree_oid = mt_stdout.lines().next().unwrap_or("");
        let files = git_command(
            &["diff-tree", "-r", "--name-status", &base_sha, tree_oid],
            project_root,
        )
        .map(|output| parse_file_changes(&output))
        .unwrap_or_default();

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
}
