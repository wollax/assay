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

    /// Write config files and return a **full streaming command line**.
    ///
    /// Unlike [`write_harness`](Self::write_harness), the returned vec includes
    /// the binary name as `args[0]` and uses streaming-compatible flags (e.g.
    /// `--output-format stream-json` for Claude). This matches the contract of
    /// [`launch_agent_streaming`](https://docs.rs/assay-core) which treats
    /// `cli_args[0]` as the binary.
    ///
    /// When `prompt` is `Some`, the provider appends it as a user message
    /// using the provider-specific flag (e.g. `-p` for Claude).
    ///
    /// The default implementation prepends an empty binary placeholder and
    /// delegates to [`write_harness`](Self::write_harness). Providers should
    /// override this to supply the correct binary name and streaming flags.
    fn write_harness_streaming(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
        prompt: Option<&str>,
    ) -> Result<Vec<String>, HarnessError> {
        let args = self.write_harness(profile, working_dir)?;
        // Default: prepend empty binary (will fail at spawn — providers
        // should override with their actual binary name).
        let mut full = vec!["agent".to_string()];
        full.extend(args);
        if let Some(p) = prompt {
            full.push("-p".to_string());
            full.push(p.to_string());
        }
        Ok(full)
    }
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
