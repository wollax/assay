//! Structured spec validation with diagnostic output.
//!
//! Core validation (`validate()`/`validate_gates_spec()`) runs during
//! `load_spec_entry_with_diagnostics()` — this module converts those errors
//! to `Vec<Diagnostic>` and layers on additional checks:
//!
//! - AgentReport prompt presence (warning)
//! - Command binary existence on PATH (opt-in)
//! - Dependency cycle detection (cross-spec)

use std::collections::{HashMap, HashSet};

use assay_types::{Diagnostic, DiagnosticSummary, Severity, ValidationResult};

use super::SpecError;

/// Convert existing SpecError vec to Diagnostic vec.
/// All SpecErrors are error-severity (they block validity).
pub fn spec_errors_to_diagnostics(errors: &[SpecError]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|e| Diagnostic {
            severity: Severity::Error,
            location: e.field.clone(),
            message: e.message.clone(),
        })
        .collect()
}

/// Check that AgentReport criteria have a non-empty `prompt` field.
/// Missing prompt is a warning (spec is usable but agent won't have guidance).
fn validate_agent_prompts(criteria: &[assay_types::Criterion]) -> Vec<Diagnostic> {
    criteria
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if c.kind == Some(assay_types::CriterionKind::AgentReport) {
                let has_prompt = c.prompt.as_ref().is_some_and(|p| !p.trim().is_empty());
                if !has_prompt {
                    return Some(Diagnostic {
                        severity: Severity::Warning,
                        location: format!("criteria[{i}].prompt"),
                        message: format!(
                            "criterion `{}` has kind=AgentReport but no prompt; agent will lack evaluation guidance",
                            c.name
                        ),
                    });
                }
            }
            None
        })
        .collect()
}

/// Validate that command binaries exist on PATH.
///
/// Uses `which::which()` for cross-platform lookup and `extract_binary()` from
/// the gate module to extract the binary name from a command string.
///
/// Produces `Severity::Warning` diagnostics (never errors), since the command
/// may exist in the execution environment but not the validation environment.
fn validate_commands(criteria: &[assay_types::Criterion]) -> Vec<Diagnostic> {
    criteria
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let cmd = c.cmd.as_ref()?;
            let binary = crate::gate::extract_binary(cmd);
            if binary.is_empty() {
                return Some(Diagnostic {
                    severity: Severity::Warning,
                    location: format!("criteria[{i}].cmd"),
                    message: format!("criterion `{}` has empty command string", c.name),
                });
            }
            match which::which(binary) {
                Ok(_) => None,
                Err(e) => Some(Diagnostic {
                    severity: Severity::Warning,
                    location: format!("criteria[{i}].cmd"),
                    message: format!(
                        "command `{binary}` not found on PATH (criterion `{}`): {e}",
                        c.name
                    ),
                }),
            }
        })
        .collect()
}

/// Color markers for DFS cycle detection.
#[derive(Clone, Copy, PartialEq)]
enum Color {
    White,
    Gray,
    Black,
}

/// Detect dependency cycles and unknown references across all provided specs.
///
/// Returns two kinds of diagnostics:
/// - `Severity::Error` for each cycle found (with the full cycle path).
/// - `Severity::Warning` for any dependency that references an unknown spec slug.
///
/// Each diagnostic carries a `specs` set indicating which spec slugs are involved,
/// enabling callers to filter diagnostics by spec without substring matching.
///
/// `specs` is a map from spec slug to its declared dependencies.
pub(crate) fn detect_cycles(specs: &HashMap<String, Vec<String>>) -> Vec<CycleDiagnostic> {
    let mut colors: HashMap<&str, Color> =
        specs.keys().map(|k| (k.as_str(), Color::White)).collect();
    let mut path: Vec<&str> = Vec::new();
    let mut diagnostics = Vec::new();

    for name in specs.keys() {
        if colors[name.as_str()] == Color::White {
            dfs(name, specs, &mut colors, &mut path, &mut diagnostics);
        }
    }

    // Also check for dependencies referencing unknown specs
    let known: HashSet<&str> = specs.keys().map(|s| s.as_str()).collect();
    for (name, deps) in specs {
        for (i, dep) in deps.iter().enumerate() {
            if !known.contains(dep.as_str()) {
                let mut involved = HashSet::new();
                involved.insert(name.clone());
                diagnostics.push(CycleDiagnostic {
                    diagnostic: Diagnostic {
                        severity: Severity::Warning,
                        location: format!("depends[{i}]"),
                        message: format!("spec `{name}` depends on `{dep}` which was not found"),
                    },
                    specs: involved,
                });
            }
        }
    }

    diagnostics
}

/// A diagnostic from cycle detection, with the set of spec slugs involved.
pub(crate) struct CycleDiagnostic {
    pub diagnostic: Diagnostic,
    pub specs: HashSet<String>,
}

fn dfs<'a>(
    node: &'a str,
    graph: &'a HashMap<String, Vec<String>>,
    colors: &mut HashMap<&'a str, Color>,
    path: &mut Vec<&'a str>,
    diagnostics: &mut Vec<CycleDiagnostic>,
) {
    colors.insert(node, Color::Gray);
    path.push(node);

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if let Some(&color) = colors.get(dep.as_str()) {
                match color {
                    Color::Gray => {
                        // Found a cycle — extract the cycle path
                        let Some(cycle_start) = path.iter().position(|&n| n == dep.as_str()) else {
                            // Invariant violated — skip rather than panic
                            continue;
                        };
                        let cycle: Vec<&str> = path[cycle_start..].to_vec();
                        let involved: HashSet<String> =
                            cycle.iter().map(|s| s.to_string()).collect();
                        let cycle_display: Vec<String> = cycle
                            .iter()
                            .chain(std::iter::once(&dep.as_str()))
                            .map(|s| s.to_string())
                            .collect();
                        diagnostics.push(CycleDiagnostic {
                            diagnostic: Diagnostic {
                                severity: Severity::Error,
                                location: "depends".to_string(),
                                message: format!(
                                    "circular dependency detected: {}",
                                    cycle_display.join(" -> ")
                                ),
                            },
                            specs: involved,
                        });
                    }
                    Color::White => {
                        dfs(dep, graph, colors, path, diagnostics);
                    }
                    Color::Black => {} // Already fully explored
                }
            }
            // If dep not in colors, it's an unknown spec (handled separately)
        }
    }

    path.pop();
    colors.insert(node, Color::Black);
}

/// Run additional validation checks on an already-loaded spec entry.
///
/// This function does NOT re-run core validation (`validate()`/`validate_gates_spec()`),
/// since `load_spec_entry` already performs that. Instead it layers on checks that
/// the loader does not cover:
/// - AgentReport prompt presence (warning)
/// - Command existence on PATH (opt-in via `check_commands`)
///
/// Cycle detection is handled separately via [`detect_cycles()`] since it
/// requires loading all specs.
///
/// For specs that failed to load (TOML parse, core validation errors), the MCP
/// handler constructs `ValidationResult` directly from the error — this function
/// is only called on the success path.
pub fn validate_spec(entry: &super::SpecEntry, check_commands: bool) -> ValidationResult {
    let (slug, criteria) = match entry {
        super::SpecEntry::Legacy { slug, spec } => (slug.clone(), spec.criteria.as_slice()),
        super::SpecEntry::Directory { slug, gates, .. } => {
            (slug.clone(), gates.criteria.as_slice())
        }
    };

    let mut diagnostics = Vec::new();

    // Additional checks beyond what load_spec_entry validates
    diagnostics.extend(validate_agent_prompts(criteria));

    if check_commands {
        diagnostics.extend(validate_commands(criteria));
    }

    let summary = build_summary(&diagnostics);
    let valid = summary.errors == 0;

    ValidationResult {
        spec: slug,
        valid,
        diagnostics,
        summary,
    }
}

/// Count diagnostics by severity level into a [`DiagnosticSummary`].
pub fn build_summary(diagnostics: &[Diagnostic]) -> DiagnosticSummary {
    let mut errors = 0;
    let mut warnings = 0;
    let mut info = 0;
    for d in diagnostics {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => info += 1,
        }
    }
    DiagnosticSummary {
        errors,
        warnings,
        info,
    }
}

/// Validate a single spec and optionally check cross-spec dependencies.
///
/// When the target spec declares a non-empty `depends` list, loads ALL specs from
/// `specs_dir` to build a dependency graph and check for cycles.
///
/// If loading specs from `specs_dir` fails (e.g., I/O error), a warning
/// diagnostic is emitted indicating that cycle detection was skipped.
pub fn validate_spec_with_dependencies(
    entry: &super::SpecEntry,
    check_commands: bool,
    specs_dir: &std::path::Path,
) -> ValidationResult {
    let mut result = validate_spec(entry, check_commands);

    // Get depends from the entry
    let depends = match entry {
        super::SpecEntry::Legacy { spec, .. } => &spec.depends,
        super::SpecEntry::Directory { gates, .. } => &gates.depends,
    };

    // Only do cycle detection if this spec has dependencies
    if !depends.is_empty() {
        let slug = entry.slug();
        // Load all specs to build dependency graph
        match super::scan(specs_dir) {
            Ok(scan_result) => {
                let mut graph: HashMap<String, Vec<String>> = HashMap::new();
                for e in &scan_result.entries {
                    let deps = match e {
                        super::SpecEntry::Legacy { spec, .. } => spec.depends.clone(),
                        super::SpecEntry::Directory { gates, .. } => gates.depends.clone(),
                    };
                    graph.insert(e.slug().to_string(), deps);
                }
                // Always use the in-memory entry's depends (may differ from on-disk)
                graph.insert(slug.to_string(), depends.clone());

                let cycle_diagnostics = detect_cycles(&graph);
                // Only include diagnostics involving this spec (by set membership, not substring)
                for cd in cycle_diagnostics {
                    if cd.specs.contains(slug) {
                        result.diagnostics.push(cd.diagnostic);
                    }
                }
                // Rebuild summary
                result.summary = build_summary(&result.diagnostics);
                result.valid = result.summary.errors == 0;
            }
            Err(e) => {
                result.diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    location: "depends".to_string(),
                    message: format!(
                        "cycle detection skipped: could not scan specs directory: {e}"
                    ),
                });
                result.summary = build_summary(&result.diagnostics);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{Criterion, CriterionKind, Spec};

    #[test]
    fn test_spec_errors_to_diagnostics() {
        let errors = vec![
            SpecError {
                field: "name".to_string(),
                message: "must not be empty".to_string(),
            },
            SpecError {
                field: "criteria".to_string(),
                message: "must have at least one".to_string(),
            },
        ];

        let diagnostics = spec_errors_to_diagnostics(&errors);
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(diagnostics[0].location, "name");
        assert_eq!(diagnostics[0].message, "must not be empty");
        assert_eq!(diagnostics[1].severity, Severity::Error);
        assert_eq!(diagnostics[1].location, "criteria");
    }

    #[test]
    fn test_validate_agent_prompts_missing() {
        let criteria = vec![Criterion {
            name: "review".to_string(),
            description: "Agent review".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::AgentReport),
            prompt: None,
            requirements: vec![],
        }];

        let diagnostics = validate_agent_prompts(&criteria);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(diagnostics[0].location, "criteria[0].prompt");
        assert!(diagnostics[0].message.contains("no prompt"));
    }

    #[test]
    fn test_validate_agent_prompts_empty_whitespace() {
        let criteria = vec![Criterion {
            name: "review".to_string(),
            description: "Agent review".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::AgentReport),
            prompt: Some("   ".to_string()),
            requirements: vec![],
        }];

        let diagnostics = validate_agent_prompts(&criteria);
        assert_eq!(diagnostics.len(), 1, "whitespace-only prompt should warn");
    }

    #[test]
    fn test_validate_agent_prompts_present() {
        let criteria = vec![Criterion {
            name: "review".to_string(),
            description: "Agent review".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: Some(CriterionKind::AgentReport),
            prompt: Some("Check for security issues".to_string()),
            requirements: vec![],
        }];

        let diagnostics = validate_agent_prompts(&criteria);
        assert!(
            diagnostics.is_empty(),
            "valid prompt should produce no diagnostic"
        );
    }

    #[test]
    fn test_validate_agent_prompts_non_agent_ignored() {
        let criteria = vec![Criterion {
            name: "build".to_string(),
            description: "Build check".to_string(),
            cmd: Some("cargo build".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        }];

        let diagnostics = validate_agent_prompts(&criteria);
        assert!(diagnostics.is_empty(), "non-AgentReport should be ignored");
    }

    #[test]
    fn test_validate_commands_missing_binary() {
        let criteria = vec![Criterion {
            name: "check".to_string(),
            description: "Run check".to_string(),
            cmd: Some("nonexistent_binary_xyz_12345 --flag".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        }];

        let diagnostics = validate_commands(&criteria);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("not found on PATH"));
        assert!(
            diagnostics[0]
                .message
                .contains("nonexistent_binary_xyz_12345")
        );
    }

    #[test]
    fn test_validate_commands_known_binary() {
        let criteria = vec![Criterion {
            name: "check".to_string(),
            description: "Run check".to_string(),
            cmd: Some("echo hello".to_string()),
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        }];

        let diagnostics = validate_commands(&criteria);
        assert!(diagnostics.is_empty(), "echo should be found on PATH");
    }

    #[test]
    fn test_validate_commands_no_cmd() {
        let criteria = vec![Criterion {
            name: "descriptive".to_string(),
            description: "No command".to_string(),
            cmd: None,
            path: None,
            timeout: None,
            enforcement: None,
            kind: None,
            prompt: None,
            requirements: vec![],
        }];

        let diagnostics = validate_commands(&criteria);
        assert!(
            diagnostics.is_empty(),
            "criteria without cmd should be skipped"
        );
    }

    #[test]
    fn test_detect_cycles_simple() {
        let mut specs = HashMap::new();
        specs.insert("a".to_string(), vec!["b".to_string()]);
        specs.insert("b".to_string(), vec!["a".to_string()]);

        let results = detect_cycles(&specs);
        let errors: Vec<_> = results
            .iter()
            .filter(|cd| cd.diagnostic.severity == Severity::Error)
            .collect();
        assert!(!errors.is_empty(), "should detect cycle between a and b");
        assert!(errors[0].diagnostic.message.contains("circular dependency"));
        assert!(errors[0].specs.contains("a"));
        assert!(errors[0].specs.contains("b"));
    }

    #[test]
    fn test_detect_cycles_none() {
        let mut specs = HashMap::new();
        specs.insert("a".to_string(), vec!["b".to_string()]);
        specs.insert("b".to_string(), vec!["c".to_string()]);
        specs.insert("c".to_string(), vec![]);

        let results = detect_cycles(&specs);
        let errors: Vec<_> = results
            .iter()
            .filter(|cd| cd.diagnostic.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "no cycle should be detected");
    }

    #[test]
    fn test_detect_cycles_unknown_dep() {
        let mut specs = HashMap::new();
        specs.insert("a".to_string(), vec!["unknown".to_string()]);

        let results = detect_cycles(&specs);
        let warnings: Vec<_> = results
            .iter()
            .filter(|cd| cd.diagnostic.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].diagnostic.message.contains("not found"));
        assert!(warnings[0].diagnostic.message.contains("unknown"));
        assert_eq!(warnings[0].diagnostic.location, "depends[0]");
    }

    #[test]
    fn test_detect_cycles_three_node_cycle() {
        let mut specs = HashMap::new();
        specs.insert("a".to_string(), vec!["b".to_string()]);
        specs.insert("b".to_string(), vec!["c".to_string()]);
        specs.insert("c".to_string(), vec!["a".to_string()]);

        let results = detect_cycles(&specs);
        let errors: Vec<_> = results
            .iter()
            .filter(|cd| cd.diagnostic.severity == Severity::Error)
            .collect();
        assert!(!errors.is_empty(), "should detect 3-node cycle");
        // The cycle path should contain all three nodes
        let msg = &errors[0].diagnostic.message;
        assert!(msg.contains("a"), "cycle should mention a: {msg}");
        assert!(msg.contains("b"), "cycle should mention b: {msg}");
        assert!(msg.contains("c"), "cycle should mention c: {msg}");
        // All three should be in the specs set
        assert!(errors[0].specs.contains("a"));
        assert!(errors[0].specs.contains("b"));
        assert!(errors[0].specs.contains("c"));
    }

    #[test]
    fn test_detect_cycles_self_loop() {
        let mut specs = HashMap::new();
        specs.insert("a".to_string(), vec!["a".to_string()]);

        let results = detect_cycles(&specs);
        let errors: Vec<_> = results
            .iter()
            .filter(|cd| cd.diagnostic.severity == Severity::Error)
            .collect();
        assert!(!errors.is_empty(), "should detect self-loop");
        assert!(errors[0].specs.contains("a"));
    }

    #[test]
    fn test_validate_spec_valid_legacy() {
        let entry = super::super::SpecEntry::Legacy {
            slug: "test-spec".to_string(),
            spec: Spec {
                name: "test".to_string(),
                description: String::new(),
                gate: None,
                depends: vec![],
                criteria: vec![Criterion {
                    name: "c1".to_string(),
                    description: "d1".to_string(),
                    cmd: Some("true".to_string()),
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: None,
                    prompt: None,
                    requirements: vec![],
                }],
            },
        };

        let result = validate_spec(&entry, false);
        assert!(result.valid);
        assert_eq!(result.spec, "test-spec");
        assert_eq!(result.summary.errors, 0);
        assert_eq!(result.summary.warnings, 0);
    }

    #[test]
    fn test_validate_spec_agent_prompt_warning() {
        // validate_spec detects AgentReport criteria without prompts
        let entry = super::super::SpecEntry::Legacy {
            slug: "agent-spec".to_string(),
            spec: Spec {
                name: "agent test".to_string(),
                description: String::new(),
                gate: None,
                depends: vec![],
                criteria: vec![Criterion {
                    name: "review".to_string(),
                    description: "Agent review".to_string(),
                    cmd: None,
                    path: None,
                    timeout: None,
                    enforcement: None,
                    kind: Some(CriterionKind::AgentReport),
                    prompt: None,
                    requirements: vec![],
                }],
            },
        };

        let result = validate_spec(&entry, false);
        // Spec is still valid (warnings don't block)
        assert!(result.valid);
        assert_eq!(result.summary.warnings, 1);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Warning && d.location == "criteria[0].prompt")
        );
    }
}
