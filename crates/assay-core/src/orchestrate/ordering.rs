//! Merge ordering strategies for session branches.
//!
//! Determines the sequence in which completed session branches are merged
//! into the base branch. Order matters: merging A then B can succeed while
//! B then A conflicts (D019).
//!
//! Two strategies are provided:
//! - [`MergeStrategy::CompletionTime`]: sort by completion timestamp with
//!   topological index as tiebreak.
//! - [`MergeStrategy::FileOverlap`]: greedy algorithm that iteratively picks
//!   the session whose changed files have the least overlap with the
//!   already-merged file set.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use assay_types::{MergePlan, MergePlanEntry, MergeStrategy};

/// A completed session ready for merge ordering.
///
/// This is an operational type — not persisted, but used as input to
/// [`order_sessions`] to produce a [`MergePlan`].
#[derive(Debug, Clone)]
pub struct CompletedSession {
    /// Effective session name.
    pub session_name: String,
    /// Git branch name for this session's work.
    pub branch_name: String,
    /// Files changed by this session (relative to repo root).
    pub changed_files: Vec<String>,
    /// When the session completed execution.
    pub completed_at: DateTime<Utc>,
    /// Topological order index from the dependency graph (lower = closer to root).
    pub topo_order: usize,
}

/// Order sessions according to the given strategy and produce a merge plan.
///
/// Returns the sessions in merge order alongside a [`MergePlan`] that records
/// the strategy and per-session placement rationale for observability.
///
/// # Determinism
///
/// Both strategies produce deterministic output for the same input:
/// - `CompletionTime`: timestamp first, then `topo_order`, then `session_name`.
/// - `FileOverlap`: overlap count first, then `topo_order`, then `session_name`.
pub fn order_sessions(
    mut sessions: Vec<CompletedSession>,
    strategy: MergeStrategy,
) -> (Vec<CompletedSession>, MergePlan) {
    let entries = match strategy {
        MergeStrategy::CompletionTime => order_by_completion_time(&mut sessions),
        MergeStrategy::FileOverlap => order_by_file_overlap(&mut sessions),
    };

    let plan = MergePlan { strategy, entries };
    (sessions, plan)
}

/// Sort by completion timestamp, with topo_order and session_name as tiebreakers.
fn order_by_completion_time(sessions: &mut [CompletedSession]) -> Vec<MergePlanEntry> {
    sessions.sort_by(|a, b| {
        a.completed_at
            .cmp(&b.completed_at)
            .then_with(|| a.topo_order.cmp(&b.topo_order))
            .then_with(|| a.session_name.cmp(&b.session_name))
    });

    sessions
        .iter()
        .enumerate()
        .map(|(i, s)| MergePlanEntry {
            session_name: s.session_name.clone(),
            position: i,
            reason: format!(
                "completed at {} (topo_order={})",
                s.completed_at.format("%H:%M:%S%.3fZ"),
                s.topo_order,
            ),
        })
        .collect()
}

/// Greedy least-overlap-first algorithm.
///
/// Iteratively picks the session whose changed files have the least overlap
/// with the already-merged file set. Ties broken by topo_order then name.
fn order_by_file_overlap(sessions: &mut Vec<CompletedSession>) -> Vec<MergePlanEntry> {
    if sessions.is_empty() {
        return Vec::new();
    }

    let mut merged_files: HashSet<String> = HashSet::new();
    let mut remaining: Vec<CompletedSession> = std::mem::take(sessions);
    let mut ordered: Vec<CompletedSession> = Vec::with_capacity(remaining.len());
    let mut entries: Vec<MergePlanEntry> = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        // Find the session with the least overlap
        let best_idx = remaining
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let overlap_a = a
                    .changed_files
                    .iter()
                    .filter(|f| merged_files.contains(*f))
                    .count();
                let overlap_b = b
                    .changed_files
                    .iter()
                    .filter(|f| merged_files.contains(*f))
                    .count();
                overlap_a
                    .cmp(&overlap_b)
                    .then_with(|| a.topo_order.cmp(&b.topo_order))
                    .then_with(|| a.session_name.cmp(&b.session_name))
            })
            .map(|(idx, _)| idx)
            .unwrap(); // remaining is non-empty

        let session = remaining.swap_remove(best_idx);
        let overlap_count = session
            .changed_files
            .iter()
            .filter(|f| merged_files.contains(*f))
            .count();

        entries.push(MergePlanEntry {
            session_name: session.session_name.clone(),
            position: ordered.len(),
            reason: format!("{} overlapping files", overlap_count),
        });

        // Add this session's files to the merged set
        for f in &session.changed_files {
            merged_files.insert(f.clone());
        }

        ordered.push(session);
    }

    *sessions = ordered;
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_session(
        name: &str,
        files: &[&str],
        completed_at: DateTime<Utc>,
        topo_order: usize,
    ) -> CompletedSession {
        CompletedSession {
            session_name: name.to_string(),
            branch_name: format!("session/{name}"),
            changed_files: files.iter().map(|f| f.to_string()).collect(),
            completed_at,
            topo_order,
        }
    }

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1700000000 + secs, 0).unwrap()
    }

    // ── CompletionTime tests ─────────────────────────────────────────

    #[test]
    fn completion_time_sorts_by_timestamp() {
        let sessions = vec![
            make_session("late", &["a.rs"], ts(30), 0),
            make_session("early", &["b.rs"], ts(10), 0),
            make_session("mid", &["c.rs"], ts(20), 0),
        ];

        let (ordered, plan) = order_sessions(sessions, MergeStrategy::CompletionTime);
        assert_eq!(plan.strategy, MergeStrategy::CompletionTime);
        assert_eq!(ordered[0].session_name, "early");
        assert_eq!(ordered[1].session_name, "mid");
        assert_eq!(ordered[2].session_name, "late");
    }

    #[test]
    fn completion_time_topo_tiebreak() {
        let t = ts(0);
        let sessions = vec![
            make_session("beta", &["a.rs"], t, 2),
            make_session("alpha", &["b.rs"], t, 1),
            make_session("gamma", &["c.rs"], t, 0),
        ];

        let (ordered, plan) = order_sessions(sessions, MergeStrategy::CompletionTime);
        assert_eq!(ordered[0].session_name, "gamma"); // topo_order 0
        assert_eq!(ordered[1].session_name, "alpha"); // topo_order 1
        assert_eq!(ordered[2].session_name, "beta"); // topo_order 2
        assert_eq!(plan.entries.len(), 3);
        assert_eq!(plan.entries[0].position, 0);
        assert_eq!(plan.entries[2].position, 2);
    }

    #[test]
    fn completion_time_name_tiebreak() {
        let t = ts(0);
        let sessions = vec![
            make_session("charlie", &["a.rs"], t, 0),
            make_session("alpha", &["b.rs"], t, 0),
            make_session("bravo", &["c.rs"], t, 0),
        ];

        let (ordered, _) = order_sessions(sessions, MergeStrategy::CompletionTime);
        assert_eq!(ordered[0].session_name, "alpha");
        assert_eq!(ordered[1].session_name, "bravo");
        assert_eq!(ordered[2].session_name, "charlie");
    }

    // ── FileOverlap tests ────────────────────────────────────────────

    #[test]
    fn file_overlap_prefers_less_overlap() {
        let t = ts(0);
        // Session "disjoint" touches only d.rs (no overlap ever).
        // Session "overlap" touches a.rs and b.rs.
        // Session "base" also touches a.rs and b.rs.
        // If "base" goes first, "overlap" has 2 overlapping files, "disjoint" has 0.
        // So "disjoint" should come before "overlap" after "base".
        let sessions = vec![
            make_session("overlap", &["a.rs", "b.rs"], t, 0),
            make_session("disjoint", &["d.rs"], t, 0),
            make_session("base", &["a.rs", "b.rs", "c.rs"], t, 0),
        ];

        let (ordered, plan) = order_sessions(sessions, MergeStrategy::FileOverlap);
        assert_eq!(plan.strategy, MergeStrategy::FileOverlap);

        // All three have 0 overlap initially. Topo=0 for all, so alphabetical:
        // "base" (0 overlap), "disjoint" (0 overlap), "overlap" (0 overlap)
        // After "base" merged: merged_files = {a.rs, b.rs, c.rs}
        // "disjoint" (d.rs) = 0 overlap, "overlap" (a.rs, b.rs) = 2 overlap
        // => "disjoint" next, then "overlap"
        assert_eq!(ordered[0].session_name, "base");
        assert_eq!(ordered[1].session_name, "disjoint");
        assert_eq!(ordered[2].session_name, "overlap");

        // Verify plan entries have correct reasons
        assert!(plan.entries[0].reason.contains("0 overlapping"));
        assert!(plan.entries[1].reason.contains("0 overlapping"));
        assert!(plan.entries[2].reason.contains("2 overlapping"));
    }

    #[test]
    fn file_overlap_tiebreak_by_topo_then_name() {
        let t = ts(0);
        // All sessions touch different files (no overlap), so topo breaks ties.
        let sessions = vec![
            make_session("beta", &["b.rs"], t, 2),
            make_session("alpha", &["a.rs"], t, 1),
            make_session("gamma", &["g.rs"], t, 0),
        ];

        let (ordered, _) = order_sessions(sessions, MergeStrategy::FileOverlap);
        assert_eq!(ordered[0].session_name, "gamma"); // topo 0
        assert_eq!(ordered[1].session_name, "alpha"); // topo 1
        assert_eq!(ordered[2].session_name, "beta"); // topo 2
    }

    // ── Edge cases ───────────────────────────────────────────────────

    #[test]
    fn single_session_returns_unchanged() {
        let session = make_session("only", &["x.rs"], ts(0), 0);
        let name = session.session_name.clone();

        let (ordered, plan) = order_sessions(vec![session], MergeStrategy::CompletionTime);
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].session_name, name);
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].position, 0);
    }

    #[test]
    fn empty_sessions_returns_empty() {
        let (ordered, plan) = order_sessions(vec![], MergeStrategy::CompletionTime);
        assert!(ordered.is_empty());
        assert!(plan.entries.is_empty());

        let (ordered, plan) = order_sessions(vec![], MergeStrategy::FileOverlap);
        assert!(ordered.is_empty());
        assert!(plan.entries.is_empty());
    }

    #[test]
    fn deterministic_across_runs() {
        let t = ts(0);
        let make_input = || {
            vec![
                make_session("c", &["shared.rs", "c.rs"], t, 1),
                make_session("a", &["shared.rs", "a.rs"], t, 0),
                make_session("b", &["b.rs"], t, 1),
            ]
        };

        // Run multiple times and verify same result
        let (first, plan1) = order_sessions(make_input(), MergeStrategy::FileOverlap);
        let (second, plan2) = order_sessions(make_input(), MergeStrategy::FileOverlap);

        let names1: Vec<_> = first.iter().map(|s| &s.session_name).collect();
        let names2: Vec<_> = second.iter().map(|s| &s.session_name).collect();
        assert_eq!(names1, names2);
        assert_eq!(plan1.entries.len(), plan2.entries.len());
        for (e1, e2) in plan1.entries.iter().zip(plan2.entries.iter()) {
            assert_eq!(e1.session_name, e2.session_name);
            assert_eq!(e1.position, e2.position);
        }
    }
}
