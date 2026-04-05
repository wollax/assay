//! Harness provider trait and built-in implementations.
//!
//! [`HarnessProvider`] is the extension point for agent adapters. Implement it
//! to plug a new agent into the assay pipeline without modifying pipeline code.
//!
//! The trait itself is defined in [`assay_types::provider`] so that `assay-core`
//! can reference it without a circular dependency on `assay-harness`.

use std::path::Path;

use assay_types::HarnessProfile;

// Re-export the trait, error type, and NullProvider from assay-types so
// existing consumers that import from `assay_harness::provider` continue to work.
pub use assay_types::provider::{HarnessError, HarnessProvider, NullProvider};

use assay_types::AgentEvent;

use crate::{claude, codex, opencode};

/// Claude Code adapter.
///
/// Delegates to [`claude::generate_config`], [`claude::write_config`], and
/// [`claude::build_cli_args`].
pub struct ClaudeProvider;

impl HarnessProvider for ClaudeProvider {
    fn write_harness(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        let config = claude::generate_config(profile);
        claude::write_config(&config, working_dir)?;
        Ok(claude::build_cli_args(&config))
    }
}

impl ClaudeProvider {
    /// Parse Claude streaming NDJSON output into typed agent events.
    ///
    /// Convenience wrapper around [`crate::claude_stream::parse_claude_events`]
    /// so callers can use the provider as the single entry point for
    /// Claude-related operations.
    pub fn parse_streaming_output(reader: impl std::io::BufRead) -> Vec<AgentEvent> {
        crate::claude_stream::parse_claude_events(reader)
    }
}

/// OpenAI Codex adapter.
///
/// Delegates to [`codex::generate_config`], [`codex::write_config`], and
/// [`codex::build_cli_args`].
pub struct CodexProvider;

impl HarnessProvider for CodexProvider {
    fn write_harness(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        let config = codex::generate_config(profile);
        codex::write_config(&config, working_dir)?;
        Ok(codex::build_cli_args(&config))
    }
}

/// OpenCode adapter.
///
/// Delegates to [`opencode::generate_config`], [`opencode::write_config`], and
/// [`opencode::build_cli_args`].
pub struct OpenCodeProvider;

impl HarnessProvider for OpenCodeProvider {
    fn write_harness(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        let config = opencode::generate_config(profile);
        opencode::write_config(&config, working_dir)?;
        Ok(opencode::build_cli_args(&config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{HarnessProfile, SettingsOverride};
    use tempfile::TempDir;

    fn minimal_profile() -> HarnessProfile {
        HarnessProfile {
            name: "test".to_string(),
            prompt_layers: vec![],
            settings: SettingsOverride {
                model: None,
                permissions: vec![],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![],
            working_dir: None,
        }
    }

    #[test]
    fn test_null_provider_returns_empty_args() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let result = NullProvider.write_harness(&profile, dir.path());
        assert!(result.is_ok(), "NullProvider should succeed");
        assert!(
            result.unwrap().is_empty(),
            "NullProvider should return empty args"
        );
    }

    #[test]
    fn test_claude_provider_produces_args() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let result = ClaudeProvider.write_harness(&profile, dir.path());
        assert!(result.is_ok(), "ClaudeProvider failed: {result:?}");
        let args = result.unwrap();
        assert!(
            !args.is_empty(),
            "ClaudeProvider should produce non-empty args"
        );
    }

    #[test]
    fn test_codex_provider_produces_args() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let result = CodexProvider.write_harness(&profile, dir.path());
        assert!(result.is_ok(), "CodexProvider failed: {result:?}");
        let args = result.unwrap();
        assert!(
            !args.is_empty(),
            "CodexProvider should produce non-empty args"
        );
    }

    #[test]
    fn test_opencode_provider_produces_args() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let result = OpenCodeProvider.write_harness(&profile, dir.path());
        assert!(result.is_ok(), "OpenCodeProvider failed: {result:?}");
        let args = result.unwrap();
        assert!(
            !args.is_empty(),
            "OpenCodeProvider should produce non-empty args"
        );
    }
}
