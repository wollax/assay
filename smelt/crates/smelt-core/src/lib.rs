//! Smelt core library — infrastructure-layer job execution engine.
//!
//! `smelt-core` provides the domain types, job manifest parser, Docker-based
//! runtime provider, and optional GitHub Forge integration used by the Smelt CI
//! orchestration system. Embed this crate to schedule and monitor containerised
//! jobs, collect results, and (optionally) report status back to GitHub pull
//! requests — all without taking a dependency on the Smelt CLI binary.
//!
//! # Feature flags
//!
//! * **`forge`** (optional) — enables `GitHubForge` and pulls in `octocrab` plus
//!   `serde_json`. Gate this feature behind `#[cfg(feature = "forge")]` in
//!   consuming crates when GitHub integration is not required.
//!
//! # Example
//!
//! ```rust,no_run
//! use smelt_core::JobManifest;
//! #[cfg(feature = "forge")]
//! use smelt_core::forge::GitHubForge;
//!
//! # async fn example() -> smelt_core::error::Result<()> {
//! // Parse a job manifest from a TOML string.
//! let manifest = JobManifest::from_str("...", std::path::Path::new("smelt.toml"))?;
//!
//! // Construct a GitHub Forge client (requires the `forge` feature).
//! #[cfg(feature = "forge")]
//! let _forge = GitHubForge::new("ghp_token".to_string())?;
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]

pub mod assay;
pub mod collector;
pub mod compose;
pub mod config;
pub mod docker;
pub mod error;
pub mod forge;
pub mod git;
pub mod k8s;
pub mod manifest;
pub mod monitor;
pub mod provider;
pub mod tracker;

pub use assay::AssayInvoker;
pub use collector::{BranchCollectResult, ResultCollector};
/// Docker Compose runtime provider for orchestrating multi-service jobs.
pub use compose::ComposeProvider;
pub use config::SmeltConfig;
pub use docker::DockerProvider;
pub use error::{Result, SmeltError};
#[cfg(feature = "forge")]
pub use forge::GitHubForge;
pub use forge::{CiStatus, ForgeClient, ForgeConfig, PrHandle, PrState, PrStatus};
pub use git::{GitCli, GitOps, preflight};
pub use k8s::KubernetesProvider;
pub use manifest::JobManifest;
pub use monitor::{JobMonitor, JobPhase, RunState, compute_job_timeout};
pub use provider::{CollectResult, ContainerId, ExecHandle, ProvisionResult, RuntimeProvider};

// Signal types re-exported from assay-types (D012 — canonical types, no local mirrors).
pub use assay_types::signal::{GateSummary, PeerUpdate, SignalRequest};
