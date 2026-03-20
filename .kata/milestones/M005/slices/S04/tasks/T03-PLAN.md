---
estimated_steps: 5
estimated_files: 5
---

# T03: Wire CLI `assay pr create` and MCP `pr_create` tool

**Slice:** S04 — Gate-Gated PR Workflow
**Milestone:** M005

## Description

Expose `pr_create_if_gates_pass` through two transport layers: the CLI (`assay pr create <milestone>`) and the MCP server (`pr_create` tool). Both follow patterns established in S02 — the CLI follows the `milestone_advance_cmd` D072 pattern (eprintln + Ok(1) for domain errors), and the MCP tool follows the `cycle_advance` spawn_blocking pattern. Adds 2 CLI unit tests and 1 MCP presence test.

## Steps

1. Create `crates/assay-cli/src/commands/pr.rs`:
   ```rust
   use clap::Subcommand;
   use super::{assay_dir, project_root};

   #[derive(Subcommand)]
   pub(crate) enum PrCommand {
       /// Create a GitHub PR for a milestone after all chunk gates pass
       Create {
           /// Slug of the milestone
           milestone: String,
           /// PR title (defaults to "feat: <milestone-slug>" if omitted)
           #[arg(long)]
           title: Option<String>,
           /// PR body text
           #[arg(long)]
           body: Option<String>,
       },
   }

   pub(crate) fn handle(command: PrCommand) -> anyhow::Result<i32> {
       match command {
           PrCommand::Create { milestone, title, body } => pr_create_cmd(milestone, title, body),
       }
   }
   ```
   `pr_create_cmd` constructs `effective_title = title.unwrap_or_else(|| format!("feat: {milestone}"))`. Calls `assay_core::pr::pr_create_if_gates_pass(&assay_dir, &specs_dir, &working_dir, &milestone, &effective_title, body.as_deref())`. On `Ok(result)`: `println!("PR created: #{} — {}", result.pr_number, result.pr_url)`, return `Ok(0)`. On `Err(e)`: `eprintln!("Error: {e}")`, return `Ok(1)`.
   
   Add unit tests:
   - `pr_create_cmd_exits_1_no_assay_dir`: creates a temp dir without `.assay/`, sets cwd, calls `handle(PrCommand::Create { milestone: "x", title: None, body: None })`, asserts `Ok(1)`.
   - `pr_create_cmd_exits_1_already_created`: creates a temp project dir, writes a milestone TOML with `pr_number = 42` and `pr_url = "https://github.com/o/r/pull/42"`, calls handle, asserts `Ok(1)`.

2. Add `pub mod pr;` to `crates/assay-cli/src/commands/mod.rs`.

3. In `crates/assay-cli/src/main.rs`: add `Pr { #[command(subcommand)] command: commands::pr::PrCommand }` variant after `Plan` in the `Command` enum. Add help annotation consistent with existing command groups. Add dispatch arm `Some(Command::Pr { command }) => commands::pr::handle(command)` to `run()`.

4. In `crates/assay-mcp/src/server.rs`:
   - Add `PrCreateParams` struct before the existing `CycleStatusParams` group:
     ```rust
     #[derive(Deserialize, JsonSchema)]
     pub(crate) struct PrCreateParams {
         pub milestone_slug: String,
         pub title: String,
         #[serde(default)]
         pub body: Option<String>,
     }
     ```
   - Add `pr_create` tool method following the `cycle_advance` pattern exactly:
     ```rust
     #[tool(description = "Create a GitHub PR for a milestone after all chunk gates pass. ...")]
     pub async fn pr_create(&self, params: Parameters<PrCreateParams>) -> Result<CallToolResult, McpError> {
         let cwd = resolve_cwd()?;
         let config = match load_config(&cwd) { Ok(c) => c, Err(e) => return Ok(e) };
         let assay_dir = cwd.join(".assay");
         let specs_dir = cwd.join(".assay").join(&config.specs_dir);
         let working_dir = resolve_working_dir(&cwd, &config);
         let p = params.0;
         let result = tokio::task::spawn_blocking(move || {
             assay_core::pr::pr_create_if_gates_pass(
                 &assay_dir, &specs_dir, &working_dir,
                 &p.milestone_slug, &p.title, p.body.as_deref(),
             )
         }).await
         .map_err(|e| McpError::internal_error(format!("pr_create panicked: {e}"), None))?;
         match result {
             Ok(r) => {
                 let json = serde_json::json!({ "pr_number": r.pr_number, "pr_url": r.pr_url });
                 Ok(CallToolResult::success(vec![Content::text(json.to_string())]))
             }
             Err(e) => Ok(domain_error(&e)),
         }
     }
     ```
   - Add `pr_create_tool_in_router` presence test following the exact pattern of `cycle_advance_tool_in_router`.
   - Update the module-level doc comment listing all tools to include `pr_create`.

5. Run `just ready` and fix any fmt/clippy issues. Confirm `cargo test --workspace` is fully green.

## Must-Haves

- [ ] `assay pr create --help` displays usage with `<milestone>` positional arg, `--title`, and `--body` options
- [ ] `assay pr create` subcommand is reachable in `main.rs`; `Some(Command::Pr { .. })` dispatch arm exists
- [ ] CLI exits 0 and prints "PR created: #N — <url>" on mock success
- [ ] CLI exits 1 with `eprintln!("Error: ...")` on all failure paths (D072 pattern)
- [ ] `pr_create` MCP tool is registered in router; `pr_create_tool_in_router` test passes
- [ ] MCP `pr_create` returns `{ "pr_number": N, "pr_url": "..." }` on success and `domain_error` on failure
- [ ] `cargo test -p assay-cli -- pr` passes (2 unit tests)
- [ ] `cargo test -p assay-mcp -- pr_create` passes (presence test)
- [ ] `just ready` green (fmt + clippy + test + deny)

## Verification

- `cargo test -p assay-cli -- pr` → 2 passed
- `cargo test -p assay-mcp -- pr_create` → 1 passed
- `cargo test --workspace` → all green
- `just ready` → "All checks passed."
- `assay pr create --help` → shows positional `<milestone>` and `--title`, `--body` flags

## Observability Impact

- Signals added/changed: CLI prints "PR created: #N — <url>" to stdout on success; `eprintln!("Error: {e}")` to stderr on failure — user-visible feedback for the PR workflow
- How a future agent inspects this: `assay pr create <milestone>` exit code 0 = success, 1 = failure; MCP tool `isError` field; `assay milestone list` shows milestone status after PR created
- Failure state exposed: all domain errors from `pr_create_if_gates_pass` surface verbatim via CLI stderr and MCP domain_error — chunk failures, PR already created, gh not found all visible

## Inputs

- `crates/assay-core/src/pr.rs` — `pr_create_if_gates_pass` and `PrCreateResult` from T02
- `crates/assay-cli/src/commands/milestone.rs` — D072 CLI pattern: `eprintln!("Error: {e}")` + `return Ok(1)` for domain errors
- `crates/assay-mcp/src/server.rs` lines 3416–3448 — `cycle_advance` method as exact MCP tool pattern to follow
- `crates/assay-cli/src/main.rs` — existing `Command` enum and `run()` dispatch structure

## Expected Output

- `crates/assay-cli/src/commands/pr.rs` — new: `PrCommand`, `handle`, `pr_create_cmd`, 2 unit tests
- `crates/assay-cli/src/commands/mod.rs` — `pub mod pr;` added
- `crates/assay-cli/src/main.rs` — `Pr` variant + dispatch arm added
- `crates/assay-mcp/src/server.rs` — `PrCreateParams` + `pr_create` method + `pr_create_tool_in_router` test + doc comment updated
