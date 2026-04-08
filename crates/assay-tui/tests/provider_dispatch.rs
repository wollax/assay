use assay_tui::agent::provider_harness_writer;
use assay_types::{Config, HarnessProfile, ProviderConfig, ProviderKind, SettingsOverride};

fn config_with_provider(kind: ProviderKind) -> Config {
    Config {
        project_name: "test".into(),
        specs_dir: "specs/".into(),
        gates: None,
        guard: None,
        worktree: None,
        sessions: None,
        provider: Some(ProviderConfig {
            provider: kind,
            planning_model: None,
            execution_model: None,
            review_model: None,
        }),
    }
}

fn run_writer(config: &Config) -> Vec<String> {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let profile = HarnessProfile {
        name: "test".into(),
        prompt_layers: vec![],
        settings: SettingsOverride {
            model: None,
            permissions: vec![],
            tools: vec![],
            max_turns: None,
        },
        hooks: vec![],
        working_dir: None,
    };
    let writer = provider_harness_writer(Some(config));
    writer
        .write_harness_streaming(&profile, tmp.path(), None)
        .expect("writer should succeed")
}

#[test]
fn provider_dispatch_anthropic_uses_claude_binary() {
    let config = config_with_provider(ProviderKind::Anthropic);
    let args = run_writer(&config);
    assert!(
        !args.is_empty(),
        "expected non-empty args from AnthropicStreamingProvider"
    );
    // provider_harness_writer returns a full command line for launch_agent_streaming:
    // args[0] must be the binary name, args[1..] are its arguments.
    assert_eq!(
        args[0], "claude",
        "expected first element to be 'claude' (binary), got: {:?}",
        args[0]
    );
    // The actual flags follow — at minimum --print should be present.
    assert!(
        args.contains(&"--print".to_string()),
        "expected '--print' in args, got: {:?}",
        args
    );
}

#[test]
fn provider_dispatch_ollama_uses_ollama_binary() {
    let config = config_with_provider(ProviderKind::Ollama);
    let args = run_writer(&config);
    assert_eq!(args[0], "ollama");
}

#[test]
fn provider_dispatch_openai_uses_openai_binary() {
    let config = config_with_provider(ProviderKind::OpenAi);
    let args = run_writer(&config);
    assert_eq!(args[0], "openai");
}
