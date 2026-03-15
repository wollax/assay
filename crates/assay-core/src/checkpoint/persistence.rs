//! Checkpoint file persistence: save, load, list, and prune.
//!
//! Checkpoints are stored as JSON frontmatter + markdown files in
//! `.assay/checkpoints/`. The `latest.md` file is always overwritten;
//! timestamped copies are archived in `archive/`.

use std::io::Write;
use std::path::{Path, PathBuf};

use assay_types::checkpoint::TeamCheckpoint;

use crate::AssayError;

/// Maximum number of archived checkpoint files to retain.
const MAX_ARCHIVE_ENTRIES: usize = 50;

/// Summary of an archived checkpoint file.
#[derive(Debug, Clone)]
pub struct CheckpointEntry {
    /// Path to the archived checkpoint file.
    pub path: PathBuf,
    /// ISO 8601 timestamp from the checkpoint.
    pub timestamp: String,
    /// What triggered this checkpoint.
    pub trigger: String,
    /// Number of agents in the checkpoint.
    pub agent_count: usize,
    /// Number of tasks in the checkpoint.
    pub task_count: usize,
}

/// Save a checkpoint to `.assay/checkpoints/latest.md` and `archive/{timestamp}.md`.
///
/// Writes are atomic (tempfile-then-rename). Archives are pruned to keep
/// at most 50 entries.
///
/// Returns the path to the archived copy.
pub fn save_checkpoint(assay_dir: &Path, checkpoint: &TeamCheckpoint) -> crate::Result<PathBuf> {
    let checkpoints_dir = assay_dir.join("checkpoints");
    let archive_dir = checkpoints_dir.join("archive");

    std::fs::create_dir_all(&archive_dir).map_err(|e| AssayError::CheckpointWrite {
        path: archive_dir.clone(),
        message: format!("creating directories: {e}"),
    })?;

    let content = render_checkpoint(checkpoint);

    // Write latest.md atomically
    let latest_path = checkpoints_dir.join("latest.md");
    atomic_write(&latest_path, &content)?;

    // Write archive copy with filesystem-safe timestamp
    let safe_ts = checkpoint.timestamp.replace(':', "-");
    let archive_filename = format!("{safe_ts}.md");
    let archive_path = archive_dir.join(&archive_filename);
    atomic_write(&archive_path, &content)?;

    // Prune archive
    prune_archive(&archive_dir)?;

    // Update last-checkpoint timestamp file
    let ts_path = checkpoints_dir.join(".last-checkpoint-ts");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Err(e) = std::fs::write(&ts_path, now.to_string()) {
        tracing::warn!(path = %ts_path.display(), "failed to write last-checkpoint-ts: {e}");
    }

    Ok(archive_path)
}

/// Load the latest checkpoint from `.assay/checkpoints/latest.md`.
pub fn load_latest_checkpoint(assay_dir: &Path) -> crate::Result<TeamCheckpoint> {
    let latest_path = assay_dir.join("checkpoints").join("latest.md");

    let content =
        std::fs::read_to_string(&latest_path).map_err(|e| AssayError::CheckpointRead {
            path: latest_path.clone(),
            message: format!("reading file: {e}"),
        })?;

    parse_frontmatter(&latest_path, &content)
}

/// List archived checkpoints, sorted newest first.
pub fn list_checkpoints(assay_dir: &Path, limit: usize) -> crate::Result<Vec<CheckpointEntry>> {
    let archive_dir = assay_dir.join("checkpoints").join("archive");

    if !archive_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(&archive_dir)
        .map_err(|e| AssayError::CheckpointRead {
            path: archive_dir.clone(),
            message: format!("reading archive directory: {e}"),
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| e.path())
        .collect();

    // Sort by filename descending (newest first, since filenames are timestamps)
    files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    let mut entries = Vec::new();
    for path in files.into_iter().take(limit) {
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(checkpoint) = parse_frontmatter(&path, &content)
        {
            entries.push(CheckpointEntry {
                path: path.clone(),
                timestamp: checkpoint.timestamp,
                trigger: checkpoint.trigger,
                agent_count: checkpoint.agents.len(),
                task_count: checkpoint.tasks.len(),
            });
        }
    }

    Ok(entries)
}

/// Render a checkpoint as JSON frontmatter + markdown body.
pub(crate) fn render_checkpoint(checkpoint: &TeamCheckpoint) -> String {
    let json = serde_json::to_string_pretty(checkpoint).unwrap_or_default();

    let mut body = String::new();

    // Frontmatter
    body.push_str("---\n");
    body.push_str(&json);
    body.push('\n');
    body.push_str("---\n\n");

    // Markdown body
    let short_session = if checkpoint.session_id.len() > 8 {
        &checkpoint.session_id[..8]
    } else {
        &checkpoint.session_id
    };
    let project_name = checkpoint
        .project
        .rsplit('/')
        .next()
        .unwrap_or(&checkpoint.project);

    body.push_str("# Team Checkpoint\n\n");
    body.push_str(&format!("**Session:** {short_session}\n"));
    body.push_str(&format!("**Project:** {project_name}\n"));
    body.push_str(&format!("**Captured:** {}\n", checkpoint.timestamp));
    body.push_str(&format!("**Trigger:** {}\n\n", checkpoint.trigger));

    // Agents table
    body.push_str("## Agents\n\n");
    body.push_str("| Agent | Model | Status | Current Task | Working Dir |\n");
    body.push_str("|-------|-------|--------|-------------|-------------|\n");
    for agent in &checkpoint.agents {
        let model = agent.model.as_deref().unwrap_or("-");
        let status = format!("{:?}", agent.status).to_lowercase();
        let task = agent.current_task.as_deref().unwrap_or("-");
        let wd = agent.working_dir.as_deref().unwrap_or("-");
        body.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            agent.agent_id, model, status, task, wd
        ));
    }
    body.push('\n');

    // Tasks table
    if !checkpoint.tasks.is_empty() {
        body.push_str("## Tasks\n\n");
        body.push_str("| ID | Subject | Status | Assigned Agent | Last Update |\n");
        body.push_str("|----|---------|--------|----------------|-------------|\n");
        for task in &checkpoint.tasks {
            let status = format!("{:?}", task.status).to_lowercase();
            let agent = task.assigned_agent.as_deref().unwrap_or("-");
            let update = task.last_update.as_deref().unwrap_or("-");
            body.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                task.task_id, task.subject, status, agent, update
            ));
        }
        body.push('\n');
    }

    // Context health
    if let Some(health) = &checkpoint.context_health {
        body.push_str("## Context Health\n\n");
        body.push_str(&format!(
            "- **Context tokens:** {} / {} ({:.1}%)\n",
            health.context_tokens, health.context_window, health.utilization_pct
        ));
        if let Some(ts) = &health.last_compaction {
            let trigger = health.compaction_trigger.as_deref().unwrap_or("unknown");
            body.push_str(&format!("- **Last compaction:** {ts} ({trigger})\n"));
        }
        body.push('\n');
    }

    body
}

/// Parse JSON frontmatter from a checkpoint file.
fn parse_frontmatter(path: &Path, content: &str) -> crate::Result<TeamCheckpoint> {
    // Find frontmatter between --- delimiters
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(AssayError::CheckpointRead {
            path: path.to_path_buf(),
            message: "missing frontmatter delimiter".into(),
        });
    }

    // Skip first ---
    let after_first = &trimmed[3..].trim_start_matches('\n');
    let end_pos = after_first
        .find("\n---")
        .ok_or_else(|| AssayError::CheckpointRead {
            path: path.to_path_buf(),
            message: "missing closing frontmatter delimiter".into(),
        })?;

    let frontmatter = &after_first[..end_pos];

    serde_json::from_str(frontmatter).map_err(|e| AssayError::CheckpointRead {
        path: path.to_path_buf(),
        message: format!("parsing frontmatter JSON: {e}"),
    })
}

/// Write content to a file atomically via tempfile-then-rename.
fn atomic_write(path: &Path, content: &str) -> crate::Result<()> {
    let dir = path.parent().ok_or_else(|| AssayError::CheckpointWrite {
        path: path.to_path_buf(),
        message: "no parent directory".into(),
    })?;

    let mut tmpfile =
        tempfile::NamedTempFile::new_in(dir).map_err(|e| AssayError::CheckpointWrite {
            path: path.to_path_buf(),
            message: format!("creating temp file: {e}"),
        })?;

    tmpfile
        .write_all(content.as_bytes())
        .map_err(|e| AssayError::CheckpointWrite {
            path: path.to_path_buf(),
            message: format!("writing content: {e}"),
        })?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| AssayError::CheckpointWrite {
            path: path.to_path_buf(),
            message: format!("syncing file: {e}"),
        })?;

    tmpfile
        .persist(path)
        .map_err(|e| AssayError::CheckpointWrite {
            path: path.to_path_buf(),
            message: format!("persisting file: {e}"),
        })?;

    Ok(())
}

/// Prune archive directory to keep at most MAX_ARCHIVE_ENTRIES files.
fn prune_archive(archive_dir: &Path) -> crate::Result<()> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(archive_dir)
        .map_err(|e| AssayError::CheckpointWrite {
            path: archive_dir.to_path_buf(),
            message: format!("reading archive for pruning: {e}"),
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| e.path())
        .collect();

    if files.len() <= MAX_ARCHIVE_ENTRIES {
        return Ok(());
    }

    // Sort ascending by filename (oldest first)
    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let to_remove = files.len() - MAX_ARCHIVE_ENTRIES;
    for path in files.into_iter().take(to_remove) {
        let _ = std::fs::remove_file(&path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::checkpoint::{
        AgentState, AgentStatus, ContextHealthSnapshot, TaskState, TaskStatus,
    };

    fn make_checkpoint() -> TeamCheckpoint {
        TeamCheckpoint {
            version: 1,
            session_id: "0509db4c-b52e-456b-b6f3-8e5578ee608f".into(),
            project: "/Users/dev/project".into(),
            timestamp: "2026-03-06T10:00:00Z".into(),
            trigger: "manual".into(),
            agents: vec![
                AgentState {
                    agent_id: "primary".into(),
                    model: Some("claude-opus-4-6".into()),
                    status: AgentStatus::Active,
                    current_task: Some("Implement auth".into()),
                    working_dir: Some("/Users/dev/project".into()),
                    is_sidechain: false,
                    last_activity: Some("2026-03-06T10:00:00Z".into()),
                },
                AgentState {
                    agent_id: "agent-abc123".into(),
                    model: Some("claude-opus-4-6".into()),
                    status: AgentStatus::Active,
                    current_task: None,
                    working_dir: None,
                    is_sidechain: true,
                    last_activity: Some("2026-03-06T09:59:00Z".into()),
                },
            ],
            tasks: vec![TaskState {
                task_id: "1".into(),
                subject: "Add auth flow".into(),
                description: Some("JWT authentication".into()),
                status: TaskStatus::InProgress,
                assigned_agent: Some("agent-abc123".into()),
                last_update: Some("2026-03-06T09:58:00Z".into()),
            }],
            context_health: Some(ContextHealthSnapshot {
                context_tokens: 168_576,
                context_window: 200_000,
                utilization_pct: 84.3,
                last_compaction: Some("2026-03-06T09:30:00Z".into()),
                compaction_trigger: Some("auto".into()),
            }),
        }
    }

    #[test]
    fn render_checkpoint_has_frontmatter_delimiters() {
        let checkpoint = make_checkpoint();
        let rendered = render_checkpoint(&checkpoint);

        assert!(rendered.starts_with("---\n"));
        assert!(rendered.contains("\n---\n"));
    }

    #[test]
    fn render_checkpoint_frontmatter_is_valid_json() {
        let checkpoint = make_checkpoint();
        let rendered = render_checkpoint(&checkpoint);

        // Extract frontmatter
        let after_first = rendered.strip_prefix("---\n").unwrap();
        let end = after_first.find("\n---").unwrap();
        let frontmatter = &after_first[..end];

        let parsed: TeamCheckpoint = serde_json::from_str(frontmatter).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.session_id, checkpoint.session_id);
        assert_eq!(parsed.agents.len(), 2);
        assert_eq!(parsed.tasks.len(), 1);
    }

    #[test]
    fn render_checkpoint_includes_markdown_body() {
        let checkpoint = make_checkpoint();
        let rendered = render_checkpoint(&checkpoint);

        assert!(rendered.contains("# Team Checkpoint"));
        assert!(rendered.contains("## Agents"));
        assert!(rendered.contains("## Tasks"));
        assert!(rendered.contains("## Context Health"));
        assert!(rendered.contains("primary"));
        assert!(rendered.contains("agent-abc123"));
        assert!(rendered.contains("Add auth flow"));
        assert!(rendered.contains("168576"));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let checkpoint = make_checkpoint();

        let archive_path = save_checkpoint(dir.path(), &checkpoint).unwrap();
        assert!(archive_path.exists());
        assert!(dir.path().join("checkpoints/latest.md").exists());

        let loaded = load_latest_checkpoint(dir.path()).unwrap();
        assert_eq!(loaded.version, checkpoint.version);
        assert_eq!(loaded.session_id, checkpoint.session_id);
        assert_eq!(loaded.project, checkpoint.project);
        assert_eq!(loaded.trigger, checkpoint.trigger);
        assert_eq!(loaded.agents.len(), checkpoint.agents.len());
        assert_eq!(loaded.tasks.len(), checkpoint.tasks.len());
        assert_eq!(loaded.agents[0].agent_id, "primary");
        assert_eq!(loaded.tasks[0].subject, "Add auth flow");
    }

    #[test]
    fn save_creates_timestamp_file() {
        let dir = tempfile::tempdir().unwrap();
        let checkpoint = make_checkpoint();

        save_checkpoint(dir.path(), &checkpoint).unwrap();

        let ts_path = dir.path().join("checkpoints/.last-checkpoint-ts");
        assert!(ts_path.exists());
        let ts_content = std::fs::read_to_string(&ts_path).unwrap();
        assert!(ts_content.parse::<u64>().is_ok());
    }

    #[test]
    fn archive_pruning_enforces_limit() {
        let dir = tempfile::tempdir().unwrap();
        let archive_dir = dir.path().join("checkpoints").join("archive");
        std::fs::create_dir_all(&archive_dir).unwrap();

        // Create 55 archive files
        for i in 0..55 {
            let filename = format!("2026-03-06T{:02}-00-00Z.md", i);
            let path = archive_dir.join(&filename);
            std::fs::write(&path, "---\n{}\n---\n").unwrap();
        }

        // Now save a checkpoint, which triggers pruning
        let mut checkpoint = make_checkpoint();
        checkpoint.timestamp = "2026-03-06T22:00:00Z".to_string();
        save_checkpoint(dir.path(), &checkpoint).unwrap();

        // Count remaining .md files in archive
        let count = std::fs::read_dir(&archive_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .count();

        // Should be at most 50 (55 existing - 6 pruned + 1 new = 50)
        assert!(
            count <= MAX_ARCHIVE_ENTRIES,
            "Expected at most {MAX_ARCHIVE_ENTRIES} files, got {count}"
        );
    }

    #[test]
    fn list_checkpoints_returns_newest_first() {
        let dir = tempfile::tempdir().unwrap();

        // Save two checkpoints with different timestamps
        let mut cp1 = make_checkpoint();
        cp1.timestamp = "2026-03-06T08:00:00Z".to_string();
        save_checkpoint(dir.path(), &cp1).unwrap();

        let mut cp2 = make_checkpoint();
        cp2.timestamp = "2026-03-06T09:00:00Z".to_string();
        save_checkpoint(dir.path(), &cp2).unwrap();

        let entries = list_checkpoints(dir.path(), 10).unwrap();
        assert_eq!(entries.len(), 2);
        // Newest first
        assert!(entries[0].timestamp > entries[1].timestamp);
    }

    #[test]
    fn list_checkpoints_respects_limit() {
        let dir = tempfile::tempdir().unwrap();

        for i in 0..5 {
            let mut cp = make_checkpoint();
            cp.timestamp = format!("2026-03-06T{:02}:00:00Z", i);
            save_checkpoint(dir.path(), &cp).unwrap();
        }

        let entries = list_checkpoints(dir.path(), 3).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn list_checkpoints_empty_when_no_archive() {
        let dir = tempfile::tempdir().unwrap();
        let entries = list_checkpoints(dir.path(), 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn load_latest_errors_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_latest_checkpoint(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn parse_frontmatter_rejects_missing_delimiters() {
        let path = PathBuf::from("/test.md");

        let result = parse_frontmatter(&path, "no frontmatter here");
        assert!(result.is_err());

        let result = parse_frontmatter(&path, "---\n{\"version\":1}\nno closing delimiter");
        assert!(result.is_err());
    }
}
