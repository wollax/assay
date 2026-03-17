//! Settings merger for resolving agent configuration overrides.
//!
//! Applies [`SettingsOverride`] values on top of base configuration to produce
//! the final agent session settings.

use assay_types::SettingsOverride;

/// Merge two [`SettingsOverride`] values, applying `overrides` on top of `base`.
///
/// # Merge Semantics
///
/// - **`Option` fields** (`model`, `max_turns`): override wins when `Some`,
///   otherwise base value is kept.
/// - **`Vec` fields** (`permissions`, `tools`): override **replaces** base
///   entirely when non-empty. An empty override Vec preserves the base Vec.
///   This is intentional replace-not-extend semantics — a profile that specifies
///   `tools: ["bash"]` means "only bash", not "bash plus whatever the base had".
///
/// Uses explicit struct construction (no `..base`) so that adding a new field
/// to `SettingsOverride` produces a compile error here, forcing the merger to
/// be updated.
pub fn merge_settings(base: &SettingsOverride, overrides: &SettingsOverride) -> SettingsOverride {
    SettingsOverride {
        model: overrides.model.clone().or_else(|| base.model.clone()),
        max_turns: overrides.max_turns.or(base.max_turns),
        permissions: if overrides.permissions.is_empty() {
            base.permissions.clone()
        } else {
            overrides.permissions.clone()
        },
        tools: if overrides.tools.is_empty() {
            base.tools.clone()
        } else {
            overrides.tools.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::{HarnessProfile, HookContract, HookEvent, PromptLayerKind};

    fn base_settings() -> SettingsOverride {
        SettingsOverride {
            model: Some("sonnet".into()),
            permissions: vec!["filesystem".into(), "network".into()],
            tools: vec!["bash".into(), "browser".into()],
            max_turns: Some(10),
        }
    }

    fn empty_settings() -> SettingsOverride {
        SettingsOverride {
            model: None,
            permissions: vec![],
            tools: vec![],
            max_turns: None,
        }
    }

    // --- Settings merger tests ---

    #[test]
    fn empty_overrides() {
        let base = base_settings();
        let result = merge_settings(&base, &empty_settings());
        assert_eq!(result, base);
    }

    #[test]
    fn full_override() {
        let base = base_settings();
        let overrides = SettingsOverride {
            model: Some("opus".into()),
            permissions: vec!["all".into()],
            tools: vec!["bash".into()],
            max_turns: Some(50),
        };
        let result = merge_settings(&base, &overrides);
        assert_eq!(result, overrides);
    }

    #[test]
    fn partial_override_model() {
        let base = base_settings();
        let overrides = SettingsOverride {
            model: Some("opus".into()),
            ..empty_settings()
        };
        let result = merge_settings(&base, &overrides);
        assert_eq!(result.model, Some("opus".into()));
        assert_eq!(result.permissions, base.permissions);
        assert_eq!(result.tools, base.tools);
        assert_eq!(result.max_turns, base.max_turns);
    }

    #[test]
    fn partial_override_max_turns() {
        let base = base_settings();
        let overrides = SettingsOverride {
            max_turns: Some(99),
            ..empty_settings()
        };
        let result = merge_settings(&base, &overrides);
        assert_eq!(result.max_turns, Some(99));
        assert_eq!(result.model, base.model);
        assert_eq!(result.permissions, base.permissions);
        assert_eq!(result.tools, base.tools);
    }

    #[test]
    fn vec_replace_semantics() {
        let base = base_settings();
        let overrides = SettingsOverride {
            permissions: vec!["readonly".into()],
            tools: vec!["bash".into()],
            ..empty_settings()
        };
        let result = merge_settings(&base, &overrides);
        // Override Vecs replace base entirely — not appended
        assert_eq!(result.permissions, vec!["readonly".to_string()]);
        assert_eq!(result.tools, vec!["bash".to_string()]);
    }

    #[test]
    fn empty_vec_preserves_base() {
        let base = base_settings();
        let overrides = SettingsOverride {
            permissions: vec![],
            tools: vec![],
            ..empty_settings()
        };
        let result = merge_settings(&base, &overrides);
        assert_eq!(result.permissions, base.permissions);
        assert_eq!(result.tools, base.tools);
    }

    // --- Hook contract validation tests ---

    #[test]
    fn hook_contract_pre_tool() {
        let hook = HookContract {
            event: HookEvent::PreTool,
            command: "echo pre-tool".into(),
            timeout_secs: Some(30),
        };
        assert_eq!(hook.event, HookEvent::PreTool);
        assert_eq!(hook.command, "echo pre-tool");
        assert_eq!(hook.timeout_secs, Some(30));
    }

    #[test]
    fn hook_contract_post_tool() {
        let hook = HookContract {
            event: HookEvent::PostTool,
            command: "notify-done".into(),
            timeout_secs: None,
        };
        let json = serde_json::to_string(&hook).unwrap();
        let roundtrip: HookContract = serde_json::from_str(&json).unwrap();
        assert_eq!(hook, roundtrip);
    }

    #[test]
    fn hook_contract_stop() {
        let hook = HookContract {
            event: HookEvent::Stop,
            command: "cleanup.sh".into(),
            timeout_secs: Some(60),
        };
        let json = serde_json::to_string(&hook).unwrap();
        let roundtrip: HookContract = serde_json::from_str(&json).unwrap();
        assert_eq!(hook, roundtrip);
    }

    #[test]
    fn hook_contracts_realistic_profile() {
        let profile = HarnessProfile {
            name: "ci-review".into(),
            prompt_layers: vec![assay_types::PromptLayer {
                kind: PromptLayerKind::System,
                name: "base".into(),
                content: "You are a code reviewer.".into(),
                priority: 0,
            }],
            settings: base_settings(),
            hooks: vec![
                HookContract {
                    event: HookEvent::PreTool,
                    command: "audit-tool --check".into(),
                    timeout_secs: Some(5),
                },
                HookContract {
                    event: HookEvent::PostTool,
                    command: "log-tool-result".into(),
                    timeout_secs: None,
                },
                HookContract {
                    event: HookEvent::Stop,
                    command: "generate-report.sh".into(),
                    timeout_secs: Some(120),
                },
            ],
            working_dir: Some("/tmp/review".into()),
        };
        let json = serde_json::to_string_pretty(&profile).unwrap();
        let roundtrip: HarnessProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, roundtrip);
    }
}
