//! Claude Code adapter for harness profile generation.
//!
//! Translates a [`HarnessProfile`](assay_types::HarnessProfile) into the
//! concrete configuration files and CLI arguments needed to launch a Claude
//! Code agent session.

use std::collections::BTreeMap;
use std::path::Path;

use assay_types::{HarnessProfile, HookEvent};
use serde_json::{Value, json};

use crate::prompt::build_prompt;

/// Generated Claude Code configuration artifacts.
///
/// All fields are pre-serialized strings ready to be written to disk.
/// This struct is produced by [`generate_config`] and consumed by the
/// file writer (T02) and CLI arg builder (T03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeConfig {
    /// Assembled CLAUDE.md content from prompt layers.
    pub claude_md: String,
    /// MCP server configuration JSON (`.mcp.json`).
    pub mcp_json: String,
    /// Claude Code settings JSON with embedded hooks.
    pub settings_json: String,
    /// Standalone hooks JSON (the `{ "hooks": { ... } }` format).
    pub hooks_json: String,
    /// Model identifier for CLI arg generation, if overridden.
    pub model: Option<String>,
}

/// Map a [`HookEvent`] to Claude Code's hook event key.
fn hook_event_key(event: HookEvent) -> &'static str {
    match event {
        HookEvent::PreTool => "PreToolUse",
        HookEvent::PostTool => "PostToolUse",
        HookEvent::Stop => "Stop",
    }
}

/// Build the hooks object from hook contracts.
///
/// Groups hooks by event, producing Claude Code's format:
/// ```json
/// { "EventName": [{ "matcher": "", "hooks": [{ "type": "command", "command": "...", "timeout": N }] }] }
/// ```
fn build_hooks(hooks: &[assay_types::HookContract]) -> Value {
    // Group hooks by event, preserving insertion order per event.
    let mut groups: BTreeMap<&str, Vec<Value>> = BTreeMap::new();
    for hook in hooks {
        let key = hook_event_key(hook.event);
        let timeout = hook.timeout_secs.unwrap_or(30);
        let entry = json!({
            "type": "command",
            "command": hook.command,
            "timeout": timeout,
        });
        groups.entry(key).or_default().push(entry);
    }

    // Convert each group into Claude's array-of-matcher-groups format.
    let mut result = serde_json::Map::new();
    for (event_key, hook_entries) in groups {
        let group = json!([{
            "matcher": "",
            "hooks": hook_entries,
        }]);
        result.insert(event_key.to_string(), group);
    }

    Value::Object(result)
}

/// Translate a [`HarnessProfile`] into Claude Code configuration artifacts.
///
/// This is a pure function — no I/O, no side effects. The returned
/// [`ClaudeConfig`] contains pre-serialized strings for each artifact file.
pub fn generate_config(profile: &HarnessProfile) -> ClaudeConfig {
    // 1. Build CLAUDE.md from prompt layers.
    let claude_md = build_prompt(&profile.prompt_layers);

    // 2. Build settings JSON.
    let hooks_value = build_hooks(&profile.hooks);

    let mut settings = serde_json::Map::new();

    // Permissions block.
    let permissions = json!({
        "allow": profile.settings.permissions,
        "deny": [],
    });
    settings.insert("permissions".to_string(), permissions);

    // Model (only if specified).
    if let Some(ref model) = profile.settings.model {
        settings.insert("model".to_string(), json!(model));
    }

    // Hooks (only if non-empty).
    if !profile.hooks.is_empty() {
        settings.insert("hooks".to_string(), hooks_value.clone());
    }

    let settings_json = serde_json::to_string_pretty(&Value::Object(settings)).unwrap();

    // 3. Build MCP JSON — empty servers wrapper.
    let mcp_json = serde_json::to_string_pretty(&json!({ "mcpServers": {} })).unwrap();

    // 4. Build standalone hooks JSON.
    let hooks_json = if profile.hooks.is_empty() {
        serde_json::to_string_pretty(&json!({ "hooks": {} })).unwrap()
    } else {
        serde_json::to_string_pretty(&json!({ "hooks": hooks_value })).unwrap()
    };

    ClaudeConfig {
        claude_md,
        mcp_json,
        settings_json,
        hooks_json,
        model: profile.settings.model.clone(),
    }
}

/// Write Claude Code configuration files to the given directory.
///
/// Creates the following layout:
/// - `dir/CLAUDE.md` — assembled prompt (skipped when `claude_md` is empty)
/// - `dir/.mcp.json` — MCP server configuration
/// - `dir/.claude/settings.json` — Claude Code settings with hooks
///
/// The `.claude/` subdirectory is created automatically if it doesn't exist.
pub fn write_config(config: &ClaudeConfig, dir: &Path) -> std::io::Result<()> {
    // CLAUDE.md — skip if empty.
    if !config.claude_md.is_empty() {
        std::fs::write(dir.join("CLAUDE.md"), &config.claude_md)?;
    }

    // .mcp.json
    std::fs::write(dir.join(".mcp.json"), &config.mcp_json)?;

    // .claude/settings.json
    let claude_dir = dir.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;
    std::fs::write(claude_dir.join("settings.json"), &config.settings_json)?;

    Ok(())
}

/// Build CLI arguments for a `claude --print` invocation.
///
/// The returned vector contains flags only (no binary name). Callers prepend
/// the `claude` binary path themselves.
pub fn build_cli_args(config: &ClaudeConfig) -> Vec<String> {
    let mut args = vec![
        "--print".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
    ];

    if let Some(ref model) = config.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }

    if !config.claude_md.is_empty() {
        args.push("--system-prompt".to_string());
        args.push(config.claude_md.clone());
    }

    args.push("--mcp-config".to_string());
    args.push(".mcp.json".to_string());

    args.push("--settings".to_string());
    args.push(".claude/settings.json".to_string());

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
            name: "full-profile".to_string(),
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
                model: Some("sonnet".to_string()),
                permissions: vec![
                    "Bash(*)".to_string(),
                    "Read(*)".to_string(),
                    "Write(*)".to_string(),
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

        assert_snapshot!("realistic_claude_md", config.claude_md);
        assert_snapshot!("realistic_settings_json", config.settings_json);
        assert_snapshot!("realistic_hooks_json", config.hooks_json);
        assert_snapshot!("realistic_mcp_json", config.mcp_json);
        assert_eq!(config.model, Some("sonnet".to_string()));
    }

    /// Minimal profile — empty prompt layers, default settings, no hooks.
    #[test]
    fn minimal_profile() {
        let profile = HarnessProfile {
            name: "minimal".to_string(),
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

        assert_snapshot!("minimal_claude_md", config.claude_md);
        assert_snapshot!("minimal_settings_json", config.settings_json);
        assert_snapshot!("minimal_hooks_json", config.hooks_json);
        assert_snapshot!("minimal_mcp_json", config.mcp_json);
        assert_eq!(config.model, None);
    }

    /// Profile with hooks but no model override — tests default timeout and missing model.
    #[test]
    fn hooks_no_model() {
        let profile = HarnessProfile {
            name: "hooks-only".to_string(),
            prompt_layers: vec![PromptLayer {
                kind: PromptLayerKind::System,
                name: "Base".to_string(),
                content: "Base instructions.".to_string(),
                priority: 0,
            }],
            settings: SettingsOverride {
                model: None,
                permissions: vec!["Bash(*)".to_string()],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![
                HookContract {
                    event: HookEvent::PreTool,
                    command: "echo pre".to_string(),
                    timeout_secs: None, // should default to 30
                },
                HookContract {
                    event: HookEvent::PostTool,
                    command: "echo post".to_string(),
                    timeout_secs: None, // should default to 30
                },
            ],
            working_dir: None,
        };

        let config = generate_config(&profile);

        assert_snapshot!("hooks_no_model_settings_json", config.settings_json);
        assert_snapshot!("hooks_no_model_hooks_json", config.hooks_json);
        assert_eq!(config.model, None);
    }

    /// Verify MCP JSON structural wrapper (always empty mcpServers).
    #[test]
    fn mcp_structural_wrapper() {
        let profile = HarnessProfile {
            name: "mcp-test".to_string(),
            prompt_layers: vec![],
            settings: SettingsOverride {
                model: Some("opus".to_string()),
                permissions: vec![],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![],
            working_dir: None,
        };

        let config = generate_config(&profile);

        assert_snapshot!("mcp_structural_wrapper", config.mcp_json);
        // Parse and verify structure programmatically too.
        let parsed: Value = serde_json::from_str(&config.mcp_json).unwrap();
        assert!(parsed["mcpServers"].is_object());
        assert_eq!(parsed["mcpServers"].as_object().unwrap().len(), 0);
    }

    // ── write_config tests ──────────────────────────────────────────

    /// Full config writes all expected files to the directory.
    #[test]
    fn write_config_full() {
        let config = ClaudeConfig {
            claude_md: "# Agent\nFollow best practices.".to_string(),
            mcp_json: r#"{"mcpServers":{}}"#.to_string(),
            settings_json: r#"{"permissions":{"allow":[],"deny":[]}}"#.to_string(),
            hooks_json: r#"{"hooks":{}}"#.to_string(),
            model: Some("sonnet".to_string()),
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        // CLAUDE.md
        let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert_eq!(claude_md, config.claude_md);

        // .mcp.json
        let mcp = std::fs::read_to_string(dir.path().join(".mcp.json")).unwrap();
        assert_eq!(mcp, config.mcp_json);

        // .claude/settings.json
        let settings = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        assert_eq!(settings, config.settings_json);
    }

    /// write_config creates the .claude/ subdirectory automatically.
    #[test]
    fn write_config_creates_claude_dir() {
        let config = ClaudeConfig {
            claude_md: String::new(),
            mcp_json: "{}".to_string(),
            settings_json: "{}".to_string(),
            hooks_json: "{}".to_string(),
            model: None,
        };

        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join(".claude").exists());

        write_config(&config, dir.path()).unwrap();

        assert!(dir.path().join(".claude").is_dir());
        assert!(dir.path().join(".claude/settings.json").exists());
    }

    /// write_config skips CLAUDE.md when claude_md is empty.
    #[test]
    fn write_config_skips_empty_claude_md() {
        let config = ClaudeConfig {
            claude_md: String::new(),
            mcp_json: r#"{"mcpServers":{}}"#.to_string(),
            settings_json: "{}".to_string(),
            hooks_json: "{}".to_string(),
            model: None,
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        assert!(!dir.path().join("CLAUDE.md").exists());
        // Other files still written.
        assert!(dir.path().join(".mcp.json").exists());
        assert!(dir.path().join(".claude/settings.json").exists());
    }

    // ── build_cli_args tests ────────────────────────────────────────

    /// Full config produces expected argument list.
    #[test]
    fn build_cli_args_full() {
        let config = ClaudeConfig {
            claude_md: "You are a coding agent.".to_string(),
            mcp_json: "{}".to_string(),
            settings_json: "{}".to_string(),
            hooks_json: "{}".to_string(),
            model: Some("sonnet".to_string()),
        };

        let args = build_cli_args(&config);
        assert_snapshot!("cli_args_full", args.join("\n"));
    }

    /// Minimal config (no model) omits --model flag.
    #[test]
    fn build_cli_args_no_model() {
        let config = ClaudeConfig {
            claude_md: "Instructions.".to_string(),
            mcp_json: "{}".to_string(),
            settings_json: "{}".to_string(),
            hooks_json: "{}".to_string(),
            model: None,
        };

        let args = build_cli_args(&config);
        assert!(!args.contains(&"--model".to_string()));
        assert!(args.contains(&"--print".to_string()));
        assert!(args.contains(&"--system-prompt".to_string()));
    }

    /// Empty claude_md omits --system-prompt flag.
    #[test]
    fn build_cli_args_empty_claude_md() {
        let config = ClaudeConfig {
            claude_md: String::new(),
            mcp_json: "{}".to_string(),
            settings_json: "{}".to_string(),
            hooks_json: "{}".to_string(),
            model: Some("opus".to_string()),
        };

        let args = build_cli_args(&config);
        assert!(!args.contains(&"--system-prompt".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"--print".to_string()));
    }
}
