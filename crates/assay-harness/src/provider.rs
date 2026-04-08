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

    fn write_harness_streaming(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
        prompt: Option<&str>,
    ) -> Result<Vec<String>, HarnessError> {
        let config = claude::generate_config(profile);
        claude::write_config(&config, working_dir)?;
        let mut args = vec!["claude".to_string()];
        args.extend(claude::build_streaming_cli_args(&config));
        if let Some(p) = prompt {
            args.push("-p".to_string());
            args.push(p.to_string());
        }
        Ok(args)
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

    fn write_harness_streaming(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
        prompt: Option<&str>,
    ) -> Result<Vec<String>, HarnessError> {
        let args = self.write_harness(profile, working_dir)?;
        let mut full = vec!["codex".to_string()];
        full.extend(args);
        if let Some(p) = prompt {
            full.push("--prompt".to_string());
            full.push(p.to_string());
        }
        Ok(full)
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

    fn write_harness_streaming(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
        prompt: Option<&str>,
    ) -> Result<Vec<String>, HarnessError> {
        let args = self.write_harness(profile, working_dir)?;
        let mut full = vec!["opencode".to_string()];
        full.extend(args);
        if let Some(p) = prompt {
            full.push("--prompt".to_string());
            full.push(p.to_string());
        }
        Ok(full)
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

    #[test]
    fn test_claude_streaming_prepends_binary_and_uses_stream_json() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let args = ClaudeProvider
            .write_harness_streaming(&profile, dir.path(), None)
            .expect("streaming harness failed");
        assert_eq!(args[0], "claude", "first arg must be binary name");
        assert!(
            args.contains(&"stream-json".to_string()),
            "must use stream-json format"
        );
        assert!(
            args.contains(&"--verbose".to_string()),
            "must include --verbose"
        );
        assert!(
            !args.iter().any(|a| a == "-p"),
            "must not include -p when prompt is None"
        );
    }

    #[test]
    fn test_claude_streaming_with_prompt() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let args = ClaudeProvider
            .write_harness_streaming(&profile, dir.path(), Some("Do the task"))
            .expect("streaming harness failed");
        assert_eq!(args[0], "claude");
        let p_idx = args.iter().position(|a| a == "-p").expect("-p not found");
        assert_eq!(args[p_idx + 1], "Do the task");
    }

    #[test]
    fn test_codex_streaming_prepends_binary() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let args = CodexProvider
            .write_harness_streaming(&profile, dir.path(), None)
            .expect("streaming harness failed");
        assert_eq!(args[0], "codex", "first arg must be codex binary");
    }

    #[test]
    fn test_opencode_streaming_prepends_binary() {
        let dir = TempDir::new().unwrap();
        let profile = minimal_profile();
        let args = OpenCodeProvider
            .write_harness_streaming(&profile, dir.path(), None)
            .expect("streaming harness failed");
        assert_eq!(args[0], "opencode", "first arg must be opencode binary");
    }
}
