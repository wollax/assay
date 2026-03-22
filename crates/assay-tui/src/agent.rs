//! Provider dispatch for the TUI agent harness.
//!
//! `provider_harness_writer` is the single dispatch point: given the optional
//! loaded [`Config`], it returns a boxed [`HarnessWriter`] closure that
//! produces the correct CLI arguments for the configured provider.

use assay_core::pipeline::HarnessWriter;
use assay_types::{Config, ProviderKind};

/// TUI-local Ollama configuration (not persisted to `assay-types`).
pub struct OllamaConfig {
    pub model: String,
}

/// TUI-local OpenAI configuration (not persisted to `assay-types`).
pub struct OpenAiConfig {
    pub model: String,
    pub api_key_env: String,
}

/// Return a boxed [`HarnessWriter`] that dispatches to the correct provider
/// adapter based on `config.provider.provider`.
///
/// When `config` is `None` (no config file loaded) the function falls back to
/// `ProviderKind::Anthropic`.
pub fn provider_harness_writer(config: Option<&Config>) -> Box<HarnessWriter> {
    let provider = config
        .and_then(|c| c.provider.as_ref())
        .map(|p| p.provider)
        .unwrap_or(ProviderKind::Anthropic);

    let model_opt = config
        .and_then(|c| c.provider.as_ref())
        .and_then(|p| p.execution_model.clone());

    match provider {
        ProviderKind::Anthropic => Box::new(
            move |profile: &assay_types::HarnessProfile, path: &std::path::Path| {
                let cfg = assay_harness::claude::generate_config(profile);
                assay_harness::claude::write_config(&cfg, path).map_err(|e| e.to_string())?;
                let mut args = vec!["claude".to_string()];
                args.extend(assay_harness::claude::build_cli_args(&cfg));
                Ok(args)
            },
        ),
        ProviderKind::Ollama => {
            let model = model_opt.unwrap_or_else(|| "llama3".into());
            Box::new(
                move |_profile: &assay_types::HarnessProfile, _path: &std::path::Path| {
                    Ok(vec!["ollama".into(), "run".into(), model.clone()])
                },
            )
        }
        ProviderKind::OpenAi => {
            let model = model_opt.unwrap_or_else(|| "gpt-4o".into());
            Box::new(
                move |_profile: &assay_types::HarnessProfile, _path: &std::path::Path| {
                    Ok(vec![
                        "openai".into(),
                        "api".into(),
                        "chat.completions.create".into(),
                        "--model".into(),
                        model.clone(),
                    ])
                },
            )
        }
    }
}
