//! Dependency graph construction and validation for session manifests.
//!
//! Given a [`RunManifest`](assay_types::RunManifest), builds a directed acyclic
//! graph (DAG) of session dependencies and validates it for cycles, missing
//! references, duplicate effective names, and self-dependencies.
//!
//! Uses `Vec<Vec<usize>>` adjacency lists instead of petgraph for zero
//! additional dependencies.

use std::collections::{HashMap, HashSet, VecDeque};

use assay_types::RunManifest;

use crate::error::AssayError;
use crate::manifest::ManifestError;

/// A validated dependency graph of manifest sessions.
///
/// Index stability: `names[i]` always corresponds to `manifest.sessions[i]`.
/// Edges are stored as adjacency lists indexed by session position.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Effective session names, indexed by session position in the manifest.
    names: Vec<String>,
    /// Forward edges: `forward_edges[i]` lists indices of sessions that depend on session `i`.
    forward_edges: Vec<Vec<usize>>,
    /// Reverse edges: `reverse_edges[i]` lists indices of sessions that session `i` depends on.
    reverse_edges: Vec<Vec<usize>>,
    /// Number of sessions in the graph.
    session_count: usize,
}

impl DependencyGraph {
    /// Number of sessions in the graph.
    pub fn session_count(&self) -> usize {
        self.session_count
    }

    /// Effective name of the session at `idx`.
    pub fn name_of(&self, idx: usize) -> &str {
        &self.names[idx]
    }

    /// Find the index of a session by its effective name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// Forward edges: sessions that depend on `idx` (dependents).
    pub fn dependents_of(&self, idx: usize) -> &[usize] {
        &self.forward_edges[idx]
    }

    /// Reverse edges: sessions that `idx` depends on (dependencies).
    pub fn dependencies_of(&self, idx: usize) -> &[usize] {
        &self.reverse_edges[idx]
    }

    /// Returns indices of sessions that are ready to execute.
    ///
    /// A session is ready when:
    /// - It is not in `completed`, `in_flight`, or `skipped`
    /// - Every dependency (reverse edge) is in `completed` or `skipped`
    ///
    /// Skipped dependencies count as satisfied (Smelt semantics): dependents
    /// of a skipped session become ready as long as their other deps are met.
    ///
    /// Returns a sorted `Vec<usize>` for deterministic dispatch ordering.
    pub fn ready_set(
        &self,
        completed: &HashSet<usize>,
        in_flight: &HashSet<usize>,
        skipped: &HashSet<usize>,
    ) -> Vec<usize> {
        let mut ready: Vec<usize> = (0..self.session_count)
            .filter(|&i| {
                !completed.contains(&i)
                    && !in_flight.contains(&i)
                    && !skipped.contains(&i)
                    && self.reverse_edges[i]
                        .iter()
                        .all(|dep| completed.contains(dep) || skipped.contains(dep))
            })
            .collect();
        ready.sort_unstable();
        ready
    }

    /// BFS-mark all transitive dependents of a failed session as skipped.
    ///
    /// Walks `forward_edges` from `failed_idx`, inserting every reachable
    /// node into `skipped`. Does **not** insert `failed_idx` itself — the
    /// caller is responsible for recording the failure.
    pub fn mark_skipped_dependents(&self, failed_idx: usize, skipped: &mut HashSet<usize>) {
        let mut queue: VecDeque<usize> = VecDeque::new();
        for &dep in &self.forward_edges[failed_idx] {
            if skipped.insert(dep) {
                queue.push_back(dep);
            }
        }
        while let Some(node) = queue.pop_front() {
            for &dep in &self.forward_edges[node] {
                if skipped.insert(dep) {
                    queue.push_back(dep);
                }
            }
        }
    }

    /// Returns parallelism groups via layer-by-layer topological sort.
    ///
    /// Layer 0 contains all sessions with no dependencies (in-degree 0).
    /// Each subsequent layer contains sessions whose dependencies are all in
    /// earlier layers. Sessions within a layer can execute concurrently.
    ///
    /// Indices within each layer are sorted for determinism.
    pub fn topological_groups(&self) -> Vec<Vec<usize>> {
        // Compute in-degree from forward_edges.
        let mut in_degree: Vec<usize> = vec![0; self.session_count];
        for edges in &self.forward_edges {
            for &dependent in edges {
                in_degree[dependent] += 1;
            }
        }

        let mut current_layer: Vec<usize> = (0..self.session_count)
            .filter(|&i| in_degree[i] == 0)
            .collect();
        current_layer.sort_unstable();

        let mut groups: Vec<Vec<usize>> = Vec::new();
        while !current_layer.is_empty() {
            let mut next_layer: Vec<usize> = Vec::new();
            for &node in &current_layer {
                for &dependent in &self.forward_edges[node] {
                    in_degree[dependent] -= 1;
                    if in_degree[dependent] == 0 {
                        next_layer.push(dependent);
                    }
                }
            }
            next_layer.sort_unstable();
            groups.push(current_layer);
            current_layer = next_layer;
        }

        groups
    }

    /// Build and validate a `DependencyGraph` from a run manifest.
    ///
    /// # Validation
    ///
    /// When any session has non-empty `depends_on`:
    /// - Effective names must be unique
    /// - Each dependency reference must resolve to an existing session
    /// - Self-dependencies are rejected
    /// - The graph must be acyclic (checked via Kahn's algorithm)
    ///
    /// When no session has `depends_on`, the graph is trivially valid with no edges.
    pub fn from_manifest(manifest: &RunManifest) -> Result<Self, AssayError> {
        let count = manifest.sessions.len();

        // Compute effective names: name if set, otherwise spec.
        let names: Vec<String> = manifest
            .sessions
            .iter()
            .map(|s| s.name.clone().unwrap_or_else(|| s.spec.clone()))
            .collect();

        let has_any_deps = manifest.sessions.iter().any(|s| !s.depends_on.is_empty());

        // If no session declares dependencies, return a trivial graph.
        if !has_any_deps {
            return Ok(Self {
                names,
                forward_edges: vec![vec![]; count],
                reverse_edges: vec![vec![]; count],
                session_count: count,
            });
        }

        // When dependencies are present, validate unique effective names.
        let mut errors = Vec::new();
        let mut name_to_idx: HashMap<&str, usize> = HashMap::with_capacity(count);
        for (i, name) in names.iter().enumerate() {
            if let Some(&prev_idx) = name_to_idx.get(name.as_str()) {
                errors.push(ManifestError {
                    field: format!("sessions[{i}]"),
                    message: format!(
                        "duplicate effective name '{name}' (conflicts with sessions[{prev_idx}])"
                    ),
                });
            } else {
                name_to_idx.insert(name, i);
            }
        }

        // If there are duplicate names, bail early — we can't resolve references.
        if !errors.is_empty() {
            return Err(AssayError::DagValidation { errors });
        }

        // Resolve dependency references and build edges.
        let mut forward_edges: Vec<Vec<usize>> = vec![vec![]; count];
        let mut reverse_edges: Vec<Vec<usize>> = vec![vec![]; count];

        for (i, session) in manifest.sessions.iter().enumerate() {
            for dep_name in &session.depends_on {
                // Self-dependency check.
                if dep_name == &names[i] {
                    errors.push(ManifestError {
                        field: format!("sessions[{i}].depends_on"),
                        message: format!("session '{name}' depends on itself", name = names[i]),
                    });
                    continue;
                }

                // Resolve reference.
                match name_to_idx.get(dep_name.as_str()) {
                    Some(&dep_idx) => {
                        // Edge from dependency to dependent (forward).
                        forward_edges[dep_idx].push(i);
                        // Edge from dependent to dependency (reverse).
                        reverse_edges[i].push(dep_idx);
                    }
                    None => {
                        errors.push(ManifestError {
                            field: format!("sessions[{i}].depends_on"),
                            message: format!(
                                "session '{name}' depends on unknown session '{dep_name}'",
                                name = names[i]
                            ),
                        });
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(AssayError::DagValidation { errors });
        }

        // Kahn's algorithm for cycle detection.
        // Compute in-degree from forward_edges (number of dependencies per node).
        let mut in_degree: Vec<usize> = vec![0; count];
        for edges in &forward_edges {
            for &dependent in edges {
                in_degree[dependent] += 1;
            }
        }

        let mut queue: VecDeque<usize> = VecDeque::new();
        for (i, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                queue.push_back(i);
            }
        }

        let mut processed = 0usize;
        while let Some(node) = queue.pop_front() {
            processed += 1;
            for &dependent in &forward_edges[node] {
                in_degree[dependent] -= 1;
                if in_degree[dependent] == 0 {
                    queue.push_back(dependent);
                }
            }
        }

        if processed < count {
            // Collect names of sessions involved in cycles (those not processed).
            let sessions: Vec<String> = (0..count)
                .filter(|&i| in_degree[i] > 0)
                .map(|i| names[i].clone())
                .collect();
            return Err(AssayError::DagCycle { sessions });
        }

        Ok(Self {
            names,
            forward_edges,
            reverse_edges,
            session_count: count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::ManifestSession;

    /// Helper to build a RunManifest from a list of (spec, name, depends_on) tuples.
    fn make_manifest(sessions: Vec<(&str, Option<&str>, Vec<&str>)>) -> RunManifest {
        RunManifest {
            sessions: sessions
                .into_iter()
                .map(|(spec, name, deps)| ManifestSession {
                    spec: spec.to_string(),
                    name: name.map(|n| n.to_string()),
                    settings: None,
                    hooks: vec![],
                    prompt_layers: vec![],
                    file_scope: vec![],
                    shared_files: vec![],
                    depends_on: deps.into_iter().map(|d| d.to_string()).collect(),
                })
                .collect(),
            ..Default::default()
        }
    }

    // ── Valid graphs ───────────────────────────────────────────────

    #[test]
    fn valid_linear_chain() {
        // a → b → c
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.session_count(), 3);
        assert_eq!(graph.name_of(0), "a");
        assert_eq!(graph.name_of(1), "b");
        assert_eq!(graph.name_of(2), "c");
        // b depends on a
        assert_eq!(graph.dependencies_of(1), &[0]);
        // a's dependents include b
        assert!(graph.dependents_of(0).contains(&1));
        // b's dependents include c
        assert!(graph.dependents_of(1).contains(&2));
    }

    #[test]
    fn valid_diamond() {
        // a → {b, c} → d
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["a"]),
            ("d", None, vec!["b", "c"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.session_count(), 4);
        assert_eq!(graph.dependencies_of(3).len(), 2); // d depends on b and c
        assert_eq!(graph.dependents_of(0).len(), 2); // a has dependents b and c
    }

    #[test]
    fn valid_parallel_no_deps() {
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec![]),
            ("c", None, vec![]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.session_count(), 3);
        // No edges at all
        for i in 0..3 {
            assert!(graph.dependents_of(i).is_empty());
            assert!(graph.dependencies_of(i).is_empty());
        }
    }

    #[test]
    fn valid_single_session() {
        let manifest = make_manifest(vec![("only", None, vec![])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.session_count(), 1);
        assert_eq!(graph.name_of(0), "only");
    }

    #[test]
    fn empty_depends_on_treated_as_no_deps() {
        // All sessions have empty depends_on — should produce no edges.
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec![])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        for i in 0..2 {
            assert!(graph.dependents_of(i).is_empty());
            assert!(graph.dependencies_of(i).is_empty());
        }
    }

    #[test]
    fn mixed_some_with_deps_some_without() {
        // a has no deps, b depends on a, c has no deps
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec![]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.session_count(), 3);
        assert!(graph.dependencies_of(0).is_empty());
        assert_eq!(graph.dependencies_of(1), &[0]);
        assert!(graph.dependencies_of(2).is_empty());
    }

    #[test]
    fn effective_name_uses_name_field_over_spec() {
        let manifest = make_manifest(vec![
            ("spec-a", Some("alpha"), vec![]),
            ("spec-b", None, vec!["alpha"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.name_of(0), "alpha");
        assert_eq!(graph.name_of(1), "spec-b");
        assert_eq!(graph.index_of("alpha"), Some(0));
        assert_eq!(graph.index_of("spec-a"), None); // spec is not the effective name
    }

    // ── Invalid graphs ────────────────────────────────────────────

    #[test]
    fn cycle_detection_two_node() {
        // a → b → a
        let manifest = make_manifest(vec![("a", None, vec!["b"]), ("b", None, vec!["a"])]);
        let err = DependencyGraph::from_manifest(&manifest).unwrap_err();
        match &err {
            AssayError::DagCycle { sessions } => {
                assert!(sessions.contains(&"a".to_string()));
                assert!(sessions.contains(&"b".to_string()));
            }
            other => panic!("expected DagCycle, got: {other:?}"),
        }
    }

    #[test]
    fn cycle_detection_three_node() {
        // a → b → c → a
        let manifest = make_manifest(vec![
            ("a", None, vec!["c"]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let err = DependencyGraph::from_manifest(&manifest).unwrap_err();
        match &err {
            AssayError::DagCycle { sessions } => {
                assert_eq!(sessions.len(), 3);
                assert!(sessions.contains(&"a".to_string()));
                assert!(sessions.contains(&"b".to_string()));
                assert!(sessions.contains(&"c".to_string()));
            }
            other => panic!("expected DagCycle, got: {other:?}"),
        }
    }

    #[test]
    fn missing_dependency_reference() {
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec!["nonexistent"])]);
        let err = DependencyGraph::from_manifest(&manifest).unwrap_err();
        match &err {
            AssayError::DagValidation { errors } => {
                assert_eq!(errors.len(), 1);
                assert!(errors[0].message.contains("nonexistent"));
                assert!(errors[0].message.contains("unknown session"));
            }
            other => panic!("expected DagValidation, got: {other:?}"),
        }
    }

    #[test]
    fn self_dependency_rejected() {
        let manifest = make_manifest(vec![("a", None, vec!["a"])]);
        let err = DependencyGraph::from_manifest(&manifest).unwrap_err();
        match &err {
            AssayError::DagValidation { errors } => {
                assert_eq!(errors.len(), 1);
                assert!(errors[0].message.contains("depends on itself"));
            }
            other => panic!("expected DagValidation, got: {other:?}"),
        }
    }

    #[test]
    fn duplicate_effective_names_with_deps() {
        // Two sessions with same effective name "a", and one references it.
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
        ]);
        let err = DependencyGraph::from_manifest(&manifest).unwrap_err();
        match &err {
            AssayError::DagValidation { errors } => {
                assert!(
                    errors
                        .iter()
                        .any(|e| e.message.contains("duplicate effective name"))
                );
            }
            other => panic!("expected DagValidation, got: {other:?}"),
        }
    }

    #[test]
    fn index_of_returns_none_for_unknown() {
        let manifest = make_manifest(vec![("a", None, vec![])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        assert_eq!(graph.index_of("nonexistent"), None);
    }

    // ── ready_set tests ─────────────────────────────────────────

    #[test]
    fn test_ready_set_returns_roots_when_nothing_completed() {
        // a → b → c: only a is ready initially
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let ready = graph.ready_set(&HashSet::new(), &HashSet::new(), &HashSet::new());
        assert_eq!(ready, vec![0]); // only 'a'
    }

    #[test]
    fn test_ready_set_completion_unblocks_dependents() {
        // a → b → c: complete a → b becomes ready
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let completed: HashSet<usize> = [0].into();
        let ready = graph.ready_set(&completed, &HashSet::new(), &HashSet::new());
        assert_eq!(ready, vec![1]); // only 'b'
    }

    #[test]
    fn test_ready_set_skipped_dep_satisfies() {
        // a → b: skip a → b becomes ready (Smelt semantics)
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec!["a"])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let skipped: HashSet<usize> = [0].into();
        let ready = graph.ready_set(&HashSet::new(), &HashSet::new(), &skipped);
        assert_eq!(ready, vec![1]); // b is ready because a is skipped
    }

    #[test]
    fn test_ready_set_in_flight_excluded() {
        // a, b, c all parallel (no deps): put a in_flight → only b,c ready
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec![]),
            ("c", None, vec![]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let in_flight: HashSet<usize> = [0].into();
        let ready = graph.ready_set(&HashSet::new(), &in_flight, &HashSet::new());
        assert_eq!(ready, vec![1, 2]);
    }

    #[test]
    fn test_ready_set_empty_when_all_completed() {
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec!["a"])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let completed: HashSet<usize> = [0, 1].into();
        let ready = graph.ready_set(&completed, &HashSet::new(), &HashSet::new());
        assert!(ready.is_empty());
    }

    #[test]
    fn test_ready_set_empty_when_all_in_flight() {
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec![])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let in_flight: HashSet<usize> = [0, 1].into();
        let ready = graph.ready_set(&HashSet::new(), &in_flight, &HashSet::new());
        assert!(ready.is_empty());
    }

    #[test]
    fn test_ready_set_diamond_needs_both_deps() {
        // a → {b,c} → d: complete a and b → d not ready yet (c still pending)
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["a"]),
            ("d", None, vec!["b", "c"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let completed: HashSet<usize> = [0, 1].into();
        let ready = graph.ready_set(&completed, &HashSet::new(), &HashSet::new());
        assert_eq!(ready, vec![2]); // only c, d still blocked on c
    }

    // ── mark_skipped_dependents tests ─────────────────────────────

    #[test]
    fn test_mark_skipped_transitive() {
        // a → b → c: fail a → skip b and c
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let mut skipped = HashSet::new();
        graph.mark_skipped_dependents(0, &mut skipped);
        assert!(skipped.contains(&1)); // b skipped
        assert!(skipped.contains(&2)); // c skipped
        assert!(!skipped.contains(&0)); // a NOT in skipped
    }

    #[test]
    fn test_mark_skipped_partial_independent_unaffected() {
        // a → b, c independent: fail a → skip b only, c unaffected
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec![]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let mut skipped = HashSet::new();
        graph.mark_skipped_dependents(0, &mut skipped);
        assert!(skipped.contains(&1));
        assert!(!skipped.contains(&2));
    }

    #[test]
    fn test_mark_skipped_does_not_add_failed_node() {
        let manifest = make_manifest(vec![("a", None, vec![]), ("b", None, vec!["a"])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let mut skipped = HashSet::new();
        graph.mark_skipped_dependents(0, &mut skipped);
        assert!(!skipped.contains(&0));
    }

    #[test]
    fn test_mark_skipped_diamond_propagation() {
        // a → {b,c} → d: fail a → skip b, c, d
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["a"]),
            ("d", None, vec!["b", "c"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let mut skipped = HashSet::new();
        graph.mark_skipped_dependents(0, &mut skipped);
        assert_eq!(skipped, HashSet::from([1, 2, 3]));
    }

    // ── topological_groups tests ──────────────────────────────────

    #[test]
    fn test_topological_groups_linear() {
        // a → b → c: 3 layers of 1
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["b"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let groups = graph.topological_groups();
        assert_eq!(groups, vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn test_topological_groups_diamond() {
        // a → {b,c} → d
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a"]),
            ("c", None, vec!["a"]),
            ("d", None, vec!["b", "c"]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let groups = graph.topological_groups();
        assert_eq!(groups, vec![vec![0], vec![1, 2], vec![3]]);
    }

    #[test]
    fn test_topological_groups_fully_parallel() {
        // a, b, c — no deps: 1 layer of 3
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec![]),
            ("c", None, vec![]),
        ]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let groups = graph.topological_groups();
        assert_eq!(groups, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn test_topological_groups_single_session() {
        let manifest = make_manifest(vec![("only", None, vec![])]);
        let graph = DependencyGraph::from_manifest(&manifest).unwrap();
        let groups = graph.topological_groups();
        assert_eq!(groups, vec![vec![0]]);
    }

    #[test]
    fn cycle_error_names_participants() {
        // Cycle: b → c → b, but a is not in the cycle
        let manifest = make_manifest(vec![
            ("a", None, vec![]),
            ("b", None, vec!["a", "c"]),
            ("c", None, vec!["b"]),
        ]);
        let err = DependencyGraph::from_manifest(&manifest).unwrap_err();
        match &err {
            AssayError::DagCycle { sessions } => {
                // a is NOT in the cycle — it has no unprocessed in-degree
                assert!(!sessions.contains(&"a".to_string()));
                assert!(sessions.contains(&"b".to_string()));
                assert!(sessions.contains(&"c".to_string()));
            }
            other => panic!("expected DagCycle, got: {other:?}"),
        }
    }
}
