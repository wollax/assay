//! Smelt core library — infrastructure-layer job execution engine.

pub mod assay;
pub mod collector;
pub mod config;
pub mod docker;
pub mod error;
pub mod git;
pub mod manifest;
pub mod monitor;
pub mod provider;

pub use assay::AssayInvoker;
pub use collector::{BranchCollectResult, ResultCollector};
pub use config::SmeltConfig;
pub use docker::DockerProvider;
pub use error::{Result, SmeltError};
pub use git::{GitCli, GitOps, preflight};
pub use manifest::JobManifest;
pub use monitor::{JobMonitor, JobPhase, RunState, compute_job_timeout};
pub use provider::{CollectResult, ContainerId, ExecHandle, RuntimeProvider};
