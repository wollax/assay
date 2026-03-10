//! Orchestration engine — DAG-based parallel session execution and merge.

pub mod dag;
pub mod types;

pub use dag::{build_dag, mark_skipped_dependents, node_by_name, ready_set, SessionDag};
pub use types::{
    FailurePolicy, MergeProgress, OrchestrationOpts, OrchestrationReport, RunPhase, RunState,
    SessionRunState,
};
