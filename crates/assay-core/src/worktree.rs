//! Git worktree lifecycle management.
//!
//! Provides create, list, status, and cleanup operations for git worktrees
//! associated with specs. All git operations are performed by shelling out
//! to the `git` CLI.

use std::path::{Path, PathBuf};
use std::process::Command;

use assay_types::{Config, WorktreeInfo, WorktreeMetadata, WorktreeStatus};

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
///
/// Returns an actionable error when the remote HEAD cannot be resolved,
/// guiding the user to configure their remote or pass `base_branch` explicitly.
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

/// Write worktree metadata to `<worktree_path>/.assay/worktree.json`.
///
/// Also adds `.assay/worktree.json` to the worktree's local git exclude
/// (`$GIT_DIR/info/exclude`) so it doesn't pollute `git status`.
fn write_metadata(worktree_path: &Path, metadata: &WorktreeMetadata) -> Result<()> {
    let meta_dir = worktree_path.join(".assay");
    std::fs::create_dir_all(&meta_dir)
        .map_err(|e| AssayError::io("creating worktree metadata dir", &meta_dir, e))?;
    let meta_path = meta_dir.join("worktree.json");
    let json = serde_json::to_string_pretty(metadata).map_err(|e| {
        AssayError::io(
            "serializing worktree metadata",
            &meta_path,
            std::io::Error::other(e),
        )
    })?;
    std::fs::write(&meta_path, json)
        .map_err(|e| AssayError::io("writing worktree metadata", &meta_path, e))?;

    // Add to the shared git exclude so the metadata file is invisible to status.
    // Use --git-common-dir to get the main .git dir (works for both main and linked worktrees).
    if let Ok(git_common_dir) = git_command(&["rev-parse", "--git-common-dir"], worktree_path) {
        let common_path = if Path::new(&git_common_dir).is_absolute() {
            PathBuf::from(&git_common_dir)
        } else {
            worktree_path.join(&git_common_dir)
        };
        let exclude_dir = common_path.join("info");
        if let Err(e) = std::fs::create_dir_all(&exclude_dir) {
            tracing::warn!(
                path = %exclude_dir.display(),
                "could not create git info dir for exclude entry: {e}"
            );
            return Ok(());
        }
        let exclude_path = exclude_dir.join("exclude");
        let exclude_entry = ".assay/worktree.json";
        let needs_entry = match std::fs::read_to_string(&exclude_path) {
            Ok(content) => !content.lines().any(|l| l.trim() == exclude_entry),
            Err(_) => true,
        };
        if needs_entry {
            use std::io::Write;
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&exclude_path)
            {
                Ok(mut file) => {
                    if let Err(e) = writeln!(file, "{exclude_entry}") {
                        tracing::warn!(
                            path = %exclude_path.display(),
                            "could not write git exclude entry: {e}"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        path = %exclude_path.display(),
                        "could not open git exclude file: {e}"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Read worktree metadata from `<worktree_path>/.assay/worktree.json`.
///
/// Returns `None` if the file is missing or cannot be parsed.
pub fn read_metadata(worktree_path: &Path) -> Option<WorktreeMetadata> {
    let meta_path = worktree_path.join(".assay").join("worktree.json");
    let content = std::fs::read_to_string(&meta_path).ok()?;
    serde_json::from_str(&content)
        .map_err(|e| {
            tracing::warn!(
                path = %meta_path.display(),
                "corrupt worktree metadata, ignoring: {e}"
            );
            e
        })
        .ok()
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
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    };

    // Canonicalize to resolve symlinks and `..` segments.
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

    // Check if worktree already exists (filesystem or lingering branch)
    if worktree_path.exists() {
        return Err(AssayError::WorktreeExists {
            spec_slug: spec_slug.to_string(),
            path: worktree_path,
        });
    }

    // Check if branch already exists (e.g., from incomplete cleanup)
    if git_command(
        &[
            "rev-parse",
            "--verify",
            &format!("refs/heads/{branch_name}"),
        ],
        project_root,
    )
    .is_ok()
    {
        return Err(AssayError::WorktreeExists {
            spec_slug: spec_slug.to_string(),
            path: worktree_path,
        });
    }

    // Ensure base directory exists
    std::fs::create_dir_all(worktree_base)
        .map_err(|e| AssayError::io("creating worktree base dir", worktree_base, e))?;

    // Resolve base branch
    let base = match base_branch {
        Some(b) => b.to_string(),
        None => detect_default_branch(project_root)?,
    };

    // Create worktree with new branch
    let path_str = worktree_path.to_string_lossy().to_string();
    git_command(
        &["worktree", "add", "-b", &branch_name, &path_str, &base],
        project_root,
    )?;

    // Persist metadata so status() can find the base branch later
    write_metadata(
        &worktree_path,
        &WorktreeMetadata {
            base_branch: base.clone(),
            spec_slug: spec_slug.to_string(),
        },
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
    Ok(entries)
}

/// Get the status of a worktree including dirty state and ahead/behind counts.
///
/// Ahead/behind counts are computed relative to the base branch (from metadata),
/// using the remote-tracking ref `origin/<base>` with a local fallback `refs/heads/<base>`.
/// If the base ref is missing, ahead/behind are `None` and a warning is included.
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

    // Read metadata to find the base branch
    let metadata = read_metadata(worktree_path);
    let base_branch = metadata.as_ref().map(|m| m.base_branch.clone());
    let mut warnings = Vec::new();

    let (ahead, behind) = if let Some(ref base) = base_branch {
        // Try remote-tracking ref first, fall back to local ref
        let remote_ref = format!("origin/{base}");
        let local_ref = format!("refs/heads/{base}");

        let base_ref = if git_command(
            &[
                "rev-parse",
                "--verify",
                &format!("refs/remotes/{remote_ref}"),
            ],
            worktree_path,
        )
        .is_ok()
        {
            remote_ref
        } else if git_command(&["rev-parse", "--verify", &local_ref], worktree_path).is_ok() {
            local_ref
        } else {
            warnings.push(format!(
                "Base branch '{base}' ref not found (tried origin/{base} and local). \
                 Ahead/behind counts unavailable."
            ));
            String::new()
        };

        if base_ref.is_empty() {
            (None, None)
        } else {
            let rev_range = format!("HEAD...{base_ref}");
            git_command(
                &["rev-list", "--left-right", "--count", &rev_range],
                worktree_path,
            )
            .ok()
            .and_then(|output| {
                let parts: Vec<&str> = output.split('\t').collect();
                if parts.len() == 2 {
                    Some((
                        Some(parts[0].parse::<u32>().unwrap_or(0)),
                        Some(parts[1].parse::<u32>().unwrap_or(0)),
                    ))
                } else {
                    None
                }
            })
            .unwrap_or((None, None))
        }
    } else {
        warnings.push("No worktree metadata found; ahead/behind counts unavailable.".to_string());
        (None, None)
    };

    Ok(WorktreeStatus {
        spec_slug: spec_slug.to_string(),
        path: worktree_path.to_path_buf(),
        branch,
        head,
        dirty,
        ahead,
        behind,
        base_branch,
        warnings,
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
    if force {
        git_command(&["worktree", "remove", "--force", &path_str], project_root)?;
    } else {
        git_command(&["worktree", "remove", &path_str], project_root)?;
    }

    // Delete the branch; use -d (safe) when not forced, -D (force) when forced
    let branch_name = format!("assay/{spec_slug}");
    let delete_flag = if force { "-D" } else { "-d" };
    if let Err(e) = git_command(&["branch", delete_flag, &branch_name], project_root) {
        eprintln!("Warning: failed to delete branch '{branch_name}': {e}");
    }

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
            sessions: None,
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

        // git init with explicit main branch
        git_command(&["init", "-b", "main"], &root).expect("git init failed");
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

        // List — should now include base_branch from metadata
        let entries = list(&root).expect("list failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].spec_slug, "auth-flow");
        assert_eq!(
            entries[0].base_branch.as_deref(),
            Some("main"),
            "list() should populate base_branch from metadata"
        );

        // Status — includes base_branch and ahead/behind relative to base
        let st = status(&info.path, "auth-flow").expect("status failed");
        assert_eq!(st.branch, "assay/auth-flow");
        assert!(!st.dirty);
        assert!(!st.head.is_empty());
        assert_eq!(st.base_branch.as_deref(), Some("main"));
        assert_eq!(st.ahead, Some(0));
        assert_eq!(st.behind, Some(0));
        assert!(st.warnings.is_empty());

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
    fn test_status_missing_metadata_returns_none_ahead_behind() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        let info = create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect("create failed");

        // Remove the metadata file to simulate a worktree without metadata
        let meta_path = info.path.join(".assay").join("worktree.json");
        std::fs::remove_file(&meta_path).expect("failed to remove metadata");

        let st = status(&info.path, "auth-flow").expect("status should still succeed");
        assert!(
            st.ahead.is_none(),
            "ahead should be None when metadata is missing"
        );
        assert!(
            st.behind.is_none(),
            "behind should be None when metadata is missing"
        );
        assert!(
            st.base_branch.is_none(),
            "base_branch should be None when metadata is missing"
        );
        assert!(
            !st.warnings.is_empty(),
            "should include a warning about missing metadata"
        );
        assert!(
            st.warnings[0].contains("metadata"),
            "warning should mention metadata, got: {}",
            st.warnings[0]
        );
    }

    #[test]
    fn test_read_write_metadata_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let metadata = WorktreeMetadata {
            base_branch: "develop".to_string(),
            spec_slug: "my-feature".to_string(),
        };

        write_metadata(dir.path(), &metadata).expect("write_metadata should succeed");
        let loaded = read_metadata(dir.path()).expect("read_metadata should return Some");
        assert_eq!(loaded, metadata);
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

    // -- resolve_worktree_dir canonicalization integration tests --

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
            sessions: None,
        }
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_canonicalizes_dotdot_segments() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        let tmp = TempDir::new().expect("failed to create temp dir");
        // Canonicalize the TempDir path itself (macOS /var/folders -> /private/var/folders)
        let canonical_tmp = std::fs::canonicalize(tmp.path()).expect("canonicalize tmp");

        // Create a subdirectory to use as project_root
        let project_dir = canonical_tmp.join("project");
        std::fs::create_dir(&project_dir).expect("failed to create project dir");

        // Use a relative path with `..` segments: "../myproject-worktrees"
        let config = make_config(Some("../myproject-worktrees"));
        let result = resolve_worktree_dir(None, &config, &project_dir);

        // The result should have no `..` segments — parent is canonicalized
        let result_str = result.to_string_lossy();
        assert!(
            !result_str.contains(".."),
            "expected no '..' segments in resolved path, got: {result_str}"
        );

        // The result should equal the canonical parent joined with the leaf
        let expected = canonical_tmp.join("myproject-worktrees");
        assert_eq!(
            result, expected,
            "resolved path should match canonicalized parent + leaf"
        );
    }

    #[test]
    #[serial]
    fn test_resolve_worktree_dir_canonicalizes_symlinks() {
        // SAFETY: Test-only; env var manipulation is single-threaded via serial_test.
        unsafe { std::env::remove_var("ASSAY_WORKTREE_DIR") };

        let tmp = TempDir::new().expect("failed to create temp dir");
        let canonical_tmp = std::fs::canonicalize(tmp.path()).expect("canonicalize tmp");

        // Create the real directory and a symlink to it
        let real_dir = canonical_tmp.join("real-worktrees");
        std::fs::create_dir(&real_dir).expect("failed to create real dir");
        let symlink_dir = canonical_tmp.join("link-worktrees");
        std::os::unix::fs::symlink(&real_dir, &symlink_dir).expect("failed to create symlink");

        // Point config at the symlink path
        let symlink_str = symlink_dir.to_string_lossy().to_string();
        let config = make_config(Some(&symlink_str));
        let project_root = canonical_tmp.join("project");
        std::fs::create_dir(&project_root).expect("failed to create project dir");

        let result = resolve_worktree_dir(None, &config, &project_root);

        // The result should point to the real path, not the symlink
        assert_eq!(
            result, real_dir,
            "resolved path should follow symlinks to real path"
        );
    }

    #[test]
    fn test_create_without_base_branch_no_remote_returns_error() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        // setup_repo() creates a repo with no remote, so detection should fail
        let err = create(&root, "auth-flow", None, &worktree_base, &specs_dir)
            .expect_err("should fail when no remote is configured and base_branch is None");

        let err_msg = err.to_string();
        assert!(
            err_msg.contains("Could not detect default branch"),
            "error should mention detection failure, got: {err_msg}"
        );
        assert!(
            err_msg.contains("init.defaultBranch"),
            "error should mention init.defaultBranch config key, got: {err_msg}"
        );
        assert!(
            err_msg.contains("git remote set-head origin --auto"),
            "error should mention git remote set-head command, got: {err_msg}"
        );
    }

    #[test]
    fn test_create_with_explicit_base_branch_skips_detection() {
        let (_tmp, _wt_tmp, root, specs_dir) = setup_repo();
        let worktree_base = _wt_tmp.path().join("worktrees");

        // Even though no remote is configured, explicit base_branch bypasses detection
        let info = create(&root, "auth-flow", Some("main"), &worktree_base, &specs_dir)
            .expect("create with explicit base_branch should succeed regardless of remote state");

        assert_eq!(info.spec_slug, "auth-flow");
        assert_eq!(info.base_branch.as_deref(), Some("main"));
        assert!(info.path.exists());
    }
}
