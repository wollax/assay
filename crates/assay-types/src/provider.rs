//! Harness provider trait.
//!
//! [`HarnessProvider`] is the extension point for agent adapters. Implement it
//! to plug a new agent into the assay pipeline without modifying pipeline code.

use std::path::Path;

use crate::HarnessProfile;

/// Boxed error type returned by [`HarnessProvider::write_harness`].
///
/// Using a boxed trait object preserves the original error source chain
/// (e.g. an [`std::io::Error`] from writing config files), whereas a plain
/// [`String`] would discard it.
pub type HarnessError = Box<dyn std::error::Error + Send + Sync>;

/// A provider that generates harness configuration and CLI arguments for an agent.
///
/// The pipeline calls [`write_harness`](HarnessProvider::write_harness) once per
/// session. The provider writes any config files the agent needs into
/// `working_dir` and returns the CLI arguments to launch the agent subprocess.
///
/// # Object Safety
///
/// This trait is object-safe and can be used as `&dyn HarnessProvider`.
pub trait HarnessProvider {
    /// Write agent configuration files to `working_dir` and return CLI arguments.
    ///
    /// The returned `Vec<String>` is the argument list passed to the agent
    /// subprocess (after the binary name). An empty vec means "no arguments" —
    /// the pipeline will still attempt to spawn the agent, which will likely
    /// fail (useful for testing).
    ///
    /// # Errors
    ///
    /// Returns a [`HarnessError`] if writing configuration files fails (e.g.
    /// the working directory is not writable). The error preserves the
    /// original source chain so callers can surface the root cause.
    fn write_harness(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError>;
}

/// No-op provider for testing.
///
/// Returns an empty argument list and writes no files. Useful for verifying
/// that the pipeline accepts any [`HarnessProvider`] implementor.
pub struct NullProvider;

impl HarnessProvider for NullProvider {
    fn write_harness(
        &self,
        _profile: &HarnessProfile,
        _working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        Ok(vec![])
    }
}
