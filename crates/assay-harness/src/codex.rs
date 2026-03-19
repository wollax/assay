//! Codex adapter for harness profile generation.
//!
//! Translates a [`HarnessProfile`](assay_types::HarnessProfile) into the
//! concrete configuration files and CLI arguments needed to launch a Codex
//! agent session.
//!
//! Codex uses:
//! - `AGENTS.md` for system prompt (assembled from prompt layers)
//! - `.codex/config.toml` for TOML-based settings
//! - `codex exec --full-auto` for CLI invocation
//!
//! Unlike Claude Code, Codex has no native hook mechanism. Hooks from the
//! profile are mapped to advisory text appended to `AGENTS.md`.

use std::path::Path;

use assay_types::{HarnessProfile, HookEvent};
use serde::Serialize;

use crate::prompt::build_prompt;

/// Generated Codex configuration artifacts.
///
/// All fields are pre-serialized strings ready to be written to disk.
/// This struct is produced by [`generate_config`] and consumed by
/// [`write_config`] and [`build_cli_args`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexConfig {
    /// Assembled AGENTS.md content from prompt layers, with optional hook advisory text.
    pub agents_md: String,
    /// TOML-serialized `.codex/config.toml` content.
    pub config_toml: String,
    /// Model identifier for CLI arg generation, if overridden.
    pub model: Option<String>,
}

/// Internal TOML structure for `.codex/config.toml`.
#[derive(Debug, Serialize)]
struct CodexConfigToml {
    /// Model identifier (e.g., `"o3"`, `"o4-mini"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    /// Approval policy — always `"full-auto"` for harness sessions.
    approval_policy: String,
    /// Sandbox permissions mode.
    sandbox_mode: String,
}

/// Map a [`HookEvent`] to a human-readable description for advisory text.
fn hook_event_label(event: HookEvent) -> &'static str {
    match event {
        HookEvent::PreTool => "before each tool invocation",
        HookEvent::PostTool => "after each tool invocation",
        HookEvent::Stop => "when the session stops",
    }
}

/// Build advisory text for hooks that Codex cannot natively execute.
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
        "executed natively by Codex. They are listed here for reference:".to_string(),
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

/// Determine the sandbox mode from permissions.
///
/// - Empty permissions → `"workspace-write"` (safe default with write access)
/// - Permissions containing network or system-level access → `"danger-full-access"`
/// - Otherwise → `"workspace-write"`
fn resolve_sandbox_mode(permissions: &[String]) -> &'static str {
    if permissions.is_empty() {
        return "workspace-write";
    }

    let needs_full_access = permissions.iter().any(|p| {
        let lower = p.to_lowercase();
        lower.contains("network")
            || lower.contains("http")
            || lower.contains("system")
            || lower.contains("admin")
    });

    if needs_full_access {
        "danger-full-access"
    } else {
        "workspace-write"
    }
}

/// Translate a [`HarnessProfile`] into Codex configuration artifacts.
///
/// This is a pure function — no I/O, no side effects. The returned
/// [`CodexConfig`] contains pre-serialized strings for each artifact file.
pub fn generate_config(profile: &HarnessProfile) -> CodexConfig {
    // 1. Build AGENTS.md from prompt layers + optional hook advisory.
    let mut agents_md = build_prompt(&profile.prompt_layers);
    let hook_advisory = build_hook_advisory(&profile.hooks);
    if !hook_advisory.is_empty() {
        agents_md.push_str(&hook_advisory);
    }

    // 2. Build config.toml via serde serialization.
    let sandbox_mode = resolve_sandbox_mode(&profile.settings.permissions);
    let config_struct = CodexConfigToml {
        model: profile.settings.model.clone(),
        approval_policy: "full-auto".to_string(),
        sandbox_mode: sandbox_mode.to_string(),
    };
    let config_toml = toml::to_string_pretty(&config_struct)
        .expect("CodexConfigToml serialization should never fail");

    CodexConfig {
        agents_md,
        config_toml,
        model: profile.settings.model.clone(),
    }
}

/// Write Codex configuration files to the given directory.
///
/// Creates the following layout:
/// - `dir/AGENTS.md` — assembled prompt with hook advisory (skipped when empty)
/// - `dir/.codex/config.toml` — Codex settings
///
/// The `.codex/` subdirectory is created automatically if it doesn't exist.
pub fn write_config(config: &CodexConfig, dir: &Path) -> std::io::Result<()> {
    // AGENTS.md — skip if empty.
    if !config.agents_md.is_empty() {
        std::fs::write(dir.join("AGENTS.md"), &config.agents_md)?;
    }

    // .codex/config.toml
    let codex_dir = dir.join(".codex");
    std::fs::create_dir_all(&codex_dir)?;
    std::fs::write(codex_dir.join("config.toml"), &config.config_toml)?;

    Ok(())
}

/// Build CLI arguments for a `codex exec` invocation.
///
/// The returned vector contains the subcommand and flags (no binary name).
/// Callers prepend the `codex` binary path themselves.
pub fn build_cli_args(config: &CodexConfig) -> Vec<String> {
    let mut args = vec!["exec".to_string(), "--full-auto".to_string()];

    if let Some(ref model) = config.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }

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
            name: "full-codex".to_string(),
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
                model: Some("o3".to_string()),
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

        assert_snapshot!("codex_realistic_agents_md", config.agents_md);
        assert_snapshot!("codex_realistic_config_toml", config.config_toml);
        assert_eq!(config.model, Some("o3".to_string()));
    }

    /// Minimal profile — empty prompt layers, default settings, no hooks.
    #[test]
    fn minimal_profile() {
        let profile = HarnessProfile {
            name: "minimal-codex".to_string(),
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

        assert_snapshot!("codex_minimal_agents_md", config.agents_md);
        assert_snapshot!("codex_minimal_config_toml", config.config_toml);
        assert_eq!(config.model, None);
    }

    /// Hooks present, no model — tests advisory text and default timeout.
    #[test]
    fn hooks_no_model() {
        let profile = HarnessProfile {
            name: "hooks-only-codex".to_string(),
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

        assert_snapshot!("codex_hooks_no_model_agents_md", config.agents_md);
        assert_snapshot!("codex_hooks_no_model_config_toml", config.config_toml);
        assert_eq!(config.model, None);
    }

    /// Verify hooks advisory text appears in agents_md when hooks are non-empty.
    #[test]
    fn hooks_advisory_in_agents_md() {
        let profile = HarnessProfile {
            name: "hook-advisory-codex".to_string(),
            prompt_layers: vec![PromptLayer {
                kind: PromptLayerKind::System,
                name: "Rules".to_string(),
                content: "Follow the rules.".to_string(),
                priority: 0,
            }],
            settings: SettingsOverride {
                model: None,
                permissions: vec![],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![HookContract {
                event: HookEvent::Stop,
                command: "notify-done.sh".to_string(),
                timeout_secs: Some(60),
            }],
            working_dir: None,
        };

        let config = generate_config(&profile);

        assert!(config.agents_md.contains("## Hook Advisory"));
        assert!(config.agents_md.contains("notify-done.sh"));
        assert!(config.agents_md.contains("when the session stops"));
        assert!(config.agents_md.contains("(timeout: 60s)"));
    }

    // ── write_config tests ──────────────────────────────────────────

    /// Full config writes all expected files to the directory.
    #[test]
    fn write_config_full() {
        let config = CodexConfig {
            agents_md: "# Agent\nFollow best practices.".to_string(),
            config_toml: "approval_policy = \"full-auto\"\nsandbox_mode = \"workspace-write\"\n"
                .to_string(),
            model: Some("o3".to_string()),
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        // AGENTS.md
        let agents_md = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert_eq!(agents_md, config.agents_md);

        // .codex/config.toml
        let config_toml = std::fs::read_to_string(dir.path().join(".codex/config.toml")).unwrap();
        assert_eq!(config_toml, config.config_toml);
    }

    /// write_config creates the .codex/ subdirectory automatically.
    #[test]
    fn write_config_creates_codex_dir() {
        let config = CodexConfig {
            agents_md: String::new(),
            config_toml: "approval_policy = \"full-auto\"\n".to_string(),
            model: None,
        };

        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join(".codex").exists());

        write_config(&config, dir.path()).unwrap();

        assert!(dir.path().join(".codex").is_dir());
        assert!(dir.path().join(".codex/config.toml").exists());
    }

    /// write_config skips AGENTS.md when agents_md is empty.
    #[test]
    fn write_config_skips_empty_agents_md() {
        let config = CodexConfig {
            agents_md: String::new(),
            config_toml: "approval_policy = \"full-auto\"\n".to_string(),
            model: None,
        };

        let dir = tempfile::tempdir().unwrap();
        write_config(&config, dir.path()).unwrap();

        assert!(!dir.path().join("AGENTS.md").exists());
        // config.toml still written.
        assert!(dir.path().join(".codex/config.toml").exists());
    }

    // ── build_cli_args tests ────────────────────────────────────────

    /// Full config produces expected argument list.
    #[test]
    fn build_cli_args_full() {
        let config = CodexConfig {
            agents_md: "You are a coding agent.".to_string(),
            config_toml: String::new(),
            model: Some("o3".to_string()),
        };

        let args = build_cli_args(&config);
        assert_snapshot!("codex_cli_args_full", args.join("\n"));
    }

    /// No model omits --model flag.
    #[test]
    fn build_cli_args_no_model() {
        let config = CodexConfig {
            agents_md: "Instructions.".to_string(),
            config_toml: String::new(),
            model: None,
        };

        let args = build_cli_args(&config);
        assert_snapshot!("codex_cli_args_no_model", args.join("\n"));
    }

    /// Minimal config — no model, empty agents_md.
    #[test]
    fn build_cli_args_minimal() {
        let config = CodexConfig {
            agents_md: String::new(),
            config_toml: String::new(),
            model: None,
        };

        let args = build_cli_args(&config);
        assert_snapshot!("codex_cli_args_minimal", args.join("\n"));
    }

    // ── sandbox_mode tests ──────────────────────────────────────────

    /// Network permissions escalate to danger-full-access.
    #[test]
    fn sandbox_escalation_network() {
        let profile = HarnessProfile {
            name: "network-codex".to_string(),
            prompt_layers: vec![],
            settings: SettingsOverride {
                model: None,
                permissions: vec!["network".to_string(), "Bash(*)".to_string()],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![],
            working_dir: None,
        };

        let config = generate_config(&profile);
        assert!(config.config_toml.contains("danger-full-access"));
    }

    /// Standard permissions stay at workspace-write.
    #[test]
    fn sandbox_default_workspace_write() {
        let profile = HarnessProfile {
            name: "standard-codex".to_string(),
            prompt_layers: vec![],
            settings: SettingsOverride {
                model: None,
                permissions: vec!["Bash(*)".to_string(), "Read(*)".to_string()],
                tools: vec![],
                max_turns: None,
            },
            hooks: vec![],
            working_dir: None,
        };

        let config = generate_config(&profile);
        assert!(config.config_toml.contains("workspace-write"));
        assert!(!config.config_toml.contains("danger-full-access"));
    }
}
