# S04: MCP Server Configuration Panel — UAT

**Milestone:** M007
**Written:** 2026-03-23

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The panel's rendering, keyboard UX, and persistence behavior can only be fully verified by a human interacting with the real TUI in a terminal

## Preconditions

- `cargo build -p assay-tui` succeeds
- An Assay project exists (`.assay/` directory present)
- Terminal is at least 80×24

## Smoke Test

Run `cargo run -p assay-tui` from an Assay project directory. Press `m`. The MCP panel should open showing "No servers configured." or any existing servers from `.assay/mcp.json`. Press `Esc` to return to Dashboard.

## Test Cases

### 1. Open MCP panel from Dashboard

1. Launch `assay-tui` from an Assay project root
2. Press `m`
3. **Expected:** Screen transitions to MCP Server Configuration panel with a bordered block, server list (or "No servers configured"), and hint bar showing "a:add  d:delete  w:save  Esc:back"

### 2. Add a server

1. From the MCP panel, press `a`
2. Type a server name (e.g., `my-mcp-server`)
3. Press `Tab` to switch to the command field
4. Type a command (e.g., `npx -y my-server`)
5. Press `Enter` to confirm
6. **Expected:** The server appears in the list, sorted alphabetically. The add-form closes.

### 3. Save servers to disk

1. After adding a server, press `w`
2. **Expected:** Screen returns to Dashboard. `.assay/mcp.json` exists on disk and contains the server entry in `{ "mcpServers": { "my-mcp-server": { "command": "npx -y my-server", "args": [] } } }` format.

### 4. Delete a server

1. Press `m` to reopen the panel (servers loaded from disk)
2. Use `Up`/`Down` to select a server
3. Press `d`
4. Press `w` to save
5. **Expected:** Server removed from list and from `.assay/mcp.json` on disk

### 5. Cancel without saving

1. Press `m` to open the panel
2. Press `a`, type a name, press `Enter` to add
3. Press `Esc` (instead of `w`)
4. Press `m` again
5. **Expected:** The previously added server is NOT in the list (changes were discarded)

## Edge Cases

### Duplicate server name

1. Add a server named "alpha"
2. Press `a` again, type "alpha", press `Enter`
3. **Expected:** Inline error "Duplicate server name: alpha" appears in red; server is NOT added

### Empty name

1. Press `a`, leave name empty, press `Enter`
2. **Expected:** Inline error "Server name cannot be empty." appears; form stays open

### No .assay/mcp.json on first open

1. Delete `.assay/mcp.json` if it exists
2. Press `m`
3. **Expected:** Panel opens with "No servers configured." — no error

## Failure Signals

- Panel shows a red "Error:" line after pressing `w` — indicates I/O failure
- Pressing `m` shows nothing or crashes — indicates Screen::McpPanel wiring is broken
- `.assay/mcp.json` is empty or malformed after save — indicates serialization bug

## Requirements Proved By This UAT

- R055 (TUI MCP server management) — this UAT proves the full add/delete/save workflow is usable by a human in a real terminal, keyboard UX is responsive, and persistence round-trips correctly

## Not Proven By This UAT

- Live MCP server connection/disconnection (deferred to M008+, per D110)
- MCP tool inspection or server health status
- Concurrent access to `.assay/mcp.json` from multiple processes

## Notes for Tester

- The add-form captures `command` as a single string — the `args` field is always an empty vec. For servers that need arguments, edit `.assay/mcp.json` directly after creating the entry.
- The panel does NOT connect to any MCP servers — it only manages the config file.
