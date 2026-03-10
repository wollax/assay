//! Git worktree lifecycle management.
//!
//! Provides create, list, status, and cleanup operations for git worktrees
//! associated with specs. All git operations are performed by shelling out
//! to the `git` CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

use assay_types::{Config, WorktreeInfo, WorktreeStatus};

use crate::error::{AssayError, Result};

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

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

/// Detect the default branch from the remote HEAD ref.
/// Falls back to "main" on failure.
fn detect_default_branch(project_root: &Path) -> String {
    git_command(&["symbolic-ref", "refs/remotes/origin/HEAD"], project_root)
        .ok()
        .and_then(|output| {
            output
                .strip_prefix("refs/remotes/origin/")
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "main".to_string())
}

/// A raw worktree entry parsed from `git worktree list --porcelain`.
#[derive(Debug)]
struct RawWorktree {
    path: PathBuf,
    #[allow(dead_code)]
    head: String,
    branch: Option<String>,
}

/// Parse the porcelain output of `git worktree list --porcelain`.
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
            Some(RawWorktree {
                path: path?,
                head: head?,
                branch,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Resolve the base directory for worktrees using the config precedence chain.
///
/// Precedence: `cli_override` > `ASSAY_WORKTREE_DIR` env var > `config.worktree.base_dir` > default.
/// The default is `../<project_name>-worktrees/` relative to `project_root`.
/// Relative paths are resolved against `project_root`.
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
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

/// Create a new git worktree for a spec.
///
/// Validates that the spec exists before creating the worktree.
/// The worktree is created at `worktree_base/<spec_slug>` with branch `assay/<spec_slug>`.
pub fn create(
    project_root: &Path,
    spec_slug: &str,
    base_branch: Option<&str>,
    worktree_base: &Path,
    specs_dir: &Path,
) -> Result<WorktreeInfo> {
    // Validate spec exists
    crate::spec::load_spec_entry(spec_slug, specs_dir)?;

    let worktree_path = worktree_base.join(spec_slug);
    let branch_name = format!("assay/{spec_slug}");

    // Check if worktree already exists
    if worktree_path.exists() {
        return Err(AssayError::WorktreeExists {
            spec_slug: spec_slug.to_string(),
            path: worktree_path,
        });
    }

    // Ensure base directory exists
    std::fs::create_dir_all(worktree_base)
        .map_err(|e| AssayError::io("creating worktree base dir", worktree_base, e))?;

    // Resolve base branch
    let base = base_branch
        .map(|s| s.to_string())
        .unwrap_or_else(|| detect_default_branch(project_root));

    // Create worktree with new branch
    let path_str = worktree_path.to_string_lossy().to_string();
    git_command(
        &["worktree", "add", "-b", &branch_name, &path_str, &base],
        project_root,
    )?;

    Ok(WorktreeInfo {
        spec_slug: spec_slug.to_string(),
        path: worktree_path,
        branch: branch_name,
        base_branch: Some(base),
    })
}

/// List all assay-managed worktrees.
///
/// Prunes stale entries first, then parses `git worktree list --porcelain`
/// and filters to worktrees whose branch starts with `assay/`.
pub fn list(project_root: &Path) -> Result<Vec<WorktreeInfo>> {
    // Prune stale entries
    let _ = git_command(&["worktree", "prune"], project_root);

    let output = git_command(&["worktree", "list", "--porcelain"], project_root)?;
    let raw = parse_worktree_list(&output);

    let mut entries: Vec<WorktreeInfo> = raw
        .into_iter()
        .filter_map(|wt| {
            let branch = wt.branch.as_deref()?;
            let slug = branch.strip_prefix("assay/")?;
            Some(WorktreeInfo {
                spec_slug: slug.to_string(),
                path: wt.path,
                branch: branch.to_string(),
                base_branch: None,
            })
        })
        .collect();

    entries.sort_by(|a, b| a.spec_slug.cmp(&b.spec_slug));
    Ok(entries)
}

/// Get the status of a worktree including dirty state and ahead/behind counts.
pub fn status(worktree_path: &Path, spec_slug: &str) -> Result<WorktreeStatus> {
    if !worktree_path.exists() {
        return Err(AssayError::WorktreeNotFound {
            spec_slug: spec_slug.to_string(),
        });
    }

    let branch = git_command(&["rev-parse", "--abbrev-ref", "HEAD"], worktree_path)?;
    let head = git_command(&["rev-parse", "--short", "HEAD"], worktree_path)?;

    let porcelain_output = git_command(&["status", "--porcelain"], worktree_path)?;
    let dirty = !porcelain_output.is_empty();

    // ahead/behind — default to 0/0 if no upstream
    let (ahead, behind) = git_command(
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        worktree_path,
    )
    .ok()
    .and_then(|output| {
        let parts: Vec<&str> = output.split('\t').collect();
        if parts.len() == 2 {
            Some((
                parts[0].parse::<usize>().unwrap_or(0),
                parts[1].parse::<usize>().unwrap_or(0),
            ))
        } else {
            None
        }
    })
    .unwrap_or((0, 0));

    Ok(WorktreeStatus {
        spec_slug: spec_slug.to_string(),
        path: worktree_path.to_path_buf(),
        branch,
        head,
        dirty,
        ahead,
        behind,
    })
}

/// Remove a worktree and its associated branch.
///
/// If the worktree has uncommitted changes and `force` is false,
/// returns `WorktreeDirty`. When force is true, uses `--force` for removal.
pub fn cleanup(
    project_root: &Path,
    worktree_path: &Path,
    spec_slug: &str,
    force: bool,
) -> Result<()> {
    if !worktree_path.exists() {
        return Err(AssayError::WorktreeNotFound {
            spec_slug: spec_slug.to_string(),
        });
    }

    // Check dirty state
    let porcelain_output = git_command(&["status", "--porcelain"], worktree_path)?;
    let dirty = !porcelain_output.is_empty();

    if dirty && !force {
        return Err(AssayError::WorktreeDirty {
            spec_slug: spec_slug.to_string(),
        });
    }

    // Remove worktree
    let path_str = worktree_path.to_string_lossy().to_string();
    if dirty || force {
        git_command(&["worktree", "remove", "--force", &path_str], project_root)?;
    } else {
        git_command(&["worktree", "remove", &path_str], project_root)?;
    }

    // Delete the branch (ignore error if branch doesn't exist)
    let branch_name = format!("assay/{spec_slug}");
    let _ = git_command(&["branch", "-D", &branch_name], project_root);

    Ok(())
}

/// Detect if the current working directory is inside a linked worktree.
///
/// Returns the main repository root path if `cwd` is a linked worktree,
/// or `None` if `cwd` is the main worktree (or not a git repo).
pub fn detect_main_worktree(cwd: &Path) -> Option<PathBuf> {
    let dot_git = cwd.join(".git");
    if dot_git.is_file() {
        // Linked worktree — .git is a file containing "gitdir: <path>"
        let content = std::fs::read_to_string(&dot_git).ok()?;
        let gitdir = content.strip_prefix("gitdir: ")?.trim();
        // gitdir points to .git/worktrees/<name> in the main repo
        let gitdir_path = if Path::new(gitdir).is_absolute() {
            PathBuf::from(gitdir)
        } else {
            cwd.join(gitdir).canonicalize().ok()?
        };
        // Navigate up: .git/worktrees/<name> -> .git/worktrees -> .git -> repo root
        let main_git_dir = gitdir_path.parent()?.parent()?;
        Some(main_git_dir.parent()?.to_path_buf())
    } else {
        None // Already in main worktree (or .git is a directory)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_worktree_list unit tests --

    #[test]
    fn test_parse_worktree_list_normal() {
        let porcelain = "\
worktree /home/user/project
HEAD abc1234
branch refs/heads/main

worktree /tmp/worktrees/auth
HEAD def5678
branch refs/heads/assay/auth-flow
";
        let result = parse_worktree_list(porcelain);
        assert_eq!(result.len(), 2);

        assert_eq!(result[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(result[0].branch.as_deref(), Some("main"));

        assert_eq!(result[1].path, PathBuf::from("/tmp/worktrees/auth"));
        assert_eq!(result[1].branch.as_deref(), Some("assay/auth-flow"));
    }

    #[test]
    fn test_parse_worktree_list_empty() {
        assert!(parse_worktree_list("").is_empty());
        assert!(parse_worktree_list("  \n  ").is_empty());
    }

    #[test]
    fn test_parse_worktree_list_bare() {
        let porcelain = "\
worktree /home/user/project.git
HEAD abc1234
bare
";
        let result = parse_worktree_list(porcelain);
        assert_eq!(result.len(), 1);
        assert!(result[0].branch.is_none());
    }

    #[test]
    fn test_parse_worktree_list_detached() {
        let porcelain = "\
worktree /tmp/detached
HEAD abc1234
detached
";
        let result = parse_worktree_list(porcelain);
        assert_eq!(result.len(), 1);
        assert!(result[0].branch.is_none());
    }

    // -- resolve_worktree_dir unit tests --

    use serial_test::serial;

    fn make_config(base_dir: Option<&str>) -> Config {
        Config {
            project_name: "myproject".to_string(),
            specs_dir: "specs/".to_string(),
            gates: None,
            guard: None,
            worktree: base_dir.map(|d| assay_types::WorktreeConfig {
                base_dir: d.to_string(),
            }),
        }
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_default() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        let config = make_config(None);
        let root = Path::new("/home/user/myproject");
        let result = resolve_worktree_dir(None, &config, root);
        assert_eq!(
            result,
            PathBuf::from("/home/user/myproject/../myproject-worktrees")
        );
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_config() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        let config = make_config(Some("/custom/worktrees"));
        let root = Path::new("/home/user/myproject");
        let result = resolve_worktree_dir(None, &config, root);
        assert_eq!(result, PathBuf::from("/custom/worktrees"));
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_env_overrides_config() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::set_var("ASSAY_WORKTREE_DIR", "/env/worktrees") };

        let config = make_config(Some("/custom/worktrees"));
        let root = Path::new("/home/user/myproject");
        let result = resolve_worktree_dir(None, &config, root);

        // SAFETY: Cleanup.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        assert_eq!(result, PathBuf::from("/env/worktrees"));
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_cli_overrides_all() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::set_var("ASSAY_WORKTREE_DIR", "/env/worktrees") };

        let config = make_config(Some("/custom/worktrees"));
        let root = Path::new("/home/user/myproject");
        let result = resolve_worktree_dir(Some("/cli/worktrees"), &config, root);

        // SAFETY: Cleanup.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        assert_eq!(result, PathBuf::from("/cli/worktrees"));
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_relative_resolved_against_root() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        let config = make_config(Some("my-worktrees"));
        let root = Path::new("/home/user/myproject");
        let result = resolve_worktree_dir(None, &config, root);
        assert_eq!(result, PathBuf::from("/home/user/myproject/my-worktrees"));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    /// Set up a temporary git repo with an initial commit and a spec file.
    /// Returns (repo_tmp, worktree_tmp, root, specs_dir).
    fn setup_repo() -> (TempDir, TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let root = tmp.path().to_path_buf();
        let wt_tmp = TempDir::new().expect("failed to create worktree temp dir");

        // git init
        git_command(&["init"], &root).expect("git init failed");
        git_command(&["config", "user.email", "test@example.com"], &root)
            .expect("git config email failed");
        git_command(&["config", "user.name", "Test User"], &root).expect("git config name failed");

        // Create specs directory with a legacy spec
        let specs_dir = root.join(".assay").join("specs");
        std::fs::create_dir_all(&specs_dir).expect("failed to create specs dir");

        let spec_content = r#"
name = "auth-flow"
description = "Authentication flow"

[[criteria]]
name = "Login works"
description = "Verify login works"
cmd = "echo ok"
"#;
        std::fs::write(specs_dir.join("auth-flow.toml"), spec_content)
            .expect("failed to write spec");

        // Create an initial commit so we have a branch
        git_command(&["add", "."], &root).expect("git add failed");
        git_command(&["commit", "-m", "initial"], &root).expect("git commit failed");

        (tmp, wt_tmp, root, specs_dir)
    }

    /// Set up a directory-based spec in an existing repo.
    fn add_directory_spec(specs_dir: &Path, slug: &str) {
        let dir = specs_dir.join(slug);
        std::fs::create_dir_all(&dir).expect("failed to create spec dir");

        let gates_content = r#"
name = "payments"
description = "Payment processing"

[[criteria]]
name = "Payments work"
description = "Verify payments work"
cmd = "echo ok"
"#;
        std::fs::write(dir.join("gates.toml"), gates_content).expect("failed to write gates.toml");
    }

    #[test]
    fn test_create_list_status_cleanup() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        // Create
        let info = create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect("create failed");
        assert_eq!(info.spec_slug, "auth-flow");
        assert_eq!(info.branch, "assay/auth-flow");
        assert_eq!(info.base_branch.as_deref(), Some("main"));
        assert!(info.path.exists());

        // List
        let entries = list(&root).expect("list failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].spec_slug, "auth-flow");

        // Status
        let st = status(&info.path, "auth-flow").expect("status failed");
        assert_eq!(st.branch, "assay/auth-flow");
        assert!(!st.dirty);
        assert!(!st.head.is_empty());

        // Cleanup
        cleanup(&root, &info.path, "auth-flow", false).expect("cleanup failed");
        assert!(!info.path.exists());

        // List should be empty now
        let entries = list(&root).expect("list failed");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_create_nonexistent_spec_returns_spec_not_found() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        let err = create(
            &root,
            "nonexistent",
            Some("main"),
            &worktree_base,
            &specs_dir,
        )
        .expect_err("should fail for nonexistent spec");
        assert!(
            matches!(err, AssayError::SpecNotFound { .. }),
            "expected SpecNotFound, got: {err:?}"
        );
    }

    #[test]
    fn test_create_duplicate_returns_worktree_exists() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect("first create should succeed");

        let err = create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect_err("duplicate create should fail");
        assert!(
            matches!(err, AssayError::WorktreeExists { .. }),
            "expected WorktreeExists, got: {err:?}"
        );
    }

    #[test]
    fn test_cleanup_dirty_without_force_returns_worktree_dirty() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        let info = create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect("create failed");

        // Make worktree dirty
        std::fs::write(info.path.join("dirty.txt"), "uncommitted")
            .expect("failed to write dirty file");

        let err = cleanup(&root, &info.path, "auth-flow", false)
            .expect_err("should fail for dirty worktree");
        assert!(
            matches!(err, AssayError::WorktreeDirty { .. }),
            "expected WorktreeDirty, got: {err:?}"
        );

        // Force cleanup should work
        cleanup(&root, &info.path, "auth-flow", true).expect("force cleanup should succeed");
        assert!(!info.path.exists());
    }

    #[test]
    fn test_create_directory_based_spec() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        // Add a directory-based spec and commit it
        add_directory_spec(&specs_dir, "payments");
        git_command(&["add", "."], &root).expect("git add failed");
        git_command(&["commit", "-m", "add payments spec"], &root).expect("git commit failed");

        let info = create(&root, "payments", Some("main"), &worktree_base, &specs_dir)
            .expect("create with directory-based spec should succeed");
        assert_eq!(info.spec_slug, "payments");
        assert_eq!(info.branch, "assay/payments");
    }

    #[test]
    fn test_status_nonexistent_returns_not_found() {
        let err = status(Path::new("/nonexistent/path"), "ghost")
            .expect_err("should fail for nonexistent worktree");
        assert!(
            matches!(err, AssayError::WorktreeNotFound { .. }),
            "expected WorktreeNotFound, got: {err:?}"
        );
    }

    #[test]
    fn test_cleanup_nonexistent_returns_not_found() {
        let (_tmp, _wt_tmp, root, _specs_dir) = setup_repo();

        let err = cleanup(&root, Path::new("/nonexistent/path"), "ghost", false)
            .expect_err("should fail for nonexistent worktree");
        assert!(
            matches!(err, AssayError::WorktreeNotFound { .. }),
            "expected WorktreeNotFound, got: {err:?}"
        );
    }

    #[test]
    fn test_detect_main_worktree_from_linked() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        let info = create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect("create failed");

        // From inside the linked worktree, detect_main_worktree should return the main repo
        let main = detect_main_worktree(&info.path);
        assert!(main.is_some(), "should detect main worktree");
        let main_path = main.unwrap();
        // Canonicalize both for comparison (temp dirs may have symlinks)
        let canon_main = std::fs::canonicalize(&main_path).expect("canonicalize main");
        let canon_root = std::fs::canonicalize(&root).expect("canonicalize root");
        assert_eq!(canon_main, canon_root);
    }

    #[test]
    fn test_detect_main_worktree_from_main_returns_none() {
        let (_tmp, _wt_tmp, root, _specs_dir) = setup_repo();
        assert!(detect_main_worktree(&root).is_none());
    }
}
