# State

## Current Position

Phase: 10 of 10 — Claude Code Plugin
Plan: 01 of 02
Status: In progress
Last activity: 2026-03-03 — Completed 10-01-PLAN.md

Progress: [█████████░] 91% (39/43 requirements)

## Milestone Progress

| Milestone | Phases | Requirements | Complete |
|-----------|--------|--------------|----------|
| v0.1.0 | 10 | 43 | 91% |

## Phase Status

| Phase | Name | Status |
|-------|------|--------|
| 1 | Workspace Prerequisites | Complete |
| 2 | MCP Spike | Complete (GO) |
| 3 | Error Types and Domain Model | Complete |
| 4 | Schema Generation | Complete |
| 5 | Config and Initialization | Complete (3/3 plans) |
| 6 | Spec Files | Complete (2/2 plans) |
| 7 | Gate Evaluation | Complete (2/2 plans) |
| 8 | MCP Server Tools | Complete (2/2 plans) |
| 9 | CLI Surface Completion | Complete (2/2 plans) |
| 10 | Claude Code Plugin | In Progress (1/2 plans) |

## Accumulated Context

### Decisions

- assay-types = pub DTOs, zero logic; assay-core = free functions, all behavior
- CLI/MCP = thin wrappers delegating to core
- Config (Gate) != State (GateResult) — never mix them
- Add error variants when consumed, not speculatively
- Criteria live on spec with optional `cmd` field (forward-compatible with `prompt` for agent track)
- MCP spike days 1-2 as GO/NO-GO gate
- M1 = foundation/proof of concept; M2 = launch/external demo
- schemars 0.8 -> 1.x is mandatory prerequisite (rmcp requires it)
- assay-mcp is a library crate, not a binary — single `assay` binary for all surfaces
- ~~`Command::output()` for gate execution~~ → superseded: `spawn()` + reader threads + `try_wait` polling for timeout support
- `spawn_blocking` for sync gate evaluation in async MCP handlers
- `#[serde(tag = "kind")]` internal tagging on GateKind for TOML compatibility
- schemars uses caret range `"1"` (not exact pin) — matches rmcp's own declaration, picks up semver patches
- deny.toml required no changes for rmcp transitive deps — all licenses already in allow-list
- **MCP Spike: GO** — rmcp 0.17 + stdio + Claude Code integration path confirmed
- rmcp's `#[tool_router]` / `#[tool_handler]` macro pattern works cleanly for tool registration
- `tracing-subscriber` stderr-only writer keeps stdout clean for JSON-RPC (no byte leakage)
- `Implementation::from_build_env()` populates server info from Cargo.toml automatically
- ~~Spike code remains as working reference until Phase 8 replaces with real tools~~ → Phase 8 Plan 01 deleted spike.rs, replaced with AssayServer
- GateResult does not derive PartialEq — DateTime equality is semantically questionable
- serde_json moved to dev-dependencies in assay-types (source files don't use it)
- schemars chrono04 feature enabled at workspace level for DateTime<Utc> JsonSchema support
- AssayError::Io carries structured fields (PathBuf, String), no #[from] — context required at call sites
- #[non_exhaustive] on AssayError — new variants are non-breaking additions
- Result<T> alias for std::result::Result<T, AssayError> re-exported from assay-core root
- inventory::iter returns IntoIterator, not Iterator — all_entries() calls .into_iter()
- Rust cargo examples CAN access dev-dependencies — serde_json stays as dev-dep
- schemas-check NOT added to `just ready` to avoid circular dependency during development
- All 8 public types get individual schema files (not just top-level Config)
- Schema $id uses https://assay.dev/schemas/{name}.schema.json (aspirational URL convention)
- Generated schemas committed to git for IDE/consumer access without building
- Convention: every JsonSchema-derived type MUST have inventory::submit! immediately after definition
- ConfigError in config/mod.rs, not error.rs — config-specific validation output stays with config concerns
- toml dep added to assay-core for config loading (Plan 02)
- Existing Workflow/Gate types left untouched — only Config replaced; placeholders revisited later
- from_str() returns toml::de::Error (not AssayError) for composability — raw parse details for tests/tools
- validate() returns Vec<ConfigError> (not AssayError) — separates collection from reporting
- load() composes from_str + validate, wraps errors with file path context
- tempfile added as workspace dev-dependency for filesystem test isolation
- String templates for generated files (toml::to_string cannot emit comments)
- create_dir() not create_dir_all() for .assay/ — atomic idempotency guard
- Minimal project name sanitization — fallback to "assay-project" only when file_name() is None/empty
- SpecError in spec/mod.rs, not error.rs — mirrors ConfigError pattern, spec-specific validation output stays with spec concerns
- Duplicate spec names: first-seen wins, later file with duplicate name becomes an error entry
- Criterion name validation: empty names checked before duplicate detection
- ANSI escape codes for CLI colors (no external color dependency), NO_COLOR env var respected per no-color.org
- println!-based table formatting (no external table library), dynamic column widths from data
- serde_json wired to assay-cli for --json output; both spec commands resolve specs_dir from config::load()
- serde added to assay-core dependencies for GateRunSummary/CriterionResult Serialize derive
- Truncation uses str::ceil_char_boundary for safe UTF-8 slicing on tail-biased truncation
- GateRunSummary and CriterionResult live in assay-core::gate (computed summaries, not DTOs)
- evaluate_file_exists is a standalone public function, not derived from Criterion (future phases add file-check criteria)
- Minimum timeout floor of 1 second enforced by resolve_timeout
- Streaming progress uses eprint!/eprintln! (stderr), summary line uses println! (stdout), JSON uses println! (stdout)
- For streaming path, CLI iterates criteria manually (not via evaluate_all) to show per-criterion "running" state
- JSON path uses evaluate_all() directly since no streaming needed
- Evidence display: multi-line output indented with 4 spaces, labeled with "stdout:" / "stderr:"
- Working dir resolved as project root (satisfies GATE-04 as explicit choice)
- Config timeout extracted from config.gates.default_timeout
- assay-types added as direct dependency of assay-mcp (transitive access through assay-core doesn't allow type naming in function signatures)
- chrono added as dev-dependency of assay-mcp for GateResult construction in tests
- Domain errors returned as CallToolResult::error (isError: true), protocol errors as Err(McpError)
- Per-call config/spec resolution (no startup validation, no stale state)
- No tool name prefix (spec_get not assay_spec_get) — MCP servers already namespace tools
- first_nonempty_line extracts failure reason from stderr for summary mode; empty stderr gets "unknown"
- Rich help via clap `after_long_help` on all commands — shows with `--help`, compact `-h` stays clean
- Bare invocation detects `.assay/` directory: in-project shows status, outside shows hint + help
- `show_status()` reuses config::load and spec::scan, no gate execution (performance: filesystem reads only)
- Hint to stderr, help to stdout via `CommandFactory::print_help` — UNIX convention
- `ANSI_COLOR_OVERHEAD` named constant replaces magic number 9 in column width calculation
- `NO_COLOR` uses `var_os` per no-color.org (handles non-UTF-8 env values correctly)
- `[[bin]] name = "assay"` in Cargo.toml — installed binary is `assay` not `assay-cli`
- plugin.json description = "Agentic development kit with spec-driven workflows" (matches CLI about text)
- plugin.json version synced from workspace Cargo.toml via `just sync-plugin-version`
- `just check-plugin-version` integrated into `just ready` for CI drift detection
- `grep + sed` for Cargo.toml version extraction, `jq` for JSON patching
- .gitignore negation pattern `!plugins/**/.mcp.json` for plugin distribution (root .mcp.json stays ignored)
- Skills use MCP tool orchestration pattern — instruct Claude to call MCP tools, not shell commands
- CLAUDE.md uses prescriptive tone with mandatory spec-first 4-step workflow
- README documents `ASSAY_STOP_HOOK_MODE` env var (enforce/warn/off) for Stop hook configuration

### Pending Issues

30 open issues (9 from Phase 8 PR review: 1 critical, 7 important, 1 suggestion)

### Blockers

None.

### Next Actions

1. Execute Phase 10 Plan 02 — Hooks and Scripts

### Session Continuity

Last session: 2026-03-03
Stopped at: Completed 10-01-PLAN.md (MCP config, skills, CLAUDE.md, README)
Resume file: None
