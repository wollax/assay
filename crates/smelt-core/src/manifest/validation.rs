//! Semantic validation for job manifests.

use std::collections::{HashMap, HashSet};

use crate::error::SmeltError;

use super::{JobManifest, SessionDef};

/// Validation errors collected during manifest validation.
#[derive(Debug)]
pub struct ValidationErrors {
    errors: Vec<String>,
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, err) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "  - {err}")?;
        }
        Ok(())
    }
}

/// Validate all semantic constraints of a [`JobManifest`].
///
/// Returns `Ok(())` if valid, or a [`crate::error::SmeltError::Manifest`] containing
/// all validation errors.
pub(super) fn validate_manifest(manifest: &JobManifest) -> crate::Result<()> {
    let mut errors = Vec::new();

    // job.name must not be empty and must be safe as a filesystem path segment —
    // it is joined directly into job state directory paths. Reject path separators
    // first; then reject the exact strings "." and ".." (path separators already
    // ruled out, so these are the only remaining single-component traversal values).
    if manifest.job.name.trim().is_empty() {
        errors.push("job.name: must not be empty".to_string());
    } else if manifest.job.name.contains('/') || manifest.job.name.contains('\\') {
        errors.push("job.name: must not contain path separators ('/' or '\\\\')".to_string());
    } else if manifest.job.name == ".." || manifest.job.name == "." {
        errors.push("job.name: must not be '.' or '..'".to_string());
    }

    // job.repo must not be empty
    if manifest.job.repo.trim().is_empty() {
        errors.push("job.repo: must not be empty".to_string());
    }

    // environment.image must not be empty
    if manifest.environment.image.trim().is_empty() {
        errors.push("environment.image: must not be empty".to_string());
    }

    // environment.runtime must be a known value
    const VALID_RUNTIMES: &[&str] = &["docker", "compose", "kubernetes"];
    if !VALID_RUNTIMES.contains(&manifest.environment.runtime.as_str()) {
        errors.push(format!(
            "environment.runtime: must be one of {:?}, got `{}`",
            VALID_RUNTIMES, manifest.environment.runtime
        ));
    }

    // kubernetes block requires kubernetes runtime and vice versa
    if manifest.environment.runtime == "kubernetes" {
        match &manifest.kubernetes {
            None => errors.push(
                "kubernetes: `runtime = \"kubernetes\"` requires a `[kubernetes]` block"
                    .to_string(),
            ),
            Some(k) => {
                if k.namespace.trim().is_empty() {
                    errors.push("kubernetes.namespace: must not be empty".to_string());
                }
                if k.ssh_key_env.trim().is_empty() {
                    errors.push("kubernetes.ssh_key_env: must not be empty".to_string());
                }
            }
        }
    } else if manifest.kubernetes.is_some() {
        errors.push(format!(
            "kubernetes: `[kubernetes]` block requires `runtime = \"kubernetes\"`, got `{}`",
            manifest.environment.runtime
        ));
    }

    // services entries require compose runtime
    if manifest.environment.runtime != "compose" && !manifest.services.is_empty() {
        errors.push(format!(
            "services: `[[services]]` entries require `runtime = \"compose\"`, got `{}`",
            manifest.environment.runtime
        ));
    }

    // At least one session required
    if manifest.session.is_empty() {
        errors.push("session: at least one session is required".to_string());
    }

    // Unique session names
    let mut seen_names = HashSet::new();
    for (i, sess) in manifest.session.iter().enumerate() {
        if sess.name.trim().is_empty() {
            errors.push(format!("session[{i}].name: must not be empty"));
        } else if !seen_names.insert(&sess.name) {
            errors.push(format!(
                "session[{i}].name: duplicate session name `{}`",
                sess.name
            ));
        }

        // timeout > 0
        if sess.timeout == 0 {
            errors.push(format!("session[{i}].timeout: must be > 0"));
        }
    }

    // depends_on references must be valid and no self-references
    let all_names: HashSet<&str> = manifest.session.iter().map(|s| s.name.as_str()).collect();
    for (i, sess) in manifest.session.iter().enumerate() {
        for dep in &sess.depends_on {
            if dep == &sess.name {
                errors.push(format!(
                    "session[{i}].depends_on: `{}` cannot depend on itself",
                    sess.name
                ));
            } else if !all_names.contains(dep.as_str()) {
                errors.push(format!("session[{i}].depends_on: unknown session `{dep}`"));
            }
        }
    }

    // Check for circular dependencies
    if let Some(cycle) = detect_cycle(&manifest.session) {
        errors.push(format!("session dependencies: cycle detected: {cycle}"));
    }

    // Per-service validation (only enforced when runtime is compose)
    if manifest.environment.runtime == "compose" {
        for (i, svc) in manifest.services.iter().enumerate() {
            if svc.name.trim().is_empty() {
                errors.push(format!("services[{i}].name: must not be empty"));
            }
            if svc.image.trim().is_empty() {
                errors.push(format!("services[{i}].image: must not be empty"));
            }
        }
    }

    // merge.target must not be empty
    if manifest.merge.target.trim().is_empty() {
        errors.push("merge.target: must not be empty".to_string());
    }

    // merge.order entries must reference valid sessions
    for entry in &manifest.merge.order {
        if !all_names.contains(entry.as_str()) {
            errors.push(format!("merge.order: unknown session `{entry}`"));
        }
    }

    // Validate forge structure but not token value or repo existence
    // (those are runtime concerns, resolved at execution time).
    if let Some(ref forge) = manifest.forge {
        if forge.token_env.trim().is_empty() {
            errors.push("forge.token_env: must not be empty".to_string());
        }
        let valid_repo = forge
            .repo
            .split_once('/')
            .map(|(owner, name)| !owner.is_empty() && !name.is_empty())
            .unwrap_or(false);
        if !valid_repo {
            errors.push(format!(
                "forge.repo: must be in `owner/repo` format, got `{}`",
                forge.repo
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        let detail = ValidationErrors { errors };
        Err(SmeltError::Manifest {
            field: "validation".to_string(),
            message: format!("manifest validation failed:\n{detail}"),
        })
    }
}

/// Detect cycles in session dependency graph using DFS.
/// Returns a description of the cycle if found.
fn detect_cycle(sessions: &[SessionDef]) -> Option<String> {
    let name_to_idx: HashMap<&str, usize> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();

    // 0 = unvisited, 1 = in-progress, 2 = done
    let mut state = vec![0u8; sessions.len()];

    fn dfs(
        idx: usize,
        sessions: &[SessionDef],
        name_to_idx: &HashMap<&str, usize>,
        state: &mut Vec<u8>,
        path: &mut Vec<String>,
    ) -> Option<String> {
        state[idx] = 1;
        path.push(sessions[idx].name.clone());

        for dep in &sessions[idx].depends_on {
            if let Some(&dep_idx) = name_to_idx.get(dep.as_str()) {
                if state[dep_idx] == 1 {
                    // Found a cycle — build the cycle path
                    path.push(dep.clone());
                    return Some(path.join(" -> "));
                }
                if state[dep_idx] == 0
                    && let Some(cycle) = dfs(dep_idx, sessions, name_to_idx, state, path)
                {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        state[idx] = 2;
        None
    }

    let mut path = Vec::new();
    for i in 0..sessions.len() {
        if state[i] == 0
            && let Some(cycle) = dfs(i, sessions, &name_to_idx, &mut state, &mut path)
        {
            return Some(cycle);
        }
    }
    None
}
