//! MCP server configuration panel — data model and JSON I/O.
//!
//! Manages `.assay/mcp.json`, which stores MCP server entries in the format:
//! ```json
//! { "mcpServers": { "name": { "command": "cmd", "args": ["a","b"] } } }
//! ```

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

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
pub struct AddServerForm {
    pub name: String,
    pub command: String,
    pub active_field: usize,
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
/// Returns an empty vec when the file is missing. Entries are sorted
/// alphabetically by name for stable display order.
pub fn mcp_config_load(root: &Path) -> Vec<McpServerEntry> {
    let path = root.join(".assay").join("mcp.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let config: McpConfigFile = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
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
    entries
}

/// Atomically save MCP server entries to `.assay/mcp.json` under `root`.
///
/// Uses the NamedTempFile-write-sync-persist pattern (D093) to prevent
/// partial writes. The `.assay/` directory must already exist.
pub fn mcp_config_save(root: &Path, servers: &[McpServerEntry]) -> Result<(), String> {
    let assay_dir = root.join(".assay");
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
        let loaded = mcp_config_load(root);

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
        let loaded = mcp_config_load(tmp.path());
        assert!(loaded.is_empty());
    }
}
