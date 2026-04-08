//! Provider dispatch for the TUI agent harness.
//!
//! `provider_harness_writer` is the single dispatch point: given the optional
//! loaded [`Config`], it returns a boxed [`HarnessProvider`] that
//! produces the full command line (binary + args) for the configured provider.
//!
//! # Binary name convention
//!
//! Both the TUI and the pipeline use `launch_agent_streaming`, which expects
//! `cli_args[0]` to be the binary name. All providers implement
//! [`HarnessProvider::write_harness_streaming`] which returns the full
//! command line including the binary.

use assay_harness::{HarnessError, HarnessProvider};
use assay_types::{Config, HarnessProfile, ProviderKind};
use std::path::Path;

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

    fn write_harness_streaming(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
        _prompt: Option<&str>,
    ) -> Result<Vec<String>, HarnessError> {
        // Ollama CLI args already include the binary name.
        self.write_harness(profile, working_dir)
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

    fn write_harness_streaming(
        &self,
        profile: &HarnessProfile,
        working_dir: &Path,
        _prompt: Option<&str>,
    ) -> Result<Vec<String>, HarnessError> {
        // OpenAI CLI args already include the binary name.
        self.write_harness(profile, working_dir)
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
        ProviderKind::Anthropic => Box::new(assay_harness::provider::ClaudeProvider),
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
