//! OpenCode adapter for harness profile generation.
//!
//! Translates a [`HarnessProfile`](assay_types::HarnessProfile) into the
//! concrete configuration files and CLI arguments needed to launch an OpenCode
//! agent session.
//!
//! OpenCode uses:
//! - `AGENTS.md` for system prompt (assembled from prompt layers)
//! - `opencode.json` for JSON-based settings (with `$schema`)
//! - `opencode run` for CLI invocation
//!
//! Like Codex, OpenCode has no native hook mechanism. Hooks from the
//! profile are mapped to advisory text appended to `AGENTS.md`.

use std::collections::BTreeMap;
use std::path::Path;

use assay_types::{HarnessProfile, HookEvent};
use serde::Serialize;

use crate::prompt::build_prompt;

/// Generated OpenCode configuration artifacts.
///
/// All fields are pre-serialized strings ready to be written to disk.
/// This struct is produced by [`generate_config`] and consumed by
/// [`write_config`] and [`build_cli_args`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCodeConfig {
    /// Assembled AGENTS.md content from prompt layers, with optional hook advisory text.
    pub agents_md: String,
    /// JSON-serialized `opencode.json` content.
    pub config_json: String,
    /// Model identifier for CLI arg generation, if overridden.
    pub model: Option<String>,
}

/// Agent configuration nested within the OpenCode config.
#[derive(Debug, Serialize)]
struct AgentConfig {
    /// Maximum number of agent steps before forced stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    steps: Option<u32>,
}

/// Internal JSON structure for `opencode.json`.
#[derive(Debug, Serialize)]
struct OpenCodeConfigJson {
    /// JSON Schema reference.
    #[serde(rename = "$schema")]
    schema: String,
    /// Model identifier (e.g., `"provider/model-id"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    /// Enabled tools map (`{"tool_name": true}`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    tools: BTreeMap<String, bool>,
    /// Permission map (`{"tool_name": "allow"}`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    permission: BTreeMap<String, String>,
    /// Agent configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<AgentConfig>,
}

/// Map a [`HookEvent`] to a human-readable description for advisory text.
fn hook_event_label(event: HookEvent) -> &'static str {
    match event {
        HookEvent::PreTool => "before each tool invocation",
        HookEvent::PostTool => "after each tool invocation",
        HookEvent::Stop => "when the session stops",
    }
}

/// Build advisory text for hooks that OpenCode cannot natively execute.
///
/// Returns an empty string if no hooks are present.
fn build_hook_advisory(hooks: &[assay_types::HookContract]) -> String {
    if hooks.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        String::new(),
        "---".to_string(),
        String::new(),
        "## Hook Advisory".to_string(),
        String::new(),
        "The following lifecycle hooks are defined for this session but cannot be".to_string(),
        "executed natively by OpenCode. They are listed here for reference:".to_string(),
        String::new(),
    ];

    for hook in hooks {
        let label = hook_event_label(hook.event);
        let timeout = hook
            .timeout_secs
            .map(|t| format!(" (timeout: {t}s)"))
            .unwrap_or_default();
        lines.push(format!("- **{label}**: `{}`{timeout}", hook.command));
    }

    lines.join("\n")
}

/// Translate a [`HarnessProfile`] into OpenCode configuration artifacts.
///
/// This is a pure function — no I/O, no side effects. The returned
/// [`OpenCodeConfig`] contains pre-serialized strings for each artifact file.
pub fn generate_config(profile: &HarnessProfile) -> OpenCodeConfig {
    // 1. Build AGENTS.md from prompt layers + optional hook advisory.
    let mut agents_md = build_prompt(&profile.prompt_layers);
    let hook_advisory = build_hook_advisory(&profile.hooks);
    if !hook_advisory.is_empty() {
        agents_md.push_str(&hook_advisory);
    }

    // 2. Build opencode.json via serde serialization.
    let tools: BTreeMap<String, bool> = profile
        .settings
        .tools
        .iter()
        .map(|t| (t.clone(), true))
        .collect();

    let permission: BTreeMap<String, String> = profile
        .settings
        .permissions
        .iter()
        .map(|p| (p.clone(), "allow".to_string()))
        .collect();

    let agent = profile
        .settings
        .max_turns
        .map(|steps| AgentConfig { steps: Some(steps) });

    let config_struct = OpenCodeConfigJson {
        schema: "https://opencode.ai/config.json".to_string(),
        model: profile.settings.model.clone(),
        tools,
        permission,
        agent,
    };

    let config_json = serde_json::to_string_pretty(&config_struct)
        .expect("OpenCodeConfigJson serialization should never fail");

    OpenCodeConfig {
        agents_md,
        config_json,
        model: profile.settings.model.clone(),
    }
}

/// Write OpenCode configuration files to the given directory.
///
/// Creates the following layout:
/// - `dir/AGENTS.md` — assembled prompt with hook advisory (skipped when empty)
/// - `dir/opencode.json` — OpenCode settings
pub fn write_config(config: &OpenCodeConfig, dir: &Path) -> std::io::Result<()> {
    // AGENTS.md — skip if empty.
    if !config.agents_md.is_empty() {
        std::fs::write(dir.join("AGENTS.md"), &config.agents_md)?;
    }

    // opencode.json
    std::fs::write(dir.join("opencode.json"), &config.config_json)?;

    Ok(())
}

/// Build CLI arguments for an `opencode run` invocation.
///
/// The returned vector contains the subcommand and flags (no binary name).
/// Callers prepend the `opencode` binary path themselves.
pub fn build_cli_args(config: &OpenCodeConfig) -> Vec<String> {
    let mut args = vec!["run".to_string()];

    if let Some(ref model) = config.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }

    args.push("--format".to_string());
    args.push("json".to_string());

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{
        HarnessProfile, HookContract, HookEvent, PromptLayer, PromptLayerKind, SettingsOverride,
    };
    use insta::assert_snapshot;

    /// Realistic profile with prompt layers, settings, and hooks.
    #[test]
    fn realistic_profile() {
        let profile = HarnessProfile {
            name: "full-opencode".to_string(),
            prompt_layers: vec![
                PromptLayer {
                    kind: PromptLayerKind::System,
                    name: "Agent Rules".to_string(),
                    content: "You are a coding agent. Follow best practices.".to_string(),
                    priority: 0,
                },
                PromptLayer {
                    kind: PromptLayerKind::Project,
                    name: "Project Context".to_string(),
                    content: "This is a Rust workspace using Cargo.".to_string(),
                    priority: 10,
                },
                PromptLayer {
                    kind: PromptLayerKind::Spec,
                    name: "Task Spec".to_string(),
                    content: "Implement the login endpoint with JWT auth.".to_string(),
                    priority: 20,
                },
            ],
            settings: SettingsOverride {
                model: Some("anthropic/claude-sonnet".to_string()),
                permissions: vec![
                    "edit".to_string(),
                    "bash".to_string(),
                    "browser".to_string(),
                ],
                tools: vec!["bash".to_string(), "browser".to_string()],
                max_turns: Some(50),
            },
            hooks: vec![
                HookContract {
                    event: HookEvent::PreTool,
                    command: "bash scripts/pre-tool-check.sh".to_string(),
                    timeout_secs: Some(10),
                },
                HookContract {
                    event: HookEvent::PostTool,
                    command: "bash scripts/post-tool-log.sh".to_string(),
                    timeout_secs: Some(5),
                },
                HookContract {
                    event: HookEvent::Stop,
                    command: "bash scripts/on-stop.sh".to_string(),
                    timeout_secs: Some(120),
                },
            ],
            working_dir: Some("/home/user/project".to_string()),
        };

        let config = generate_config(&profile);

        assert_snapshot!("opencode_realistic_agents_md", config.agents_md);
        assert_snapshot!("opencode_realistic_config_json", config.config_json);
        assert_eq!(config.model, Some("anthropic/claude-sonnet".to_string()));
    }

    /// Minimal profile — empty prompt layers, default settings, no hooks.
    #[test]
    fn minimal_profile() {
        let profile = HarnessProfile {
            name: "minimal-opencode".to_string(),
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

        let config = generate_config(&profile);

        assert_snapshot!("opencode_minimal_agents_md", config.agents_md);
        assert_snapshot!("opencode_minimal_config_json", config.config_json);
        assert_eq!(config.model, None);
    }

    /// Hooks present, no model — tests advisory text and default timeout.
    #[test]
    fn hooks_no_model() {
        let profile = HarnessProfile {
            name: "hooks-only-opencode".to_string(),
            prompt_layers: vec![PromptLayer {
                kind: PromptLayerKind::System,
                name: "Base".to_string(),
                content: "Base instructions.".to_string(),
                priority: 0,
            }],
            settings: SettingsOverride {
                model: None,
                permissions: vec!["bash".to_string()],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![
                HookContract {
                    event: HookEvent::PreTool,
                    command: "echo pre".to_string(),
                    timeout_secs: None,
                },
                HookContract {
                    event: HookEvent::PostTool,
                    command: "echo post".to_string(),
                    timeout_secs: None,
                },
            ],
            working_dir: None,
        };

        let config = generate_config(&profile);

        assert_snapshot!("opencode_hooks_no_model_agents_md", config.agents_md);
        assert_snapshot!("opencode_hooks_no_model_config_json", config.config_json);
        assert_eq!(config.model, None);
    }

    /// Verify `$schema` field is present in config_json.
    #[test]
    fn schema_field_present() {
        let profile = HarnessProfile {
            name: "schema-check".to_string(),
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

        let config = generate_config(&profile);
        assert!(config.config_json.contains("\"$schema\""));
        assert!(
            config
                .config_json
                .contains("https://opencode.ai/config.json")
        );
    }

    // ── write_config tests ──────────────────────────────────────────

    /// Full config writes all expected files to the directory.
    #[test]
    fn write_config_full() {
        let config = OpenCodeConfig {
            agents_md: "# Agent\nFollow best practices.".to_string(),
            config_json: r#"{"$schema": "https://opencode.ai/config.json"}"#.to_string(),
            model: Some("anthropic/claude-sonnet".to_string()),
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        // AGENTS.md
        let agents_md = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert_eq!(agents_md, config.agents_md);

        // opencode.json
        let config_json = std::fs::read_to_string(dir.path().join("opencode.json")).unwrap();
        assert_eq!(config_json, config.config_json);
    }

    /// write_config creates both files.
    #[test]
    fn write_config_creates_files() {
        let config = OpenCodeConfig {
            agents_md: "Instructions".to_string(),
            config_json: "{}".to_string(),
            model: None,
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        assert!(dir.path().join("AGENTS.md").exists());
        assert!(dir.path().join("opencode.json").exists());
    }

    /// write_config skips AGENTS.md when agents_md is empty.
    #[test]
    fn write_config_skips_empty_agents_md() {
        let config = OpenCodeConfig {
            agents_md: String::new(),
            config_json: "{}".to_string(),
            model: None,
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        assert!(!dir.path().join("AGENTS.md").exists());
        // opencode.json still written.
        assert!(dir.path().join("opencode.json").exists());
    }

    // ── build_cli_args tests ────────────────────────────────────────

    /// Full config produces expected argument list.
    #[test]
    fn build_cli_args_full() {
        let config = OpenCodeConfig {
            agents_md: "You are a coding agent.".to_string(),
            config_json: String::new(),
            model: Some("anthropic/claude-sonnet".to_string()),
        };

        let args = build_cli_args(&config);
        assert_snapshot!("opencode_cli_args_full", args.join("\n"));
    }

    /// No model omits --model flag.
    #[test]
    fn build_cli_args_no_model() {
        let config = OpenCodeConfig {
            agents_md: "Instructions.".to_string(),
            config_json: String::new(),
            model: None,
        };

        let args = build_cli_args(&config);
        assert_snapshot!("opencode_cli_args_no_model", args.join("\n"));
    }

    /// Minimal config — no model, empty agents_md.
    #[test]
    fn build_cli_args_minimal() {
        let config = OpenCodeConfig {
            agents_md: String::new(),
            config_json: String::new(),
            model: None,
        };

        let args = build_cli_args(&config);
        assert_snapshot!("opencode_cli_args_minimal", args.join("\n"));
    }
}
