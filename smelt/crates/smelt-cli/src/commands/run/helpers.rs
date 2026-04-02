//! Helper functions for the `smelt run` subcommand.

use smelt_core::forge::ForgeConfig;

/// Returns true when Phase 9 should attempt PR creation.
pub(super) fn should_create_pr(no_pr: bool, no_changes: bool, forge: Option<&ForgeConfig>) -> bool {
    !no_pr && !no_changes && forge.is_some()
}

/// Ensure `.assay/` appears in the repo's `.gitignore`.
///
/// - If `.gitignore` does not exist: creates it with `.assay/\n`.
/// - If `.gitignore` exists and already contains `.assay/`: no-op (idempotent).
/// - If `.gitignore` exists but lacks `.assay/`: appends, preserving a trailing
///   newline boundary so the new entry always starts on its own line.
pub(super) fn ensure_gitignore_assay(repo_path: &std::path::Path) -> anyhow::Result<()> {
    let gitignore_path = repo_path.join(".gitignore");

    if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path)?;
        // Idempotency check: already present — nothing to do
        if content.contains(".assay/") {
            return Ok(());
        }
        // Append, ensuring the entry begins on a new line
        let append = if content.ends_with('\n') {
            ".assay/\n".to_string()
        } else {
            "\n.assay/\n".to_string()
        };
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&gitignore_path)?;
        file.write_all(append.as_bytes())?;
    } else {
        std::fs::write(&gitignore_path, ".assay/\n")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── ensure_gitignore_assay tests ────────────────────────────────────────

    #[test]
    fn test_ensure_gitignore_creates() {
        let tmp = TempDir::new().unwrap();
        // No .gitignore exists — should create it
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(
            content.contains(".assay/"),
            "created .gitignore should contain .assay/"
        );
    }

    #[test]
    fn test_ensure_gitignore_appends() {
        let tmp = TempDir::new().unwrap();
        // Existing .gitignore with trailing newline
        std::fs::write(tmp.path().join(".gitignore"), "target/\n").unwrap();
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert!(content.contains("target/"), "original entry preserved");
        assert!(content.contains(".assay/"), "new entry added");
    }

    #[test]
    fn test_ensure_gitignore_trailing_newline() {
        let tmp = TempDir::new().unwrap();
        // Existing .gitignore WITHOUT trailing newline
        std::fs::write(tmp.path().join(".gitignore"), "target/").unwrap();
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        // Must NOT produce "target/.assay/" on the same line
        assert!(
            !content.contains("target/.assay/"),
            "entries must be on separate lines, got: {content:?}"
        );
        assert!(content.contains(".assay/"), ".assay/ must appear in file");
    }

    #[test]
    fn test_ensure_gitignore_idempotent() {
        let tmp = TempDir::new().unwrap();
        // Already contains .assay/
        std::fs::write(tmp.path().join(".gitignore"), ".assay/\n").unwrap();
        // Call twice
        ensure_gitignore_assay(tmp.path()).unwrap();
        ensure_gitignore_assay(tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        let count = content.matches(".assay/").count();
        assert_eq!(count, 1, ".assay/ should appear exactly once, got {count}");
    }

    // ── should_create_pr tests ──────────────────────────────────────────────

    fn forge_cfg() -> ForgeConfig {
        ForgeConfig {
            provider: "github".to_string(),
            repo: "owner/repo".to_string(),
            token_env: "GITHUB_TOKEN".to_string(),
        }
    }

    #[test]
    fn test_should_create_pr_guard() {
        let cfg = forge_cfg();

        // forge=None → always false regardless of other flags
        assert!(
            !should_create_pr(false, false, None),
            "forge=None should be false"
        );
        assert!(
            !should_create_pr(false, true, None),
            "forge=None+no_changes should be false"
        );
        assert!(
            !should_create_pr(true, false, None),
            "forge=None+no_pr should be false"
        );
        assert!(
            !should_create_pr(true, true, None),
            "forge=None+both flags should be false"
        );

        // no_pr=true → always false
        assert!(
            !should_create_pr(true, false, Some(&cfg)),
            "no_pr=true should be false"
        );
        assert!(
            !should_create_pr(true, true, Some(&cfg)),
            "no_pr=true+no_changes should be false"
        );

        // no_changes=true → false
        assert!(
            !should_create_pr(false, true, Some(&cfg)),
            "no_changes=true should be false"
        );

        // all three conditions clear → true
        assert!(
            should_create_pr(false, false, Some(&cfg)),
            "all clear should be true"
        );
    }
}
