//! Gate composition: slug validation, criteria library I/O, and resolution.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use assay_types::{CriteriaLibrary, CriterionSource, GatesSpec, ResolvedCriterion, ResolvedGate};

use crate::error::{AssayError, Result};

/// Validate a criteria library or gate slug.
///
/// A valid slug:
/// - Is non-empty
/// - Is at most 64 characters long
/// - Consists only of ASCII lowercase letters (`a-z`), digits (`0-9`),
///   hyphens (`-`), and underscores (`_`)
/// - The first character must be an ASCII lowercase letter or digit (`[a-z0-9]`)
///
/// Returns `Ok(())` if valid, or `Err(AssayError::InvalidSlug)` describing
/// the specific violation.
pub fn validate_slug(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(AssayError::InvalidSlug {
            slug: value.to_string(),
            reason: "slug must not be empty".to_string(),
        });
    }

    if value.len() > 64 {
        return Err(AssayError::InvalidSlug {
            slug: value.to_string(),
            reason: format!("slug must be at most 64 characters, got {}", value.len()),
        });
    }

    let first = value.chars().next().expect("non-empty checked above");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(AssayError::InvalidSlug {
            slug: value.to_string(),
            reason: "first character must be an ASCII lowercase letter or digit".to_string(),
        });
    }

    for ch in value.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' && ch != '_' {
            return Err(AssayError::InvalidSlug {
                slug: value.to_string(),
                reason: format!(
                    "invalid character '{ch}': only lowercase letters, digits, hyphens, and underscores are allowed"
                ),
            });
        }
    }

    Ok(())
}

/// Load a criteria library from a TOML file.
///
/// Mirrors the `load_gates` pattern: reads the file, deserialises via toml with
/// `format_toml_error` for rich parse diagnostics. No additional semantic
/// validation step — `CriteriaLibrary`'s `deny_unknown_fields` handles schema
/// enforcement at parse time.
pub fn load_library(path: &Path) -> Result<CriteriaLibrary> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading criteria library".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let lib: CriteriaLibrary = toml::from_str(&content).map_err(|e| AssayError::LibraryParse {
        path: path.to_path_buf(),
        message: crate::config::format_toml_error(&content, &e),
    })?;

    Ok(lib)
}

/// Save a criteria library to `.assay/criteria/<slug>.toml` atomically.
///
/// Validates the library's `name` field as a slug before any I/O.
/// Uses `NamedTempFile` → `write_all` → `sync_all` → `persist` for atomicity.
///
/// Returns the path of the written file on success.
pub fn save_library(assay_dir: &Path, lib: &CriteriaLibrary) -> Result<PathBuf> {
    validate_slug(&lib.name)?;

    let criteria_dir = assay_dir.join("criteria");
    std::fs::create_dir_all(&criteria_dir).map_err(|source| AssayError::Io {
        operation: "creating criteria directory".into(),
        path: criteria_dir.clone(),
        source,
    })?;

    let toml_str = toml::to_string_pretty(lib).map_err(|e| AssayError::LibraryParse {
        path: criteria_dir.join(format!("{}.toml", lib.name)),
        message: e.to_string(),
    })?;

    let final_path = criteria_dir.join(format!("{}.toml", lib.name));

    use std::io::Write as _;
    use tempfile::NamedTempFile;
    let mut tmpfile = NamedTempFile::new_in(&criteria_dir).map_err(|source| AssayError::Io {
        operation: "creating temp file for criteria library".into(),
        path: criteria_dir.clone(),
        source,
    })?;
    tmpfile
        .write_all(toml_str.as_bytes())
        .map_err(|source| AssayError::Io {
            operation: "writing criteria library content".into(),
            path: criteria_dir.clone(),
            source,
        })?;
    tmpfile
        .as_file()
        .sync_all()
        .map_err(|source| AssayError::Io {
            operation: "syncing criteria library file".into(),
            path: criteria_dir.clone(),
            source,
        })?;
    tmpfile
        .persist(&final_path)
        .map_err(|e| AssayError::io("persisting criteria library", &final_path, e.error))?;

    Ok(final_path)
}

/// Scan all criteria libraries in `.assay/criteria/`.
///
/// Returns `Ok(vec![])` if the criteria directory does not exist.
/// Skips non-`.toml` files and silently ignores parse errors (consistent with
/// `scan()` in `spec/mod.rs`). Returns libraries sorted by name.
pub fn scan_libraries(assay_dir: &Path) -> Result<Vec<CriteriaLibrary>> {
    let criteria_dir = assay_dir.join("criteria");
    if !criteria_dir.is_dir() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&criteria_dir).map_err(|source| AssayError::Io {
        operation: "reading criteria directory".into(),
        path: criteria_dir.clone(),
        source,
    })?;

    let mut libs: Vec<CriteriaLibrary> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|e| e == "toml")
                .unwrap_or(false)
        })
        .filter_map(|entry| load_library(&entry.path()).ok())
        .collect();

    libs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(libs)
}

/// Load a criteria library by slug from `.assay/criteria/<slug>.toml`.
///
/// Validates the slug, then attempts to load the file. If the file doesn't
/// exist, scans available slugs and provides a fuzzy-match suggestion.
pub fn load_library_by_slug(assay_dir: &Path, slug: &str) -> Result<CriteriaLibrary> {
    validate_slug(slug)?;

    let criteria_dir = assay_dir.join("criteria");
    let path = criteria_dir.join(format!("{slug}.toml"));

    if !path.exists() {
        // Collect available slugs for fuzzy suggestion
        let available: Vec<String> = if criteria_dir.is_dir() {
            std::fs::read_dir(&criteria_dir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "toml")
                        .unwrap_or(false)
                })
                .filter_map(|e| {
                    e.path()
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .collect()
        } else {
            vec![]
        };

        let suggestion = crate::spec::find_fuzzy_match(slug, &available);

        return Err(AssayError::LibraryNotFound {
            slug: slug.to_string(),
            criteria_dir,
            suggestion,
        });
    }

    load_library(&path)
}

/// Resolve a gate's effective criteria by merging parent, library, and own criteria.
///
/// Given a `gate` (loaded from `gates.toml`) and its `gate_slug` (the directory name,
/// used as its identity), this function:
///
/// 1. Validates any `extends` and `include` slugs.
/// 2. Detects circular `extends` chains (self-extend and mutual extend).
/// 3. Loads the parent gate's criteria via `load_gate` (if `extends` is set).
/// 4. Loads each library's criteria via `load_library` (for each `include` slug).
/// 5. Merges all criteria in order: **parent → libraries → own**.
///    Name collisions are resolved with **own-wins** semantics (the later entry overwrites
///    an earlier one with the same `name`).
///
/// # Single-level inheritance
///
/// Only one level of `extends` is followed. The parent's own `extends` and `include`
/// fields are **ignored** — parent criteria are taken as-is from the parent's own
/// `criteria` list.
///
/// # Closure interfaces
///
/// Both closures return errors directly; `resolve` propagates them unchanged.
/// - `load_gate(slug) -> Result<GatesSpec>`: Load a gate by its slug.
/// - `load_library(slug) -> Result<CriteriaLibrary>`: Load a library by its slug.
///
/// # Errors
///
/// - [`AssayError::InvalidSlug`] — `extends` or an `include` slug is syntactically invalid.
/// - [`AssayError::CycleDetected`] — `extends` creates a cycle (self or mutual).
/// - [`AssayError::ParentGateNotFound`] — the `extends` slug was not found (propagated
///   from `load_gate`; callers should return this error type).
/// - Any error returned from `load_gate` or `load_library` closures.
pub fn resolve(
    gate: &GatesSpec,
    gate_slug: &str,
    load_gate: impl Fn(&str) -> Result<GatesSpec>,
    load_library: impl Fn(&str) -> Result<CriteriaLibrary>,
) -> Result<ResolvedGate> {
    // ── 1. Validate slugs ─────────────────────────────────────────────────────
    if let Some(extends_slug) = &gate.extends {
        validate_slug(extends_slug)?;
    }
    for include_slug in &gate.include {
        validate_slug(include_slug)?;
    }

    // ── 2. Cycle detection + parent loading ───────────────────────────────────
    let parent_criteria: Vec<(assay_types::Criterion, CriterionSource)> =
        if let Some(extends_slug) = &gate.extends {
            // Self-extend: gate_slug == extends_slug
            if extends_slug == gate_slug {
                return Err(AssayError::CycleDetected {
                    gate_slug: gate_slug.to_string(),
                    parent_slug: extends_slug.clone(),
                });
            }

            let parent = load_gate(extends_slug)?;

            // Mutual extend: parent.extends == Some(gate_slug)
            if parent.extends.as_deref() == Some(gate_slug) {
                return Err(AssayError::CycleDetected {
                    gate_slug: gate_slug.to_string(),
                    parent_slug: extends_slug.clone(),
                });
            }

            // Take parent's OWN criteria only (single-level: parent's extends/include ignored)
            parent
                .criteria
                .into_iter()
                .map(|c| {
                    let source = CriterionSource::Parent {
                        gate_slug: extends_slug.clone(),
                    };
                    (c, source)
                })
                .collect()
        } else {
            vec![]
        };

    // ── 3. Library criteria ───────────────────────────────────────────────────
    let mut library_criteria: Vec<(assay_types::Criterion, CriterionSource)> = vec![];
    for include_slug in &gate.include {
        let lib = load_library(include_slug)?;
        for c in lib.criteria {
            library_criteria.push((
                c,
                CriterionSource::Library {
                    slug: include_slug.clone(),
                },
            ));
        }
    }

    // ── 4. Merge with own-wins semantics ──────────────────────────────────────
    //
    // Strategy: collect all criteria into a Vec in order (parent → libraries → own),
    // then reverse-dedup by name (keeping the LAST occurrence, which is own if present,
    // or latest library). Finally, reverse the result to restore original ordering.
    //
    // This naturally gives us:
    //   - own-wins over parent and libraries
    //   - later-library-wins over earlier libraries
    //   - ordering within surviving entries follows insertion order
    let own_criteria: Vec<(assay_types::Criterion, CriterionSource)> = gate
        .criteria
        .iter()
        .map(|c| (c.clone(), CriterionSource::Own))
        .collect();

    let all_criteria: Vec<(assay_types::Criterion, CriterionSource)> = parent_criteria
        .into_iter()
        .chain(library_criteria)
        .chain(own_criteria)
        .collect();

    // Reverse-dedup: iterate from back, track seen names, keep first-seen (= last in forward order)
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped_rev: Vec<ResolvedCriterion> = all_criteria
        .into_iter()
        .rev()
        .filter(|(c, _)| seen.insert(c.name.clone()))
        .map(|(criterion, source)| ResolvedCriterion { criterion, source })
        .collect();

    // Restore forward order
    deduped_rev.reverse();

    Ok(ResolvedGate {
        gate_slug: gate_slug.to_string(),
        parent_slug: gate.extends.clone(),
        included_libraries: gate.include.clone(),
        criteria: deduped_rev,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve() tests ───────────────────────────────────────────────────────

    use assay_types::criterion::When as CriterionWhen;
    use assay_types::{CriterionSource, GatesSpec};

    fn make_criterion(name: &str) -> assay_types::Criterion {
        assay_types::Criterion {
            name: name.to_string(),
            description: format!("{name} desc"),
            cmd: Some(format!("echo {name}")),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
            when: CriterionWhen::default(),
        }
    }

    fn make_gate(name: &str, criteria_names: &[&str]) -> GatesSpec {
        GatesSpec {
            name: name.to_string(),
            description: String::new(),
            gate: None,
            depends: vec![],
            milestone: None,
            order: None,
            extends: None,
            include: vec![],
            preconditions: None,
            criteria: criteria_names.iter().map(|n| make_criterion(n)).collect(),
        }
    }

    fn no_gate(_slug: &str) -> Result<GatesSpec> {
        Err(AssayError::ParentGateNotFound {
            gate_slug: "self".to_string(),
            parent_slug: _slug.to_string(),
        })
    }

    fn no_library(_slug: &str) -> Result<CriteriaLibrary> {
        Err(AssayError::LibraryNotFound {
            slug: _slug.to_string(),
            criteria_dir: std::path::PathBuf::from(".assay/criteria"),
            suggestion: None,
        })
    }

    // Happy path: no extends, no include → own criteria only

    #[test]
    fn resolve_no_extends_no_include_returns_own_criteria() {
        let gate = make_gate("my-gate", &["compiles", "tests-pass"]);
        let resolved =
            resolve(&gate, "my-gate", no_gate, no_library).expect("resolve should succeed");

        assert_eq!(resolved.gate_slug, "my-gate");
        assert!(resolved.parent_slug.is_none());
        assert!(resolved.included_libraries.is_empty());
        assert_eq!(resolved.criteria.len(), 2);
        assert_eq!(resolved.criteria[0].criterion.name, "compiles");
        assert!(matches!(resolved.criteria[0].source, CriterionSource::Own));
        assert_eq!(resolved.criteria[1].criterion.name, "tests-pass");
        assert!(matches!(resolved.criteria[1].source, CriterionSource::Own));
    }

    // extends: parent criteria + own criteria with correct sources

    #[test]
    fn resolve_with_extends_includes_parent_criteria() {
        let mut gate = make_gate("child", &["own-check"]);
        gate.extends = Some("parent-gate".to_string());

        let parent = make_gate("parent", &["build", "lint"]);

        let resolved = resolve(
            &gate,
            "child",
            |slug| {
                assert_eq!(slug, "parent-gate");
                Ok(parent.clone())
            },
            no_library,
        )
        .expect("resolve should succeed");

        assert_eq!(resolved.gate_slug, "child");
        assert_eq!(resolved.parent_slug, Some("parent-gate".to_string()));
        assert_eq!(resolved.criteria.len(), 3); // build, lint (parent) + own-check (own)

        let build = &resolved.criteria[0];
        assert_eq!(build.criterion.name, "build");
        assert!(
            matches!(&build.source, CriterionSource::Parent { gate_slug } if gate_slug == "parent-gate"),
            "build should be from parent"
        );

        let own = &resolved.criteria[2];
        assert_eq!(own.criterion.name, "own-check");
        assert!(matches!(&own.source, CriterionSource::Own));
    }

    // include: library criteria + own criteria with correct sources

    #[test]
    fn resolve_with_include_merges_library_criteria() {
        let mut gate = make_gate("my-gate", &["own-check"]);
        gate.include = vec!["rust-basics".to_string()];

        let mut lib = make_library("rust-basics");
        lib.criteria = vec![make_criterion("compiles"), make_criterion("tests-pass")];

        let resolved = resolve(&gate, "my-gate", no_gate, |slug| {
            assert_eq!(slug, "rust-basics");
            Ok(lib.clone())
        })
        .expect("resolve should succeed");

        assert_eq!(resolved.included_libraries, vec!["rust-basics"]);
        assert_eq!(resolved.criteria.len(), 3); // compiles, tests-pass (lib) + own-check (own)

        let compiles = &resolved.criteria[0];
        assert_eq!(compiles.criterion.name, "compiles");
        assert!(
            matches!(&compiles.source, CriterionSource::Library { slug } if slug == "rust-basics")
        );

        let own = &resolved.criteria[2];
        assert_eq!(own.criterion.name, "own-check");
        assert!(matches!(&own.source, CriterionSource::Own));
    }

    // extends + include: all three sources merged in order

    #[test]
    fn resolve_with_extends_and_include_merges_all_sources() {
        let mut gate = make_gate("child", &["own-check"]);
        gate.extends = Some("parent-gate".to_string());
        gate.include = vec!["lib-a".to_string()];

        let parent = make_gate("parent", &["parent-crit"]);
        let mut lib = make_library("lib-a");
        lib.criteria = vec![make_criterion("lib-crit")];

        let resolved = resolve(&gate, "child", |_| Ok(parent.clone()), |_| Ok(lib.clone()))
            .expect("resolve should succeed");

        assert_eq!(resolved.criteria.len(), 3);
        assert_eq!(resolved.criteria[0].criterion.name, "parent-crit");
        assert!(matches!(
            &resolved.criteria[0].source,
            CriterionSource::Parent { .. }
        ));
        assert_eq!(resolved.criteria[1].criterion.name, "lib-crit");
        assert!(matches!(
            &resolved.criteria[1].source,
            CriterionSource::Library { .. }
        ));
        assert_eq!(resolved.criteria[2].criterion.name, "own-check");
        assert!(matches!(&resolved.criteria[2].source, CriterionSource::Own));
    }

    // own-wins: own criterion overrides parent criterion with same name

    #[test]
    fn resolve_own_wins_over_parent() {
        let mut gate = make_gate("child", &["compiles"]);
        gate.extends = Some("parent-gate".to_string());

        let parent = make_gate("parent", &["compiles", "lint"]);

        let resolved = resolve(&gate, "child", |_| Ok(parent.clone()), no_library)
            .expect("resolve should succeed");

        // "compiles" appears from both parent and own, own should win
        // "lint" is only from parent
        let compiles_entries: Vec<_> = resolved
            .criteria
            .iter()
            .filter(|c| c.criterion.name == "compiles")
            .collect();
        assert_eq!(
            compiles_entries.len(),
            1,
            "compiles should appear only once"
        );
        assert!(
            matches!(&compiles_entries[0].source, CriterionSource::Own),
            "own wins: compiles should have Own source"
        );

        let lint_entries: Vec<_> = resolved
            .criteria
            .iter()
            .filter(|c| c.criterion.name == "lint")
            .collect();
        assert_eq!(lint_entries.len(), 1);
        assert!(matches!(
            &lint_entries[0].source,
            CriterionSource::Parent { .. }
        ));
    }

    // own-wins: own criterion overrides library criterion with same name

    #[test]
    fn resolve_own_wins_over_library() {
        let mut gate = make_gate("my-gate", &["tests-pass"]);
        gate.include = vec!["lib-a".to_string()];

        let mut lib = make_library("lib-a");
        lib.criteria = vec![make_criterion("tests-pass"), make_criterion("lint")];

        let resolved = resolve(&gate, "my-gate", no_gate, |_| Ok(lib.clone()))
            .expect("resolve should succeed");

        let tests_pass_entries: Vec<_> = resolved
            .criteria
            .iter()
            .filter(|c| c.criterion.name == "tests-pass")
            .collect();
        assert_eq!(
            tests_pass_entries.len(),
            1,
            "tests-pass should appear only once"
        );
        assert!(
            matches!(&tests_pass_entries[0].source, CriterionSource::Own),
            "own wins over library"
        );
    }

    // later library wins when two libraries define the same criterion name

    #[test]
    fn resolve_later_library_wins_over_earlier_library() {
        let mut gate = make_gate("my-gate", &["own-check"]);
        gate.include = vec!["lib-a".to_string(), "lib-b".to_string()];

        let mut lib_a = make_library("lib-a");
        lib_a.criteria = vec![make_criterion("shared")];
        let mut lib_b = make_library("lib-b");
        lib_b.criteria = vec![make_criterion("shared")];

        let resolved = resolve(&gate, "my-gate", no_gate, |slug| match slug {
            "lib-a" => Ok(lib_a.clone()),
            "lib-b" => Ok(lib_b.clone()),
            _ => no_library(slug),
        })
        .expect("resolve should succeed");

        let shared_entries: Vec<_> = resolved
            .criteria
            .iter()
            .filter(|c| c.criterion.name == "shared")
            .collect();
        assert_eq!(shared_entries.len(), 1, "shared should appear only once");
        assert!(
            matches!(&shared_entries[0].source, CriterionSource::Library { slug } if slug == "lib-b"),
            "later library (lib-b) should win"
        );
    }

    // Cycle detection: self-extend

    #[test]
    fn resolve_self_extend_returns_cycle_detected() {
        let mut gate = make_gate("my-gate", &["compiles"]);
        gate.extends = Some("my-gate".to_string()); // self-extend

        let err = resolve(&gate, "my-gate", no_gate, no_library).unwrap_err();
        assert!(
            matches!(&err, AssayError::CycleDetected { gate_slug, parent_slug }
                if gate_slug == "my-gate" && parent_slug == "my-gate"),
            "expected CycleDetected, got: {err:?}"
        );
    }

    // Cycle detection: mutual extend (A extends B, B extends A)

    #[test]
    fn resolve_mutual_extend_returns_cycle_detected() {
        let mut gate_a = make_gate("gate-a", &["compiles"]);
        gate_a.extends = Some("gate-b".to_string());

        let mut gate_b = make_gate("gate-b", &["lint"]);
        gate_b.extends = Some("gate-a".to_string()); // closes the cycle

        let err = resolve(
            &gate_a,
            "gate-a",
            |slug| {
                assert_eq!(slug, "gate-b");
                Ok(gate_b.clone())
            },
            no_library,
        )
        .unwrap_err();

        assert!(
            matches!(&err, AssayError::CycleDetected { gate_slug, parent_slug }
                if gate_slug == "gate-a" && parent_slug == "gate-b"),
            "expected CycleDetected for mutual extend, got: {err:?}"
        );
    }

    // Invalid slug in extends

    #[test]
    fn resolve_invalid_extends_slug_returns_invalid_slug() {
        let mut gate = make_gate("my-gate", &["compiles"]);
        gate.extends = Some("INVALID-SLUG!".to_string());

        let err = resolve(&gate, "my-gate", no_gate, no_library).unwrap_err();
        assert!(
            matches!(&err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    // Invalid slug in include

    #[test]
    fn resolve_invalid_include_slug_returns_invalid_slug() {
        let mut gate = make_gate("my-gate", &["compiles"]);
        gate.include = vec!["INVALID!".to_string()];

        let err = resolve(&gate, "my-gate", no_gate, no_library).unwrap_err();
        assert!(
            matches!(&err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug for invalid include slug, got: {err:?}"
        );
    }

    // Missing parent gate → ParentGateNotFound

    #[test]
    fn resolve_missing_parent_returns_parent_gate_not_found() {
        let mut gate = make_gate("child", &["compiles"]);
        gate.extends = Some("nonexistent-parent".to_string());

        let err = resolve(
            &gate,
            "child",
            |_slug| {
                Err(AssayError::ParentGateNotFound {
                    gate_slug: "child".to_string(),
                    parent_slug: "nonexistent-parent".to_string(),
                })
            },
            no_library,
        )
        .unwrap_err();

        assert!(
            matches!(&err, AssayError::ParentGateNotFound { gate_slug, parent_slug }
                if gate_slug == "child" && parent_slug == "nonexistent-parent"),
            "expected ParentGateNotFound, got: {err:?}"
        );
    }

    // Missing library → error propagated from load_library closure

    #[test]
    fn resolve_missing_library_propagates_error() {
        let mut gate = make_gate("my-gate", &["compiles"]);
        gate.include = vec!["missing-lib".to_string()];

        let err = resolve(&gate, "my-gate", no_gate, |slug| {
            Err(AssayError::LibraryNotFound {
                slug: slug.to_string(),
                criteria_dir: std::path::PathBuf::from(".assay/criteria"),
                suggestion: None,
            })
        })
        .unwrap_err();

        assert!(
            matches!(&err, AssayError::LibraryNotFound { slug, .. } if slug == "missing-lib"),
            "expected LibraryNotFound, got: {err:?}"
        );
    }

    // Edge: empty own criteria + extends with criteria → all parent criteria present

    #[test]
    fn resolve_empty_own_with_extends_has_all_parent_criteria() {
        let mut gate = make_gate("child", &[]); // empty own criteria
        gate.extends = Some("parent-gate".to_string());

        let parent = make_gate("parent", &["build", "lint", "test"]);

        let resolved = resolve(&gate, "child", |_| Ok(parent.clone()), no_library)
            .expect("resolve should succeed");

        assert_eq!(resolved.criteria.len(), 3);
        assert!(
            resolved
                .criteria
                .iter()
                .all(|c| matches!(c.source, CriterionSource::Parent { .. }))
        );
    }

    // Edge: gate with criteria + empty include → no library criteria

    #[test]
    fn resolve_empty_include_has_no_library_criteria() {
        let gate = make_gate("my-gate", &["compiles"]); // no include

        let resolved =
            resolve(&gate, "my-gate", no_gate, no_library).expect("resolve should succeed");

        assert!(resolved.included_libraries.is_empty());
        assert_eq!(resolved.criteria.len(), 1);
        assert!(matches!(&resolved.criteria[0].source, CriterionSource::Own));
    }

    // Ordering: parent first, then library, then own (each group in original order)

    #[test]
    fn resolve_ordering_parent_then_library_then_own() {
        let mut gate = make_gate("child", &["own-a", "own-b"]);
        gate.extends = Some("parent-gate".to_string());
        gate.include = vec!["lib-one".to_string()];

        let parent = make_gate("parent", &["parent-x", "parent-y"]);
        let mut lib = make_library("lib-one");
        lib.criteria = vec![make_criterion("lib-p"), make_criterion("lib-q")];

        let resolved = resolve(&gate, "child", |_| Ok(parent.clone()), |_| Ok(lib.clone()))
            .expect("resolve should succeed");

        assert_eq!(resolved.criteria.len(), 6);
        assert_eq!(resolved.criteria[0].criterion.name, "parent-x");
        assert_eq!(resolved.criteria[1].criterion.name, "parent-y");
        assert_eq!(resolved.criteria[2].criterion.name, "lib-p");
        assert_eq!(resolved.criteria[3].criterion.name, "lib-q");
        assert_eq!(resolved.criteria[4].criterion.name, "own-a");
        assert_eq!(resolved.criteria[5].criterion.name, "own-b");
    }

    // Single-level only: parent's extends is ignored

    #[test]
    fn resolve_single_level_only_parent_extends_ignored() {
        let mut gate = make_gate("child", &["own-check"]);
        gate.extends = Some("parent-gate".to_string());

        // parent also has extends — but this should NOT be followed (single-level decision)
        let mut parent = make_gate("parent", &["parent-crit"]);
        parent.extends = Some("grandparent".to_string());

        // grandparent resolver should never be called
        let resolved = resolve(
            &gate,
            "child",
            |slug| {
                assert_eq!(
                    slug, "parent-gate",
                    "only parent-gate should be loaded, not grandparent"
                );
                Ok(parent.clone())
            },
            no_library,
        )
        .expect("resolve should succeed");

        // Only parent-crit and own-check; grandparent criteria absent
        assert_eq!(resolved.criteria.len(), 2);
        let names: Vec<_> = resolved
            .criteria
            .iter()
            .map(|c| c.criterion.name.as_str())
            .collect();
        assert_eq!(names, vec!["parent-crit", "own-check"]);
    }

    // ── validate_slug tests ────────────────────────────────────────────────────

    #[test]
    fn validate_slug_rust_basics_ok() {
        assert!(validate_slug("rust-basics").is_ok());
    }

    #[test]
    fn validate_slug_underscore_ok() {
        assert!(validate_slug("my_lib").is_ok());
    }

    #[test]
    fn validate_slug_starts_with_digit_ok() {
        assert!(validate_slug("0starts-with-digit").is_ok());
    }

    #[test]
    fn validate_slug_empty_err() {
        let err = validate_slug("").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_uppercase_err() {
        let err = validate_slug("A-Upper").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_path_traversal_err() {
        let err = validate_slug("../evil").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_too_long_err() {
        let slug = "a".repeat(65);
        let err = validate_slug(&slug).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    #[test]
    fn validate_slug_max_length_ok() {
        let slug = "a".repeat(64);
        assert!(validate_slug(&slug).is_ok(), "64 chars should be accepted");
    }

    #[test]
    fn validate_slug_starts_with_dash_err() {
        let err = validate_slug("-starts-with-dash").unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { ref slug, .. } if slug == "-starts-with-dash"),
            "expected InvalidSlug, got: {err:?}"
        );
    }

    // ── library I/O tests ────────────────────────────────────────────────────

    use assay_types::criterion::When;
    use assay_types::{CriteriaLibrary, Criterion};

    fn make_library(name: &str) -> CriteriaLibrary {
        CriteriaLibrary {
            name: name.to_string(),
            description: "Test library".to_string(),
            version: Some("1.0.0".to_string()),
            tags: vec!["test".to_string()],
            criteria: vec![Criterion {
                name: "compiles".to_string(),
                description: "Code compiles".to_string(),
                cmd: Some("cargo build".to_string()),
                path: None,
                timeout: None,
                enforcement: None,
                kind: None,
                prompt: None,
                requirements: vec![],
                when: When::default(),
            }],
        }
    }

    #[test]
    fn load_library_valid_toml() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        let toml_str = toml::to_string_pretty(&lib).expect("serialize");
        let path = tmp.path().join("rust-basics.toml");
        std::fs::write(&path, &toml_str).expect("write");
        let loaded = load_library(&path).expect("load_library");
        assert_eq!(loaded, lib);
    }

    #[test]
    fn load_library_invalid_toml_returns_library_parse_err() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("bad.toml");
        std::fs::write(&path, "not valid toml = [[[").expect("write bad toml");
        let err = load_library(&path).unwrap_err();
        assert!(
            matches!(err, AssayError::LibraryParse { .. }),
            "expected LibraryParse, got: {err:?}"
        );
    }

    #[test]
    fn load_library_nonexistent_returns_io_err() {
        let path = std::path::Path::new("/tmp/nonexistent-assay-test-abc123.toml");
        let err = load_library(path).unwrap_err();
        assert!(
            matches!(err, AssayError::Io { .. }),
            "expected Io error, got: {err:?}"
        );
    }

    #[test]
    fn save_library_valid_slug_writes_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        let path = save_library(tmp.path(), &lib).expect("save_library");
        assert!(path.exists(), "file should exist after save");
        assert_eq!(path, tmp.path().join("criteria/rust-basics.toml"));
    }

    #[test]
    fn save_library_invalid_slug_returns_err_before_io() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut lib = make_library("rust-basics");
        lib.name = "INVALID-SLUG".to_string();
        let err = save_library(tmp.path(), &lib).unwrap_err();
        assert!(
            matches!(err, AssayError::InvalidSlug { .. }),
            "expected InvalidSlug, got: {err:?}"
        );
        // No criteria directory should have been created
        assert!(
            !tmp.path().join("criteria").exists(),
            "criteria dir should not be created on slug validation failure"
        );
    }

    #[test]
    fn save_and_load_library_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        let path = save_library(tmp.path(), &lib).expect("save_library");
        let loaded = load_library(&path).expect("load_library");
        assert_eq!(loaded, lib, "roundtrip should preserve all fields");
    }

    #[test]
    fn scan_libraries_missing_dir_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = scan_libraries(tmp.path()).expect("scan_libraries");
        assert!(
            result.is_empty(),
            "should return empty for missing criteria dir"
        );
    }

    #[test]
    fn scan_libraries_returns_all_toml_files_sorted() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib_a = make_library("aaa-lib");
        let lib_b = make_library("bbb-lib");
        save_library(tmp.path(), &lib_b).expect("save bbb");
        save_library(tmp.path(), &lib_a).expect("save aaa");

        let result = scan_libraries(tmp.path()).expect("scan_libraries");
        assert_eq!(result.len(), 2, "should find 2 libraries");
        assert_eq!(result[0].name, "aaa-lib", "should be sorted by name");
        assert_eq!(result[1].name, "bbb-lib");
    }

    #[test]
    fn scan_libraries_skips_non_toml_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let criteria_dir = tmp.path().join("criteria");
        std::fs::create_dir_all(&criteria_dir).expect("create criteria dir");
        std::fs::write(criteria_dir.join("ignored.json"), r#"{"name":"test"}"#)
            .expect("write json");
        std::fs::write(criteria_dir.join("ignored.txt"), "text file").expect("write txt");

        let lib = make_library("valid-lib");
        save_library(tmp.path(), &lib).expect("save valid-lib");

        let result = scan_libraries(tmp.path()).expect("scan_libraries");
        assert_eq!(result.len(), 1, "should only load .toml files");
        assert_eq!(result[0].name, "valid-lib");
    }

    #[test]
    fn load_library_by_slug_existing_returns_library() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        save_library(tmp.path(), &lib).expect("save");
        let loaded = load_library_by_slug(tmp.path(), "rust-basics").expect("load by slug");
        assert_eq!(loaded, lib);
    }

    #[test]
    fn load_library_by_slug_missing_returns_library_not_found_with_suggestion() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let lib = make_library("rust-basics");
        save_library(tmp.path(), &lib).expect("save");

        // Slightly misspelled — should get fuzzy suggestion
        let err = load_library_by_slug(tmp.path(), "rust-bascs").unwrap_err();
        match err {
            AssayError::LibraryNotFound {
                slug, suggestion, ..
            } => {
                assert_eq!(slug, "rust-bascs");
                assert_eq!(
                    suggestion,
                    Some("rust-basics".to_string()),
                    "expected fuzzy suggestion"
                );
            }
            other => panic!("expected LibraryNotFound, got: {other:?}"),
        }
    }
}
