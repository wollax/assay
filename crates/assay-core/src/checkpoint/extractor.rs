//! Team state extraction from session JSONL entries.
//!
//! Scans `ParsedEntry` vectors to discover agents, extract task operations,
//! detect compaction boundaries, and assemble a [`TeamCheckpoint`].

use std::collections::HashMap;
use std::path::Path;

use assay_types::checkpoint::{
    AgentState, AgentStatus, ContextHealthSnapshot, TaskState, TaskStatus, TeamCheckpoint,
};
use assay_types::context::SessionEntry;

use crate::context::ParsedEntry;

use super::config::{discover_team_config, merge_team_config};

/// Extract a complete team state checkpoint from a project's session JSONL.
///
/// Discovers the session directory, resolves the session file, parses it,
/// and assembles a [`TeamCheckpoint`] with agents, tasks, and context health.
pub fn extract_team_state(
    project_dir: &Path,
    session_id: Option<&str>,
    trigger: &str,
) -> crate::Result<TeamCheckpoint> {
    let session_dir = crate::context::find_session_dir(project_dir)?;
    let session_path = crate::context::resolve_session(&session_dir, session_id)?;
    let (entries, _skipped) = crate::context::parse_session(&session_path)?;

    let resolved_session_id = session_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let agents = extract_agents(&entries);
    let tasks = extract_tasks(&entries);
    let (last_compaction, compaction_trigger) = extract_compaction(&entries);

    let context_health = crate::context::quick_token_estimate(&session_path)
        .ok()
        .flatten()
        .map(|usage| {
            let context_tokens = usage.context_tokens();
            let context_window = crate::context::extract_usage(&entries)
                .map(|_| 200_000u64)
                .unwrap_or(200_000);
            let utilization_pct = if context_window > 0 {
                (context_tokens as f64 / context_window as f64) * 100.0
            } else {
                0.0
            };
            ContextHealthSnapshot {
                context_tokens,
                context_window,
                utilization_pct,
                last_compaction,
                compaction_trigger,
            }
        });

    let mut checkpoint = TeamCheckpoint {
        version: 1,
        session_id: resolved_session_id,
        project: project_dir.to_string_lossy().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        trigger: trigger.to_string(),
        agents,
        tasks,
        context_health,
    };

    // Enrich with team config if available
    let team_config = discover_team_config();
    merge_team_config(&mut checkpoint, team_config.as_ref());

    Ok(checkpoint)
}

/// Extract agent states from parsed session entries.
///
/// The primary agent is always present. Subagents are discovered from entries
/// with `is_sidechain == true` and an `agentId` field in progress entry data.
pub(crate) fn extract_agents(entries: &[ParsedEntry]) -> Vec<AgentState> {
    let mut agents: HashMap<String, AgentState> = HashMap::new();

    // Primary agent is always present
    let mut primary = AgentState {
        agent_id: "primary".to_string(),
        model: None,
        status: AgentStatus::Unknown,
        current_task: None,
        working_dir: None,
        is_sidechain: false,
        last_activity: None,
    };

    for parsed in entries {
        match &parsed.entry {
            SessionEntry::Assistant(a) => {
                if !a.meta.is_sidechain {
                    if let Some(msg) = &a.message
                        && msg.model.is_some()
                    {
                        primary.model = msg.model.clone();
                    }
                    primary.last_activity = Some(a.meta.timestamp.clone());
                    primary.working_dir = a.meta.cwd.clone();
                    primary.status = AgentStatus::Active;
                }
            }
            SessionEntry::User(u) => {
                if !u.meta.is_sidechain {
                    primary.last_activity = Some(u.meta.timestamp.clone());
                    if u.meta.cwd.is_some() {
                        primary.working_dir = u.meta.cwd.clone();
                    }
                }
            }
            SessionEntry::Progress(p) => {
                if p.meta.is_sidechain {
                    // Try to extract agentId for subagent discovery
                    if let Some(data) = &p.data
                        && let Some(agent_id) = data.get("agentId").and_then(|a| a.as_str())
                    {
                        let agent =
                            agents
                                .entry(agent_id.to_string())
                                .or_insert_with(|| AgentState {
                                    agent_id: agent_id.to_string(),
                                    model: None,
                                    status: AgentStatus::Active,
                                    current_task: None,
                                    working_dir: None,
                                    is_sidechain: true,
                                    last_activity: None,
                                });
                        agent.last_activity = Some(p.meta.timestamp.clone());
                        if p.meta.cwd.is_some() {
                            agent.working_dir = p.meta.cwd.clone();
                        }

                        // Try to extract model from nested assistant message
                        if let Some(model) = data
                            .pointer("/message/message/model")
                            .and_then(|m| m.as_str())
                        {
                            agent.model = Some(model.to_string());
                        }
                    }
                } else {
                    primary.last_activity = Some(p.meta.timestamp.clone());
                }
            }
            _ => {}
        }
    }

    let mut result = vec![primary];
    let mut subagents: Vec<AgentState> = agents.into_values().collect();
    subagents.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
    result.extend(subagents);
    result
}

/// Extract task states from progress entries containing TaskCreate/TaskUpdate tool uses.
///
/// Tasks are discovered by scanning progress entry `data` fields for nested
/// content blocks with `name: "TaskCreate"` or `name: "TaskUpdate"`.
pub(crate) fn extract_tasks(entries: &[ParsedEntry]) -> Vec<TaskState> {
    let mut tasks: Vec<TaskState> = Vec::new();
    let mut task_index: usize = 0;

    for parsed in entries {
        let SessionEntry::Progress(progress) = &parsed.entry else {
            continue;
        };
        let Some(data) = &progress.data else {
            continue;
        };

        let agent_id = data.get("agentId").and_then(|a| a.as_str());

        // Navigate: data.message.message.content[].{name, input}
        let Some(blocks) = data
            .pointer("/message/message/content")
            .and_then(|c| c.as_array())
        else {
            continue;
        };

        for block in blocks {
            let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let input = block.get("input");

            match name {
                "TaskCreate" => {
                    if let Some(input) = input {
                        task_index += 1;
                        let subject = input
                            .get("subject")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let description = input
                            .get("description")
                            .and_then(|s| s.as_str())
                            .map(String::from);

                        tasks.push(TaskState {
                            task_id: task_index.to_string(),
                            subject,
                            description,
                            status: TaskStatus::Pending,
                            assigned_agent: agent_id.map(String::from),
                            last_update: Some(progress.meta.timestamp.clone()),
                        });
                    }
                }
                "TaskUpdate" => {
                    if let Some(input) = input {
                        let task_id_str =
                            input.get("taskId").and_then(|s| s.as_str()).unwrap_or("");
                        let status_str = input.get("status").and_then(|s| s.as_str()).unwrap_or("");

                        let status = match status_str {
                            "in_progress" => TaskStatus::InProgress,
                            "completed" => TaskStatus::Completed,
                            "cancelled" => TaskStatus::Cancelled,
                            _ => TaskStatus::Pending,
                        };

                        // Find and update existing task, or create a placeholder
                        if let Some(task) = tasks.iter_mut().find(|t| t.task_id == task_id_str) {
                            task.status = status;
                            task.last_update = Some(progress.meta.timestamp.clone());
                            if agent_id.is_some() {
                                task.assigned_agent = agent_id.map(String::from);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    tasks
}

/// Extract compaction information from system entries.
///
/// Returns `(last_compaction_timestamp, compaction_trigger)` from the most
/// recent compact boundary entry.
pub(crate) fn extract_compaction(entries: &[ParsedEntry]) -> (Option<String>, Option<String>) {
    let mut last_compaction: Option<String> = None;
    let mut trigger: Option<String> = None;

    for parsed in entries {
        let SessionEntry::System(system) = &parsed.entry else {
            continue;
        };
        let Some(data) = &system.data else {
            continue;
        };

        // Check for compact_boundary subtype or type
        let is_compact = data
            .get("subtype")
            .and_then(|s| s.as_str())
            .is_some_and(|s| s == "compact_boundary")
            || data
                .get("type")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "compact_boundary");

        if is_compact {
            last_compaction = Some(system.meta.timestamp.clone());
            trigger = data
                .pointer("/compactMetadata/trigger")
                .and_then(|t| t.as_str())
                .map(String::from);
        }
    }

    (last_compaction, trigger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::context::{
        AssistantEntry, AssistantMessage, EntryMetadata, ProgressEntry, SystemEntry,
    };

    fn make_meta(is_sidechain: bool, timestamp: &str, cwd: Option<&str>) -> EntryMetadata {
        EntryMetadata {
            uuid: "test-uuid".into(),
            timestamp: timestamp.into(),
            session_id: "s1".into(),
            parent_uuid: None,
            is_sidechain,
            cwd: cwd.map(String::from),
            version: None,
        }
    }

    fn make_parsed(entry: SessionEntry, line: usize) -> ParsedEntry {
        ParsedEntry {
            entry,
            line_number: line,
            raw_bytes: 100,
            raw_line: String::new(),
        }
    }

    #[test]
    fn extract_agents_primary_only() {
        let entries = vec![make_parsed(
            SessionEntry::Assistant(AssistantEntry {
                meta: make_meta(false, "2026-03-06T10:00:00Z", Some("/project")),
                message: Some(AssistantMessage {
                    model: Some("claude-opus-4-6".into()),
                    content: vec![],
                    usage: None,
                    stop_reason: None,
                }),
            }),
            1,
        )];

        let agents = extract_agents(&entries);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "primary");
        assert_eq!(agents[0].model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(agents[0].status, AgentStatus::Active);
        assert!(!agents[0].is_sidechain);
        assert_eq!(agents[0].working_dir.as_deref(), Some("/project"));
    }

    #[test]
    fn extract_agents_with_subagents() {
        let entries = vec![
            make_parsed(
                SessionEntry::Assistant(AssistantEntry {
                    meta: make_meta(false, "2026-03-06T10:00:00Z", None),
                    message: Some(AssistantMessage {
                        model: Some("claude-opus-4-6".into()),
                        content: vec![],
                        usage: None,
                        stop_reason: None,
                    }),
                }),
                1,
            ),
            make_parsed(
                SessionEntry::Progress(ProgressEntry {
                    meta: make_meta(true, "2026-03-06T10:01:00Z", Some("/project")),
                    data: Some(serde_json::json!({
                        "agentId": "agent-abc123",
                        "message": {
                            "message": {
                                "model": "claude-opus-4-6",
                                "content": []
                            }
                        }
                    })),
                }),
                2,
            ),
            make_parsed(
                SessionEntry::Progress(ProgressEntry {
                    meta: make_meta(true, "2026-03-06T10:02:00Z", None),
                    data: Some(serde_json::json!({
                        "agentId": "agent-def456",
                    })),
                }),
                3,
            ),
        ];

        let agents = extract_agents(&entries);
        assert_eq!(agents.len(), 3);
        assert_eq!(agents[0].agent_id, "primary");
        // Subagents sorted by ID
        assert_eq!(agents[1].agent_id, "agent-abc123");
        assert!(agents[1].is_sidechain);
        assert_eq!(agents[1].model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(agents[2].agent_id, "agent-def456");
        assert!(agents[2].is_sidechain);
    }

    #[test]
    fn extract_tasks_from_progress_entries() {
        let entries = vec![
            make_parsed(
                SessionEntry::Progress(ProgressEntry {
                    meta: make_meta(true, "2026-03-06T10:00:00Z", None),
                    data: Some(serde_json::json!({
                        "agentId": "agent-abc",
                        "message": {
                            "message": {
                                "content": [
                                    {
                                        "type": "tool_use",
                                        "id": "tu1",
                                        "name": "TaskCreate",
                                        "input": {
                                            "subject": "Implement auth flow",
                                            "description": "Add JWT authentication"
                                        }
                                    }
                                ]
                            }
                        }
                    })),
                }),
                1,
            ),
            make_parsed(
                SessionEntry::Progress(ProgressEntry {
                    meta: make_meta(true, "2026-03-06T10:01:00Z", None),
                    data: Some(serde_json::json!({
                        "agentId": "agent-abc",
                        "message": {
                            "message": {
                                "content": [
                                    {
                                        "type": "tool_use",
                                        "id": "tu2",
                                        "name": "TaskUpdate",
                                        "input": {
                                            "taskId": "1",
                                            "status": "in_progress"
                                        }
                                    }
                                ]
                            }
                        }
                    })),
                }),
                2,
            ),
        ];

        let tasks = extract_tasks(&entries);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_id, "1");
        assert_eq!(tasks[0].subject, "Implement auth flow");
        assert_eq!(
            tasks[0].description.as_deref(),
            Some("Add JWT authentication")
        );
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
        assert_eq!(tasks[0].assigned_agent.as_deref(), Some("agent-abc"));
    }

    #[test]
    fn extract_tasks_multiple_creates() {
        let entries = vec![make_parsed(
            SessionEntry::Progress(ProgressEntry {
                meta: make_meta(true, "2026-03-06T10:00:00Z", None),
                data: Some(serde_json::json!({
                    "agentId": "agent-abc",
                    "message": {
                        "message": {
                            "content": [
                                {
                                    "type": "tool_use",
                                    "id": "tu1",
                                    "name": "TaskCreate",
                                    "input": { "subject": "Task A" }
                                },
                                {
                                    "type": "tool_use",
                                    "id": "tu2",
                                    "name": "TaskCreate",
                                    "input": { "subject": "Task B" }
                                }
                            ]
                        }
                    }
                })),
            }),
            1,
        )];

        let tasks = extract_tasks(&entries);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].task_id, "1");
        assert_eq!(tasks[0].subject, "Task A");
        assert_eq!(tasks[1].task_id, "2");
        assert_eq!(tasks[1].subject, "Task B");
    }

    #[test]
    fn extract_compaction_from_system_entries() {
        let entries = vec![
            make_parsed(
                SessionEntry::System(SystemEntry {
                    meta: make_meta(false, "2026-03-06T09:00:00Z", None),
                    data: Some(serde_json::json!({
                        "subtype": "compact_boundary",
                        "compactMetadata": {
                            "trigger": "auto",
                            "preTokens": 168576
                        }
                    })),
                }),
                1,
            ),
            make_parsed(
                SessionEntry::System(SystemEntry {
                    meta: make_meta(false, "2026-03-06T10:00:00Z", None),
                    data: Some(serde_json::json!({
                        "subtype": "compact_boundary",
                        "compactMetadata": {
                            "trigger": "manual",
                            "preTokens": 195000
                        }
                    })),
                }),
                2,
            ),
        ];

        let (ts, trigger) = extract_compaction(&entries);
        // Should return the latest compaction
        assert_eq!(ts.as_deref(), Some("2026-03-06T10:00:00Z"));
        assert_eq!(trigger.as_deref(), Some("manual"));
    }

    #[test]
    fn extract_compaction_none_when_no_compact_entries() {
        let entries = vec![make_parsed(
            SessionEntry::System(SystemEntry {
                meta: make_meta(false, "2026-03-06T10:00:00Z", None),
                data: Some(serde_json::json!({
                    "subtype": "turn_duration",
                    "duration_ms": 5000
                })),
            }),
            1,
        )];

        let (ts, trigger) = extract_compaction(&entries);
        assert!(ts.is_none());
        assert!(trigger.is_none());
    }

    #[test]
    fn solo_agent_produces_valid_checkpoint() {
        // Only primary agent entries, no sidechain
        let entries = vec![make_parsed(
            SessionEntry::Assistant(AssistantEntry {
                meta: make_meta(false, "2026-03-06T10:00:00Z", Some("/project")),
                message: Some(AssistantMessage {
                    model: Some("claude-opus-4-6".into()),
                    content: vec![],
                    usage: None,
                    stop_reason: None,
                }),
            }),
            1,
        )];

        let agents = extract_agents(&entries);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_id, "primary");
        assert!(!agents[0].is_sidechain);
        assert_eq!(agents[0].status, AgentStatus::Active);

        let tasks = extract_tasks(&entries);
        assert!(tasks.is_empty());
    }
}
