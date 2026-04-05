//! Provider dispatch for the TUI agent harness.
//!
//! `provider_harness_writer` is the single dispatch point: given the optional
//! loaded [`Config`], it returns a boxed [`HarnessProvider`] that
//! produces the full command line (binary + args) for the configured provider.
//!
//! # Binary name convention
//!
//! Unlike the pipeline path (where `launch_agent` hard-codes `"claude"`),
//! the TUI uses `launch_agent_streaming`, which expects `cli_args[0]` to be
//! the binary name and `cli_args[1..]` to be its arguments.  Every provider
//! returned by `provider_harness_writer` therefore includes the binary as the
//! first element.

use assay_harness::{HarnessError, HarnessProvider};
use assay_types::{Config, HarnessProfile, ProviderKind};
use std::path::Path;

/// TUI wrapper around [`assay_harness::provider::ClaudeProvider`] for streaming.
///
/// `launch_agent_streaming` interprets `cli_args[0]` as the binary name.
/// `ClaudeProvider::write_harness` returns only the *arguments* (no binary),
/// which is correct for the pipeline path where `launch_agent` hard-codes
/// `"claude"`.  This wrapper prepends `"claude"` so the TUI path works
/// correctly.
struct AnthropicStreamingProvider;

impl HarnessProvider for AnthropicStreamingProvider {
    fn write_harness(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        let args = assay_harness::provider::ClaudeProvider.write_harness(profile, working_dir)?;
        let mut full = vec!["claude".to_string()];
        full.extend(args);
        Ok(full)
    }
}

/// TUI-local Ollama provider.
///
/// Not in `assay-harness` because Ollama is not a first-class harness adapter —
/// it's a TUI convenience that bypasses config file generation entirely.
struct OllamaProvider {
    model: String,
}

impl HarnessProvider for OllamaProvider {
    fn write_harness(
        &self,
        _profile: &HarnessProfile,
        _working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        Ok(vec!["ollama".into(), "run".into(), self.model.clone()])
    }
}

/// TUI-local OpenAI provider.
///
/// Not in `assay-harness` because the OpenAI CLI adapter is not a first-class
/// harness adapter — it's a TUI convenience.
struct OpenAiProvider {
    model: String,
}

impl HarnessProvider for OpenAiProvider {
    fn write_harness(
        &self,
        _profile: &HarnessProfile,
        _working_dir: &Path,
    ) -> Result<Vec<String>, HarnessError> {
        Ok(vec![
            "openai".into(),
            "api".into(),
            "chat.completions.create".into(),
            "--model".into(),
            self.model.clone(),
        ])
    }
}

/// Return a boxed [`HarnessProvider`] that dispatches to the correct provider
/// adapter based on `config.provider.provider`.
///
/// The returned provider yields a full command line: `args[0]` is the binary
/// name, `args[1..]` are its arguments.  This matches what
/// [`assay_core::pipeline::launch_agent_streaming`] expects.
///
/// When `config` is `None` (no config file loaded) the function falls back to
/// `ProviderKind::Anthropic`.
pub fn provider_harness_writer(config: Option<&Config>) -> Box<dyn HarnessProvider> {
    let provider = config
        .and_then(|c| c.provider.as_ref())
        .map(|p| p.provider)
        .unwrap_or(ProviderKind::Anthropic);

    let model_opt = config
        .and_then(|c| c.provider.as_ref())
        .and_then(|p| p.execution_model.clone());

    match provider {
        ProviderKind::Anthropic => Box::new(AnthropicStreamingProvider),
        ProviderKind::Ollama => {
            let model = model_opt.unwrap_or_else(|| "llama3".into());
            Box::new(OllamaProvider { model })
        }
        ProviderKind::OpenAi => {
            let model = model_opt.unwrap_or_else(|| "gpt-4o".into());
            Box::new(OpenAiProvider { model })
        }
    }
}
