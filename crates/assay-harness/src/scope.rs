//! Scope enforcement and multi-agent prompt generation.
//!
//! Provides [`check_scope`] to detect file-access violations against declared
//! `file_scope` and `shared_files` glob patterns, and [`generate_scope_prompt`]
//! to produce multi-agent awareness markdown for injection as a system prompt layer.

use assay_types::{ScopeViolation, ScopeViolationType};
use globset::{Glob, GlobSet, GlobSetBuilder};

/// Build a [`GlobSet`] from a slice of glob pattern strings.
///
/// Returns `None` if patterns is empty. Returns `Err` if any pattern is invalid.
fn build_glob_set(patterns: &[String]) -> Result<Option<GlobSet>, globset::Error> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        builder.add(Glob::new(pat)?);
    }
    Ok(Some(builder.build()?))
}

/// Check changed files against scope boundaries, returning any violations.
///
/// Rules:
/// - If `file_scope` is empty, no restrictions apply — returns an empty vec.
/// - A file matching `shared_files` produces an advisory [`ScopeViolationType::SharedFileConflict`].
/// - A file matching neither `file_scope` nor `shared_files` produces [`ScopeViolationType::OutOfScope`].
/// - A file matching `file_scope` (and not `shared_files`) produces no violation.
///
/// The returned violations are advisory per design decision D027.
pub fn check_scope(
    file_scope: &[String],
    shared_files: &[String],
    changed_files: &[String],
) -> Vec<ScopeViolation> {
    // Empty file_scope means no restrictions.
    if file_scope.is_empty() {
        return Vec::new();
    }

    let scope_set = match build_glob_set(file_scope) {
        Ok(Some(s)) => s,
        Ok(None) => return Vec::new(),
        Err(_) => return Vec::new(),
    };

    let shared_set = build_glob_set(shared_files).ok().flatten();

    let mut violations = Vec::new();

    for file in changed_files {
        let in_shared = shared_set.as_ref().is_some_and(|s| s.is_match(file));
        let in_scope = scope_set.is_match(file);

        if in_shared {
            // Find the first matching shared_files pattern for context.
            let pattern = shared_files
                .iter()
                .find(|p| {
                    Glob::new(p)
                        .ok()
                        .map(|g| g.compile_matcher().is_match(file))
                        .unwrap_or(false)
                })
                .cloned()
                .unwrap_or_default();

            violations.push(ScopeViolation {
                file: file.clone(),
                violation_type: ScopeViolationType::SharedFileConflict,
                pattern,
            });
        } else if !in_scope {
            // Find the closest file_scope pattern for diagnostic context.
            let pattern = file_scope.first().cloned().unwrap_or_default();

            violations.push(ScopeViolation {
                file: file.clone(),
                violation_type: ScopeViolationType::OutOfScope,
                pattern,
            });
        }
    }

    violations
}

/// Generate multi-agent awareness markdown for a session.
///
/// Produces a concise markdown string listing:
/// - This session's owned file scope
/// - Shared files requiring coordination
/// - Direct neighbors (other sessions whose scopes overlap or share files)
///
/// `all_sessions` is a slice of `(name, file_scope, shared_files)` tuples
/// for every session in the manifest. Only sessions with overlapping scope
/// or shared files are listed as neighbors.
pub fn generate_scope_prompt(
    session_name: &str,
    file_scope: &[String],
    shared_files: &[String],
    all_sessions: &[(String, Vec<String>, Vec<String>)],
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# Scope: {session_name}"));
    lines.push(String::new());

    // Owned scope
    if file_scope.is_empty() {
        lines.push("**File scope:** unrestricted".to_string());
    } else {
        lines.push("**File scope:**".to_string());
        for pat in file_scope {
            lines.push(format!("- `{pat}`"));
        }
    }
    lines.push(String::new());

    // Shared files
    if !shared_files.is_empty() {
        lines.push("**Shared files** (coordinate before editing):".to_string());
        for pat in shared_files {
            lines.push(format!("- `{pat}`"));
        }
        lines.push(String::new());
    }

    // Find neighbors: sessions that share files or have overlapping scope.
    let neighbors: Vec<&str> = all_sessions
        .iter()
        .filter(|(name, other_scope, other_shared)| {
            if name == session_name {
                return false;
            }
            // Check if any of our shared_files overlap with their scope or shared_files.
            let shared_overlap = shared_files
                .iter()
                .any(|sf| other_scope.contains(sf) || other_shared.contains(sf));
            // Check if any of their shared_files overlap with our scope.
            let reverse_overlap = other_shared
                .iter()
                .any(|sf| file_scope.contains(sf) || shared_files.contains(sf));
            // Check direct scope overlap via shared_files on either side.
            let their_shared_in_our_scope =
                !other_shared.is_empty() && other_shared.iter().any(|sf| file_scope.contains(sf));
            shared_overlap || reverse_overlap || their_shared_in_our_scope
        })
        .map(|(name, _, _)| name.as_str())
        .collect();

    if !neighbors.is_empty() {
        lines.push("**Neighbors** (sessions with overlapping scope):".to_string());
        for n in &neighbors {
            // Show neighbor's scope summary.
            if let Some((_, scope, shared)) = all_sessions.iter().find(|(name, _, _)| name == n) {
                let scope_summary = if scope.is_empty() {
                    "unrestricted".to_string()
                } else {
                    scope
                        .iter()
                        .map(|s| format!("`{s}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let mut desc = format!("- **{n}**: {scope_summary}");
                if !shared.is_empty() {
                    let shared_str = shared
                        .iter()
                        .map(|s| format!("`{s}`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    desc.push_str(&format!(" (shares: {shared_str})"));
                }
                lines.push(desc);
            }
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(val: &str) -> String {
        val.to_string()
    }

    // (a) Empty file_scope returns no violations.
    #[test]
    fn empty_file_scope_returns_no_violations() {
        let result = check_scope(&[], &[s("shared/**")], &[s("anything.rs")]);
        assert!(result.is_empty());
    }

    // (b) File matching file_scope returns no violations.
    #[test]
    fn file_matching_scope_returns_no_violations() {
        let result = check_scope(
            &[s("src/**/*.rs")],
            &[],
            &[s("src/main.rs"), s("src/lib.rs")],
        );
        assert!(result.is_empty());
    }

    // (c) File outside file_scope returns OutOfScope.
    #[test]
    fn file_outside_scope_returns_out_of_scope() {
        let result = check_scope(&[s("src/**/*.rs")], &[], &[s("tests/integration.rs")]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, "tests/integration.rs");
        assert_eq!(result[0].violation_type, ScopeViolationType::OutOfScope);
    }

    // (d) File matching shared_files returns SharedFileConflict.
    #[test]
    fn file_matching_shared_returns_conflict() {
        let result = check_scope(
            &[s("src/**")],
            &[s("shared/**")],
            &[s("shared/config.toml")],
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, "shared/config.toml");
        assert_eq!(
            result[0].violation_type,
            ScopeViolationType::SharedFileConflict
        );
        assert_eq!(result[0].pattern, "shared/**");
    }

    // (e) Glob patterns with **/*.rs and {src,tests}/** work.
    #[test]
    fn complex_glob_patterns_work() {
        let result = check_scope(
            &[s("**/*.rs"), s("{src,tests}/**")],
            &[],
            &[s("src/lib.rs"), s("tests/foo.txt"), s("docs/readme.md")],
        );
        // src/lib.rs matches **/*.rs — no violation
        // tests/foo.txt matches {src,tests}/** — no violation
        // docs/readme.md matches neither — OutOfScope
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file, "docs/readme.md");
        assert_eq!(result[0].violation_type, ScopeViolationType::OutOfScope);
    }

    // (f) generate_scope_prompt produces expected markdown.
    #[test]
    fn generate_scope_prompt_with_neighbors() {
        let all_sessions = vec![
            (
                s("frontend"),
                vec![s("src/ui/**")],
                vec![s("shared/api.ts")],
            ),
            (
                s("backend"),
                vec![s("src/api/**")],
                vec![s("shared/api.ts")],
            ),
        ];
        let prompt = generate_scope_prompt(
            "frontend",
            &[s("src/ui/**")],
            &[s("shared/api.ts")],
            &all_sessions,
        );
        assert!(prompt.contains("# Scope: frontend"));
        assert!(prompt.contains("`src/ui/**`"));
        assert!(prompt.contains("Shared files"));
        assert!(prompt.contains("`shared/api.ts`"));
        assert!(prompt.contains("Neighbors"));
        assert!(prompt.contains("backend"));
    }

    // (g) generate_scope_prompt with no neighbors is concise.
    #[test]
    fn generate_scope_prompt_no_neighbors() {
        let all_sessions = vec![
            (s("solo"), vec![s("src/**")], vec![]),
            (s("other"), vec![s("tests/**")], vec![]),
        ];
        let prompt = generate_scope_prompt("solo", &[s("src/**")], &[], &all_sessions);
        assert!(prompt.contains("# Scope: solo"));
        assert!(prompt.contains("`src/**`"));
        assert!(!prompt.contains("Neighbors"));
        assert!(!prompt.contains("Shared files"));
    }

    // Additional: file in both scope and shared_files returns SharedFileConflict.
    #[test]
    fn file_in_scope_and_shared_returns_shared_conflict() {
        let result = check_scope(&[s("src/**")], &[s("src/shared.rs")], &[s("src/shared.rs")]);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].violation_type,
            ScopeViolationType::SharedFileConflict
        );
    }

    // Additional: unrestricted scope prompt.
    #[test]
    fn generate_scope_prompt_unrestricted() {
        let prompt = generate_scope_prompt("agent", &[], &[], &[]);
        assert!(prompt.contains("unrestricted"));
    }
}
