//! MCP server configuration panel — data model and JSON I/O.
//!
//! Manages `.assay/mcp.json`, which stores MCP server entries in the format:
//! ```json
//! { "mcpServers": { "name": { "command": "cmd", "args": ["a","b"] } } }
//! ```

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

// ── Public types ──────────────────────────────────────────────────────────────

/// A single MCP server entry, ready for display and persistence.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

/// State for the inline "add server" form.
///
/// `active_field`: 0 = name, 1 = command.
#[derive(Default)]
pub struct AddServerForm {
    pub name: String,
    pub command: String,
    pub active_field: usize,
}

impl AddServerForm {
    /// Create a new empty add-server form with focus on the name field.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            active_field: 0,
        }
    }
}

// ── Internal serde model ──────────────────────────────────────────────────────

/// Mirrors the on-disk `{ "mcpServers": { ... } }` JSON shape.
#[derive(Serialize, Deserialize, Debug)]
struct McpConfigFile {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: HashMap<String, McpServerValue>,
}

/// Value side of one entry inside `mcpServers`.
#[derive(Serialize, Deserialize, Debug)]
struct McpServerValue {
    command: String,
    #[serde(default)]
    args: Vec<String>,
}

// ── JSON I/O ──────────────────────────────────────────────────────────────────

/// Load MCP server entries from `.assay/mcp.json` under `root`.
///
/// Returns `Ok(empty vec)` when the file does not exist.
/// Returns `Err` when the file exists but is unreadable or contains invalid JSON.
/// Entries are sorted alphabetically by name for stable display order.
pub fn mcp_config_load(root: &Path) -> Result<Vec<McpServerEntry>, String> {
    let path = root.join(".assay").join("mcp.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Failed to read {}: {e}", path.display())),
    };
    let config: McpConfigFile = serde_json::from_str(&content)
        .map_err(|e| format!("{} contains invalid JSON: {e}", path.display()))?;
    let mut entries: Vec<McpServerEntry> = config
        .mcp_servers
        .into_iter()
        .map(|(name, val)| McpServerEntry {
            name,
            command: val.command,
            args: val.args,
        })
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Atomically save MCP server entries to `.assay/mcp.json` under `root`.
///
/// Uses the NamedTempFile-write-sync-persist pattern (D093) to prevent
/// partial writes. Returns `Err` if `.assay/` does not exist.
pub fn mcp_config_save(root: &Path, servers: &[McpServerEntry]) -> Result<(), String> {
    let assay_dir = root.join(".assay");
    if !assay_dir.exists() {
        return Err(format!(
            "Project directory {} not found. Run `assay init` to create it.",
            assay_dir.display()
        ));
    }
    let path = assay_dir.join("mcp.json");

    let mut map = HashMap::new();
    for entry in servers {
        map.insert(
            entry.name.clone(),
            McpServerValue {
                command: entry.command.clone(),
                args: entry.args.clone(),
            },
        );
    }
    let config = McpConfigFile { mcp_servers: map };

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("failed to serialize mcp.json: {e}"))?;

    let mut tmpfile = NamedTempFile::new_in(&assay_dir)
        .map_err(|e| format!("failed to create temp file for mcp.json: {e}"))?;

    tmpfile
        .write_all(content.as_bytes())
        .map_err(|e| format!("failed to write mcp.json: {e}"))?;

    tmpfile
        .as_file()
        .sync_all()
        .map_err(|e| format!("failed to sync mcp.json: {e}"))?;

    tmpfile
        .persist(&path)
        .map_err(|e| format!("failed to persist mcp.json: {e}"))?;

    Ok(())
}

// ── Drawing ───────────────────────────────────────────────────────────────────

/// Render the MCP server configuration panel.
///
/// Shows a server list with selection highlight, an optional add-form popup,
/// an optional inline error, and a hint bar at the bottom.
pub fn draw_mcp_panel(
    frame: &mut ratatui::Frame,
    area: Rect,
    servers: &[McpServerEntry],
    selected: usize,
    add_form: Option<&AddServerForm>,
    error: Option<&str>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" MCP Server Configuration ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: list fills, optional error row, hint row.
    let error_height = if error.is_some() { 1 } else { 0 };
    let [list_area, error_area, hint_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(error_height),
        Constraint::Length(1),
    ])
    .areas(inner);

    // ── Server list ──────────────────────────────────────────────────────
    if servers.is_empty() {
        let msg = Paragraph::new(Line::from("No servers configured.").dim());
        frame.render_widget(msg, list_area);
    } else {
        let items: Vec<ListItem> = servers
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let prefix = if i == selected { "▶ " } else { "  " };
                let text = format!("{prefix}{}  ({})", entry.name, entry.command);
                let style = if i == selected {
                    Style::default().bold().reversed()
                } else {
                    Style::default()
                };
                ListItem::new(text).style(style)
            })
            .collect();
        let list = List::new(items);
        frame.render_widget(list, list_area);
    }

    // ── Error line ───────────────────────────────────────────────────────
    if let Some(msg) = error {
        let err_line = Paragraph::new(Line::from(Span::styled(
            format!("Error: {msg}"),
            Style::default().fg(Color::Red),
        )));
        frame.render_widget(err_line, error_area);
    }

    // ── Hint bar ─────────────────────────────────────────────────────────
    let hint_text = "a:add  d:delete  w:save  Esc:back";
    let hint = Paragraph::new(Line::from(hint_text).dim());
    frame.render_widget(hint, hint_area);

    // ── Add-form overlay ─────────────────────────────────────────────────
    if let Some(form) = add_form {
        let popup_w = area.width.min(50);
        let popup_h = 8;
        let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
        let popup = Rect::new(x, y, popup_w, popup_h);

        frame.render_widget(Clear, popup);

        let form_block = Block::default().borders(Borders::ALL).title(" Add Server ");
        let form_inner = form_block.inner(popup);
        frame.render_widget(form_block, popup);

        let [name_area, cmd_area, form_hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(form_inner);

        let name_style = if form.active_field == 0 {
            Style::default().bold().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let cmd_style = if form.active_field == 1 {
            Style::default().bold().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let name_line = Line::from(vec![
            Span::styled("Name:    ", Style::default().dim()),
            Span::styled(&form.name, name_style),
            if form.active_field == 0 {
                Span::styled("█", name_style)
            } else {
                Span::raw("")
            },
        ]);
        let cmd_line = Line::from(vec![
            Span::styled("Command: ", Style::default().dim()),
            Span::styled(&form.command, cmd_style),
            if form.active_field == 1 {
                Span::styled("█", cmd_style)
            } else {
                Span::raw("")
            },
        ]);

        frame.render_widget(Paragraph::new(name_line), name_area);
        frame.render_widget(Paragraph::new(cmd_line), cmd_area);

        let form_hint = Paragraph::new(Line::from("Tab:switch  Enter:confirm  Esc:cancel").dim());
        frame.render_widget(form_hint, form_hint_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trip_json() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".assay")).unwrap();

        let servers = vec![
            McpServerEntry {
                name: "bravo".into(),
                command: "npx".into(),
                args: vec!["-y".into(), "bravo-server".into()],
            },
            McpServerEntry {
                name: "alpha".into(),
                command: "node".into(),
                args: vec![],
            },
        ];

        mcp_config_save(root, &servers).unwrap();
        let loaded = mcp_config_load(root).unwrap();

        // Loaded entries are sorted alphabetically.
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "alpha");
        assert_eq!(loaded[0].command, "node");
        assert_eq!(loaded[1].name, "bravo");
        assert_eq!(loaded[1].command, "npx");
        assert_eq!(loaded[1].args, vec!["-y", "bravo-server"]);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let loaded = mcp_config_load(tmp.path()).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn load_corrupt_json_returns_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".assay")).unwrap();
        std::fs::write(root.join(".assay").join("mcp.json"), "not valid json").unwrap();
        let result = mcp_config_load(root);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid JSON"));
    }
}
