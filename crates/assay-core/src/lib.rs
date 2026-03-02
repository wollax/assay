/// Error types for Assay operations.
pub mod error;

pub use error::{AssayError, Result};

/// Spec authoring and validation.
pub mod spec;

/// Quality gate evaluation.
pub mod gate;

/// Work review against specs.
pub mod review;

/// Workflow orchestration.
pub mod workflow;

/// Configuration loading and validation.
pub mod config;

/// Project initialization.
pub mod init;
