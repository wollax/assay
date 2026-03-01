# Phase 2: MCP Spike - Research

**Researched:** 2026-02-28
**Domain:** MCP server implementation via rmcp (Rust SDK) with stdio transport
**Confidence:** HIGH

## Summary

This phase validates that rmcp 0.17 + stdio transport + Claude Code's MCP client exchange protocol work end-to-end. The research confirms that rmcp 0.17.0 provides a mature, well-documented macro-driven API for building MCP servers with stdio transport. The pattern is straightforward: define a struct with `#[tool_router]`, implement `ServerHandler` with `#[tool_handler]`, and serve via `stdio()` transport.

The critical integration surface is between the `assay` binary (running `assay mcp serve` as a stdio process) and Claude Code's MCP client. Claude Code discovers stdio MCP servers through `.mcp.json` (project scope) or `claude mcp add` (local scope), launches them as child processes, and communicates over stdin/stdout JSON-RPC.

**Primary recommendation:** Use rmcp's `#[tool_router]` + `#[tool_handler]` macros to build a minimal spike server in `assay-mcp`, expose it through a `Command::Mcp(McpCommand::Serve)` subcommand in `assay-cli`, and validate with Claude Code using project-scoped `.mcp.json`.

## Standard Stack

The established libraries/tools for this domain:

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rmcp | 0.17.0 | MCP server SDK | Official Rust SDK for Model Context Protocol; provides `#[tool_router]`, `#[tool_handler]`, `ServerHandler` trait, stdio transport |
| tokio | 1.49.0 | Async runtime | Required by rmcp; already a workspace dependency with `full` features |
| schemars | 1.x | JSON Schema generation | Required by rmcp `server` feature for tool parameter schemas; already a workspace dependency |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1.44 | Structured logging facade | MCP-06 requirement: all diagnostics must go to stderr, never stdout |
| tracing-subscriber | 0.3.22 | Logging subscriber with EnvFilter | Format and filter log output to stderr; `fmt` + `env-filter` features needed |

### Not Needed for Spike
| Library | Reason |
|---------|--------|
| serde / serde_json | Already transitive through rmcp; assay-mcp already lists them as deps |
| color-eyre | Not needed in MCP server path; errors returned as `McpError` / `ErrorData` |

**Installation (workspace Cargo.toml additions):**
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

**Crate dependency additions:**
- `assay-mcp`: add `tracing.workspace = true`, `tracing-subscriber.workspace = true`
- `assay-cli`: add `assay-mcp.workspace = true`, `tokio.workspace = true`

## Architecture Patterns

### Recommended Project Structure
```
crates/
├── assay-mcp/
│   └── src/
│       ├── lib.rs           # pub fn serve() entry point + re-exports
│       ├── spike.rs         # SpikeServer struct, #[tool_router], #[tool_handler]
│       └── logging.rs       # tracing-subscriber init to stderr
├── assay-cli/
│   └── src/
│       └── main.rs          # Cli { Command::Mcp(McpCommand::Serve) }
```

### Pattern 1: Tool Router + Tool Handler (rmcp canonical pattern)

**What:** Define a struct holding a `ToolRouter<Self>`, annotate tool methods with `#[tool]`, and use `#[tool_handler]` on the `ServerHandler` impl to auto-generate `list_tools` and `call_tool`.

**When to use:** Every MCP server built with rmcp.

**Example:**
```rust
// Source: Context7 rmcp docs + official counter example
use rmcp::{
    ErrorData as McpError,
    ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    model::*,
    tool, tool_router, tool_handler,
    transport::stdio,
};

#[derive(Clone)]
pub struct SpikeServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SpikeServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Spike: runs a hardcoded echo and returns output")]
    async fn spike_echo(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "spike: hello from assay",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for SpikeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Assay MCP spike server. Provides a single spike_echo tool for protocol validation."
                    .to_string(),
            ),
        }
    }
}
```

### Pattern 2: Async main with stdio transport

**What:** The server entry point initializes tracing to stderr, creates the service, and blocks on `waiting()`.

**When to use:** The `assay mcp serve` subcommand handler.

**Example:**
```rust
// Source: official counter_stdio.rs example
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    // MCP-06: tracing to stderr, never stdout
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting assay MCP server");

    let service = SpikeServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("MCP serve error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}
```

### Pattern 3: Clap nested subcommand for `assay mcp serve`

**What:** The CLI adds a `Mcp` variant to its top-level command enum, with a nested `McpCommand::Serve` subcommand that delegates to `assay_mcp::serve()`.

**When to use:** Wiring the MCP server into the single `assay` binary.

**Example:**
```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "assay", version, about = "Agentic development kit")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// MCP server operations
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
}

#[derive(Subcommand)]
enum McpCommand {
    /// Start the MCP server (stdio transport)
    Serve,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Mcp { command }) => match command {
            McpCommand::Serve => {
                if let Err(e) = assay_mcp::serve().await {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        },
        None => {
            println!("assay {}", env!("CARGO_PKG_VERSION"));
        }
    }
}
```

### Anti-Patterns to Avoid
- **Printing anything to stdout:** The JSON-RPC protocol owns stdout entirely. Any `println!`, debug output, or panic message on stdout will corrupt the protocol stream and cause Claude Code to disconnect.
- **Using `#[tokio::main]` on a library function:** The `#[tokio::main]` attribute belongs on `main()` in the binary crate. The library's `serve()` function should be a plain `async fn`.
- **Custom ServerHandler method implementations when macros suffice:** The `#[tool_handler]` macro auto-generates `list_tools` and `call_tool`. Don't hand-implement these.
- **Blocking in async tool handlers:** If a tool needs to run a synchronous operation (not needed for spike, but relevant for Phase 8), use `tokio::task::spawn_blocking`.

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON-RPC framing | Custom JSON-RPC parser/serializer | rmcp's transport layer | JSON-RPC 2.0 has subtle edge cases (batching, error codes); rmcp handles all of it |
| Tool schema generation | Manual JSON Schema | `schemars` derive + `#[tool]` macro | The macro generates input schemas from Rust types automatically |
| Protocol version negotiation | Manual handshake | `ServerHandler::initialize` default impl | rmcp's default implementation handles the MCP initialize handshake |
| Tool dispatch | Manual name matching | `ToolRouter` from `#[tool_router]` | The router handles tool name lookup, parameter deserialization, and error wrapping |
| Stderr logging setup | Custom stderr writer | `tracing_subscriber::fmt().with_writer(std::io::stderr)` | One line; handles buffering, formatting, filtering correctly |

**Key insight:** rmcp's macro system (`#[tool_router]` + `#[tool_handler]`) eliminates nearly all boilerplate. The spike server is ~50 lines of actual code.

## Common Pitfalls

### Pitfall 1: Stdout Pollution
**What goes wrong:** Any non-JSON-RPC bytes on stdout break the protocol. The MCP client receives invalid data, fails to parse, and disconnects.
**Why it happens:** `println!` calls, panic messages (default panic hook writes to stdout on some configs), `dbg!()` macro, or libraries that print to stdout.
**How to avoid:**
1. Initialize tracing to stderr before any other code runs
2. Use `tracing::info!` / `tracing::debug!` instead of `println!`
3. Set a panic hook that writes to stderr (Phase 1 already established this pattern for TUI)
4. Verify with `assay mcp serve < /dev/null 2>/dev/null | xxd` — should produce only valid JSON-RPC or nothing
**Warning signs:** Claude Code shows "Connection closed" or "Failed to parse response" errors.

### Pitfall 2: Missing `macros` Feature
**What goes wrong:** `#[tool_router]` and `#[tool_handler]` macros are unavailable.
**Why it happens:** rmcp's `macros` feature not enabled. However, in this workspace, `rmcp = { version = "0.17", features = ["server", "transport-io"] }` — the `server` feature does NOT automatically enable `macros`.
**How to avoid:** The workspace Cargo.toml must include the `macros` feature. Check: the `default` feature includes `macros`, but since explicit features are listed (`server`, `transport-io`), default features may be disabled depending on how cargo resolves them. **Verified via cargo metadata:** the resolved features include `default` and `macros` — this is correct because no `default-features = false` is set.
**Warning signs:** Compilation error: `cannot find attribute tool_router in this scope`.

### Pitfall 3: `Implementation::from_build_env()` Reports Wrong Crate Name
**What goes wrong:** The MCP server identifies itself with the binary crate name (`assay-cli`) instead of a meaningful name.
**Why it happens:** `from_build_env()` uses `env!("CARGO_CRATE_NAME")` which resolves at compile time to the crate being compiled. Since `assay-mcp` is a library and the macro expands in the binary crate, it reports the binary crate's name.
**How to avoid:** Either accept `assay-cli` as the server name (it's fine — the binary is named `assay`), or construct `Implementation` manually with a custom name.
**Warning signs:** Claude Code shows an unexpected server name in `/mcp` output.

### Pitfall 4: Tracing Subscriber Double Initialization
**What goes wrong:** If both the CLI and the MCP library try to initialize a tracing subscriber, the second call panics ("a global default trace dispatcher has already been set").
**Why it happens:** Both `main()` and `assay_mcp::serve()` call `tracing_subscriber::fmt().init()`.
**How to avoid:** Initialize tracing exactly once, in the MCP serve path only. The CLI's non-MCP paths don't need tracing for the spike.
**Warning signs:** Panic on startup: "a global default trace dispatcher has already been set".

### Pitfall 5: `ProtocolVersion::LATEST` vs Hardcoded Version
**What goes wrong:** Using a hardcoded `V_2024_11_05` when `LATEST` (currently `V_2025_03_26`) is available means missing protocol features.
**Why it happens:** Copying from older examples.
**How to avoid:** Use `ProtocolVersion::LATEST` unless there's a specific reason to pin a version. Claude Code supports the latest protocol version.
**Warning signs:** None immediately, but may miss newer protocol features.

## Code Examples

Verified patterns from official sources:

### Minimal Stdio Server (Complete)
```rust
// Source: Context7 rmcp docs, verified against official counter_stdio.rs example
// File: crates/assay-mcp/src/lib.rs

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    model::*,
    tool, tool_router, tool_handler,
    transport::stdio,
};

mod spike;
pub use spike::SpikeServer;

/// Start the MCP server on stdio transport.
///
/// This function initializes tracing to stderr (MCP-06) and blocks
/// until the client disconnects.
pub async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    tracing::info!("Starting assay MCP server");

    let service = SpikeServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("MCP serve error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}

fn init_logging() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}
```

### Spike Tool Definition
```rust
// Source: Context7 rmcp docs
// File: crates/assay-mcp/src/spike.rs

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::ToolRouter,
    model::*,
    tool, tool_router, tool_handler,
};

#[derive(Clone)]
pub struct SpikeServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl SpikeServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Spike: returns a hardcoded greeting to validate MCP protocol")]
    async fn spike_echo(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "spike: hello from assay",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for SpikeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Assay MCP spike server. Single spike_echo tool for protocol validation."
                    .to_string(),
            ),
        }
    }
}
```

### Claude Code Plugin Configuration
```json
// File: .mcp.json (project-scoped, checked into repo)
{
  "mcpServers": {
    "assay": {
      "type": "stdio",
      "command": "cargo",
      "args": ["run", "-p", "assay-cli", "--", "mcp", "serve"]
    }
  }
}
```

**Alternative (release binary, faster startup):**
```json
{
  "mcpServers": {
    "assay": {
      "type": "stdio",
      "command": "./target/release/assay-cli",
      "args": ["mcp", "serve"]
    }
  }
}
```

### Verification Script (manual JSON-RPC test)
```bash
# Send initialize request and verify response
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | cargo run -p assay-cli -- mcp serve 2>/dev/null | head -1 | python3 -c "import sys,json; r=json.load(sys.stdin); print('GO' if r.get('result',{}).get('serverInfo') else 'NO-GO')"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|-------------|-----------------|--------------|--------|
| Manual `ServerHandler` impl with `list_tools`/`call_tool` | `#[tool_handler]` macro auto-generates both | rmcp 0.14+ | Eliminates ~30 lines of dispatch boilerplate |
| `ProtocolVersion::V_2024_11_05` | `ProtocolVersion::LATEST` (= `V_2025_03_26`) | rmcp 0.17 | Newer protocol version; LATEST tracks upstream |
| Custom tool schema construction | `#[tool]` macro + schemars derive | rmcp 0.14+ | Schema auto-generated from Rust types |
| `ServerCapabilities` manual struct | `ServerCapabilities::builder().enable_tools().build()` | rmcp 0.15+ | Builder pattern for capabilities |

**Current in rmcp 0.17.0:**
- `ProtocolVersion::LATEST` = `V_2025_03_26` (V_2025_06_18 exists but LATEST not yet pointed to it; comment says "until full compliance and automated testing are in place")
- `Implementation::from_build_env()` uses `CARGO_CRATE_NAME` and `CARGO_PKG_VERSION`
- `ServerCapabilities::builder()` supports `.enable_tools()`, `.enable_prompts()`, `.enable_resources()`
- `#[tool_handler]` defaults to `self.tool_router` field name

## Open Questions

Things that couldn't be fully resolved:

1. **`Implementation::from_build_env()` crate name in library context**
   - What we know: It uses `env!("CARGO_CRATE_NAME")`. When called from code compiled in the `assay-cli` binary crate, it will report `assay-cli` (or `assay_cli`).
   - What's unclear: Whether this is desirable or if a custom `Implementation { name: "assay".into(), ... }` would be better.
   - Recommendation: Use `from_build_env()` for the spike — it's correct enough. Revisit in Phase 8 if the name matters for Claude Code's UI.

2. **Claude Code MCP server startup timeout**
   - What we know: Default timeout exists; configurable via `MCP_TIMEOUT` env var. Using `cargo run` for the spike means slow startup (compilation + linking).
   - What's unclear: Exact default timeout value (not documented precisely).
   - Recommendation: Build release binary first (`cargo build --release -p assay-cli`), then point `.mcp.json` at the compiled binary for reliable startup. Or use `cargo run` in the `.mcp.json` and accept the slow first start.

3. **Spike code lifecycle after GO**
   - What we know: CONTEXT.md marks this as Claude's Discretion.
   - What's unclear: Whether to strip spike code after GO or leave it until Phase 8.
   - Recommendation: Leave spike code in place until Phase 8 replaces it. It serves as a working reference and integration test fixture.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_rmcp` — tool_router, tool_handler, ServerHandler, ServiceExt, stdio transport, Content, CallToolResult, ProtocolVersion, Implementation
- Context7 `/websites/rs_rmcp_rmcp` — tool_handler macro expansion details
- Context7 `/websites/rs_tracing-subscriber` — fmt subscriber, EnvFilter, stderr writer
- [Official rmcp counter_stdio.rs example](https://github.com/modelcontextprotocol/rust-sdk/blob/main/examples/servers/src/counter_stdio.rs) — verified complete server main pattern
- [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp) — plugin configuration, scopes, .mcp.json format, stdio transport setup

### Secondary (MEDIUM confidence)
- [Shuttle blog: How to Build a stdio MCP Server in Rust](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust) — confirms `#[tool_handler]` pattern, verified against Context7
- cargo metadata output — rmcp 0.17.0 resolved features: `base64, default, macros, server, transport-async-rw, transport-io`

### Tertiary (LOW confidence)
- None. All findings verified against primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — rmcp 0.17.0 verified via Context7 + cargo metadata; tracing-subscriber verified via Context7
- Architecture: HIGH — patterns verified against official examples and Context7 docs
- Pitfalls: HIGH — derived from verified API behavior (stdout/stderr semantics, macro requirements, env! resolution)

**Research date:** 2026-02-28
**Valid until:** 2026-03-28 (rmcp is active but 0.17 is stable; patterns unlikely to change within 30 days)
