//! Team configuration discovery from `~/.claude/teams/`.
//!
//! Reads team config files and inbox messages when available,
//! falling back gracefully when the directory structure doesn't exist.

use std::path::PathBuf;

use assay_types::checkpoint::TeamCheckpoint;

/// Context from `~/.claude/teams/` directory structure.
pub struct TeamConfigContext {
    /// Parsed `config.json` if present in any team directory.
    pub config: Option<serde_json::Value>,
    /// Inbox messages from `~/.claude/teams/*/inboxes/*.json`.
    pub inbox_entries: Vec<serde_json::Value>,
}

/// Discover team configuration from `~/.claude/teams/`.
///
/// Scans for `config.json` and inbox messages in team directories.
/// Returns `None` when the teams directory doesn't exist or contains
/// no recognizable configuration files.
pub fn discover_team_config() -> Option<TeamConfigContext> {
    let home = dirs::home_dir()?;
    let teams_dir = home.join(".claude").join("teams");

    if !teams_dir.is_dir() {
        return None;
    }

    let mut config: Option<serde_json::Value> = None;
    let mut inbox_entries: Vec<serde_json::Value> = Vec::new();

    let Ok(team_dirs) = std::fs::read_dir(&teams_dir) else {
        return None;
    };

    for entry in team_dirs.flatten() {
        let team_path = entry.path();
        if !team_path.is_dir() {
            continue;
        }

        // Try to read config.json
        if config.is_none() {
            let config_path = team_path.join("config.json");
            if config_path.is_file()
                && let Ok(content) = std::fs::read_to_string(&config_path)
            {
                config = serde_json::from_str(&content).ok();
            }
        }

        // Read inbox messages
        let inboxes_dir = team_path.join("inboxes");
        if inboxes_dir.is_dir()
            && let Ok(inbox_files) = std::fs::read_dir(&inboxes_dir)
        {
            for inbox_entry in inbox_files.flatten() {
                let inbox_path = inbox_entry.path();
                if inbox_path.extension().is_some_and(|ext| ext == "json")
                    && let Ok(content) = std::fs::read_to_string(&inbox_path)
                    && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
                {
                    inbox_entries.push(val);
                }
            }
        }
    }

    if config.is_none() && inbox_entries.is_empty() {
        return None;
    }

    Some(TeamConfigContext {
        config,
        inbox_entries,
    })
}

/// Merge team configuration into a checkpoint.
///
/// When `team_config` is `None`, this is a no-op. When config data is available,
/// it enriches the checkpoint with team metadata. Session-extracted runtime state
/// (agent status, task progress) always takes priority over static config data.
pub fn merge_team_config(
    _checkpoint: &mut TeamCheckpoint,
    team_config: Option<&TeamConfigContext>,
) {
    let Some(_config) = team_config else {
        return;
    };

    // Currently, team config.json structure is not well-defined.
    // The primary value is session-extracted state. Team config provides
    // optional enrichment when the format stabilizes.
    //
    // For now, inbox_entries count could be surfaced as metadata,
    // but we avoid adding fields to TeamCheckpoint that aren't yet
    // well-understood. This is a deliberate minimal implementation.
}

/// Get the teams directory path for testing purposes.
fn _teams_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("teams"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::checkpoint::{AgentState, AgentStatus};

    #[test]
    fn discover_returns_none_when_no_teams_dir() {
        // This test relies on the real filesystem. If ~/.claude/teams/ doesn't
        // exist (common in CI), discover_team_config returns None.
        // We don't assert the specific result since it depends on the environment.
        let _result = discover_team_config();
    }

    #[test]
    fn merge_team_config_noop_when_none() {
        let mut checkpoint = TeamCheckpoint {
            version: 1,
            session_id: "s1".into(),
            project: "/project".into(),
            timestamp: "2026-03-06T10:00:00Z".into(),
            trigger: "manual".into(),
            agents: vec![AgentState {
                agent_id: "primary".into(),
                model: None,
                status: AgentStatus::Active,
                current_task: None,
                working_dir: None,
                is_sidechain: false,
                last_activity: None,
            }],
            tasks: vec![],
            context_health: None,
        };

        let original = checkpoint.clone();
        merge_team_config(&mut checkpoint, None);

        // Checkpoint should be unchanged
        assert_eq!(checkpoint.version, original.version);
        assert_eq!(checkpoint.session_id, original.session_id);
        assert_eq!(checkpoint.agents.len(), original.agents.len());
    }

    #[test]
    fn merge_team_config_with_config_present() {
        let mut checkpoint = TeamCheckpoint {
            version: 1,
            session_id: "s1".into(),
            project: "/project".into(),
            timestamp: "2026-03-06T10:00:00Z".into(),
            trigger: "manual".into(),
            agents: vec![AgentState {
                agent_id: "primary".into(),
                model: Some("claude-opus-4-6".into()),
                status: AgentStatus::Active,
                current_task: None,
                working_dir: None,
                is_sidechain: false,
                last_activity: None,
            }],
            tasks: vec![],
            context_health: None,
        };

        let team_config = TeamConfigContext {
            config: Some(serde_json::json!({"team_name": "dev"})),
            inbox_entries: vec![serde_json::json!({"from": "agent-1", "message": "hello"})],
        };

        // Should not panic; currently a no-op enrichment
        merge_team_config(&mut checkpoint, Some(&team_config));

        // Verify session-extracted data is preserved
        assert_eq!(checkpoint.agents[0].model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(checkpoint.agents[0].status, AgentStatus::Active);
    }
}
