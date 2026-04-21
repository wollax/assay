# Assay Monorepo -- Annotated Source Tree

```
assay/
├── Cargo.toml                          # Workspace root: members = ["crates/*", "smelt/crates/*"]
├── Cargo.lock                          # Pinned dependency graph
├── justfile                            # Task runner (build, test, lint, ready)
├── rust-toolchain.toml                 # Pinned Rust toolchain version
├── rustfmt.toml                        # Formatter config
├── clippy.toml                         # Linter config
├── deny.toml                           # cargo-deny license/advisory checks
├── package.json                        # Node tooling (schema generation, plugins)
├── package-lock.json                   # Node lockfile
├── CLAUDE.md -> ...                    # Symlinked agent instructions
├── AGENTS.md                           # Multi-agent orchestration rules
├── WORKFLOW-assay.md                   # Solo dev workflow reference
├── README.md                           # Project overview
├── CHANGELOG.md                        # Release history
├── CONTRIBUTING.md                     # Contribution guidelines
├── LICENSE                             # License file
├── rust_out                            # Compiled binary (gitignored)
│
│   ══════════════════════════════════════════════════════════════════
│   DEPENDENCY DAG (assay crates):
│
│     assay-types  <──  assay-core  <──┬── assay-backends
│     (leaf types)     (domain logic)  ├── assay-harness
│                                      ├── assay-mcp
│                                      ├── assay-cli    [ENTRY POINT: binary]
│                                      └── assay-tui    [ENTRY POINT: binary]
│
│   CROSS-PROJECT: smelt-core ──depends-on──> assay-types (path dep)
│   ══════════════════════════════════════════════════════════════════
│
├── crates/                             # *** ASSAY CRATES ***
│   │
│   ├── assay-types/                    # [LEAF] Shared serializable types (serde, schemars)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Public re-exports for all types
│   │       ├── agent_event.rs          # Agent lifecycle event types
│   │       ├── checkpoint.rs           # Checkpoint/snapshot types
│   │       ├── context.rs              # Context window types
│   │       ├── coverage.rs             # Spec coverage tracking
│   │       ├── criteria_library.rs     # Reusable criteria definitions
│   │       ├── criterion.rs            # Individual criterion types
│   │       ├── enforcement.rs          # Gate enforcement policy types
│   │       ├── evaluator.rs            # Evaluator role/result types
│   │       ├── evidence.rs             # Evidence collection types
│   │       ├── feature_spec.rs         # Feature specification types
│   │       ├── gate_run.rs             # Gate execution record types
│   │       ├── gate.rs                 # Gate definition types
│   │       ├── gates_spec.rs           # Gates specification types
│   │       ├── harness.rs              # Harness config/profile types
│   │       ├── manifest.rs             # Run manifest types
│   │       ├── merge.rs               # Merge proposal/result types
│   │       ├── milestone.rs            # Milestone tracking types
│   │       ├── orchestrate.rs          # Multi-agent orchestration types
│   │       ├── precondition.rs         # Gate precondition types
│   │       ├── provider.rs             # AI provider config types
│   │       ├── resolved_gate.rs        # Resolved gate reference types
│   │       ├── review.rs              # Review check types
│   │       ├── schema_registry.rs      # JSON Schema registry types
│   │       ├── session.rs              # Session info types
│   │       ├── signal.rs               # MCP signal protocol types
│   │       ├── state_backend.rs        # State backend config types  *** USED BY smelt-core ***
│   │       ├── validation.rs           # Validation result types
│   │       ├── wizard_input.rs         # TUI wizard input types
│   │       ├── work_session.rs         # Work session tracking types
│   │       └── worktree.rs             # Git worktree types
│   │
│   ├── assay-core/                     # [CENTRAL] Domain logic: specs, gates, reviews, workflows
│   │   ├── Cargo.toml                  # Depends on: assay-types
│   │   └── src/
│   │       ├── lib.rs                  # Public API surface
│   │       ├── init.rs                 # Project initialization
│   │       ├── manifest.rs             # Run manifest loading
│   │       ├── manifest_gen.rs         # Manifest generation from specs
│   │       ├── merge.rs                # Merge operations
│   │       ├── pr.rs                   # Pull request integration
│   │       ├── pipeline.rs             # Gate pipeline execution
│   │       ├── pipeline_checkpoint.rs  # Pipeline checkpoint persistence
│   │       ├── state_backend.rs        # State backend dispatch
│   │       ├── evaluator.rs            # Gate evaluator logic
│   │       ├── error.rs                # Error types
│   │       ├── telemetry.rs            # Telemetry/tracing
│   │       ├── work_session.rs         # Work session management
│   │       ├── worktree.rs             # Git worktree management
│   │       │
│   │       ├── checkpoint/             # Checkpoint subsystem
│   │       │   ├── mod.rs
│   │       │   ├── config.rs           # Checkpoint configuration
│   │       │   ├── extractor.rs        # Data extraction from sessions
│   │       │   └── persistence.rs      # Checkpoint storage
│   │       │
│   │       ├── config/                 # Configuration loading
│   │       │   └── mod.rs
│   │       │
│   │       ├── context/                # Context window management
│   │       │   ├── mod.rs
│   │       │   ├── budgeting.rs        # Token budget allocation
│   │       │   ├── diagnostics.rs      # Context diagnostics
│   │       │   ├── discovery.rs        # Session file discovery
│   │       │   ├── parser.rs           # JSONL session parser
│   │       │   ├── tokens.rs           # Token counting
│   │       │   └── pruning/            # Context pruning engine
│   │       │       ├── mod.rs
│   │       │       ├── backup.rs       # Pre-prune backup
│   │       │       ├── protection.rs   # Protected content rules
│   │       │       ├── report.rs       # Pruning report generation
│   │       │       ├── strategy.rs     # Strategy trait definition
│   │       │       └── strategies/     # Pluggable pruning strategies
│   │       │           ├── mod.rs
│   │       │           ├── metadata_strip.rs
│   │       │           ├── progress_collapse.rs
│   │       │           ├── stale_reads.rs
│   │       │           ├── system_reminder_dedup.rs
│   │       │           ├── thinking_blocks.rs
│   │       │           └── tool_output_trim.rs
│   │       │
│   │       ├── gate/                   # Gate evaluation engine
│   │       │   ├── mod.rs
│   │       │   ├── evidence.rs         # Evidence collection
│   │       │   ├── render.rs           # Gate result rendering
│   │       │   └── session.rs          # Gate session management
│   │       │
│   │       ├── guard/                  # Resource guard daemon
│   │       │   ├── mod.rs
│   │       │   ├── circuit_breaker.rs  # Circuit breaker pattern
│   │       │   ├── config.rs           # Guard configuration
│   │       │   ├── daemon.rs           # Background guard daemon
│   │       │   ├── pid.rs              # Process ID tracking
│   │       │   ├── thresholds.rs       # Resource thresholds
│   │       │   └── watcher.rs          # File/process watcher
│   │       │
│   │       ├── history/                # Session history analytics
│   │       │   ├── mod.rs
│   │       │   └── analytics.rs        # Usage analytics
│   │       │
│   │       ├── milestone/              # Milestone lifecycle
│   │       │   ├── mod.rs
│   │       │   └── cycle.rs            # Milestone cycle management
│   │       │
│   │       ├── orchestrate/            # Multi-agent orchestration
│   │       │   ├── mod.rs
│   │       │   ├── conflict_resolver.rs # Merge conflict resolution
│   │       │   ├── dag.rs              # Task dependency DAG
│   │       │   ├── executor.rs         # DAG executor
│   │       │   ├── gossip.rs           # Peer gossip protocol
│   │       │   ├── merge_runner.rs     # Orchestrated merge runner
│   │       │   ├── mesh.rs             # Agent mesh networking
│   │       │   └── ordering.rs         # Task ordering
│   │       │
│   │       ├── review/                 # Code review subsystem
│   │       │   └── mod.rs
│   │       │
│   │       ├── spec/                   # Spec management
│   │       │   ├── mod.rs
│   │       │   ├── compose.rs          # Spec composition
│   │       │   ├── coverage.rs         # Spec coverage analysis
│   │       │   ├── promote.rs          # Spec promotion workflow
│   │       │   └── validate.rs         # Spec validation
│   │       │
│   │       ├── wizard/                 # Interactive wizard logic
│   │       │   ├── mod.rs
│   │       │   ├── criteria.rs         # Criteria wizard
│   │       │   ├── gate.rs             # Gate wizard
│   │       │   └── milestone.rs        # Milestone wizard
│   │       │
│   │       └── workflow/               # Workflow engine
│   │           └── mod.rs
│   │
│   ├── assay-backends/                 # State backend implementations
│   │   ├── Cargo.toml                  # Depends on: assay-types, assay-core
│   │   └── src/
│   │       ├── lib.rs                  # Backend trait + re-exports
│   │       ├── factory.rs              # Backend factory dispatch
│   │       ├── github.rs              # GitHub state backend
│   │       ├── linear.rs               # Linear issue tracker backend
│   │       ├── smelt.rs                # Smelt infrastructure backend
│   │       └── ssh.rs                  # SSH remote backend
│   │
│   ├── assay-harness/                  # Single-agent harness (spec runner)
│   │   ├── Cargo.toml                  # Depends on: assay-types, assay-core
│   │   └── src/
│   │       ├── lib.rs                  # Harness public API
│   │       ├── claude.rs               # Claude Code harness adapter
│   │       ├── claude_stream.rs        # Claude streaming output parser
│   │       ├── codex.rs                # OpenAI Codex harness adapter
│   │       ├── opencode.rs             # Opencode harness adapter
│   │       ├── prompt.rs               # Prompt assembly/templating
│   │       ├── provider.rs             # Provider dispatch
│   │       ├── scope.rs                # Scope enforcement
│   │       ├── settings.rs             # Harness settings
│   │       └── snapshots/              # Insta snapshot tests (19 snaps)
│   │
│   ├── assay-mcp/                      # MCP server with signal endpoint
│   │   ├── Cargo.toml                  # Depends on: assay-types, assay-core
│   │   └── src/
│   │       ├── lib.rs                  # MCP server library
│   │       ├── server.rs               # MCP JSON-RPC server
│   │       ├── signal_server.rs        # Signal protocol endpoint
│   │       └── snapshots/              # Insta snapshot tests (2 snaps)
│   │
│   ├── assay-cli/                      # [ENTRY POINT] CLI binary (clap)
│   │   ├── Cargo.toml                  # Depends on: all assay crates
│   │   └── src/
│   │       ├── main.rs                 # *** Binary entry point ***
│   │       └── commands/               # CLI subcommands
│   │           ├── mod.rs              # Command dispatch
│   │           ├── checkpoint.rs       # `assay checkpoint` command
│   │           ├── context.rs          # `assay context` command
│   │           ├── criteria.rs         # `assay criteria` command
│   │           ├── gate.rs             # `assay gate` command
│   │           ├── harness.rs          # `assay harness` command
│   │           ├── history.rs          # `assay history` command
│   │           ├── init.rs             # `assay init` command
│   │           ├── manifest.rs         # `assay manifest` command
│   │           ├── mcp.rs              # `assay mcp` command
│   │           ├── milestone.rs        # `assay milestone` command
│   │           ├── plan.rs             # `assay plan` command
│   │           ├── pr.rs               # `assay pr` command
│   │           ├── run.rs              # `assay run` command
│   │           ├── spec.rs             # `assay spec` command
│   │           ├── traces.rs           # `assay traces` command
│   │           ├── wizard_helpers.rs   # Wizard utility functions
│   │           └── worktree.rs         # `assay worktree` command
│   │
│   └── assay-tui/                      # [ENTRY POINT] TUI binary (ratatui)
│       ├── Cargo.toml                  # Depends on: assay-types, assay-core, assay-mcp
│       └── src/
│           ├── main.rs                 # *** Binary entry point ***
│           ├── lib.rs                  # TUI library
│           ├── app.rs                  # Application state/loop
│           ├── agent.rs                # Agent panel
│           ├── event.rs                # Terminal event handling
│           ├── gate_wizard.rs          # Interactive gate wizard
│           ├── mcp_panel.rs            # MCP server panel
│           ├── slash.rs                # Slash command handling
│           ├── trace_viewer.rs         # Trace visualization
│           └── wizard.rs               # TUI wizard framework
│
├── smelt/                              # *** SMELT PROJECT (infrastructure layer) ***
│   ├── Cargo.lock                      # Smelt-specific lockfile
│   ├── justfile                        # Smelt-specific tasks
│   ├── README.md
│   ├── examples/                       # Smelt manifest examples
│   │   ├── agent-manifest.toml         # Agent job manifest
│   │   ├── job-manifest.toml           # Basic job manifest
│   │   ├── job-manifest-compose.toml   # Docker Compose manifest
│   │   ├── job-manifest-forge.toml     # Forge delivery manifest
│   │   ├── job-manifest-k8s.toml       # Kubernetes manifest
│   │   ├── bad-manifest.toml           # Invalid manifest (for testing)
│   │   └── server.toml                 # Server configuration
│   │
│   └── crates/
│       ├── smelt-core/                 # Infrastructure domain logic
│       │   ├── Cargo.toml              # Depends on: assay-types (path = "../../../crates/assay-types")
│       │   └── src/
│       │       ├── lib.rs              # Core library exports
│       │       ├── assay.rs            # *** INTEGRATION: assay-types bridge ***
│       │       ├── collector.rs        # Artifact collector
│       │       ├── compose.rs          # Docker Compose orchestration
│       │       ├── config.rs           # Smelt configuration
│       │       ├── docker.rs           # Docker container lifecycle
│       │       ├── error.rs            # Error types
│       │       ├── forge.rs            # Forge (Forgejo/Gitea) delivery
│       │       ├── k8s.rs              # Kubernetes job execution
│       │       ├── monitor.rs          # Job health monitoring
│       │       ├── provider.rs         # Provider abstraction
│       │       ├── tracker.rs          # Job state tracker
│       │       ├── git/                # Git operations
│       │       │   ├── mod.rs
│       │       │   └── cli/            # Git CLI wrapper
│       │       │       ├── mod.rs
│       │       │       └── tests/      # Git CLI tests
│       │       │           ├── mod.rs
│       │       │           ├── basic.rs
│       │       │           ├── branch.rs
│       │       │           ├── commit.rs
│       │       │           ├── merge.rs
│       │       │           └── worktree.rs
│       │       └── manifest/           # Smelt manifest parsing
│       │           ├── mod.rs
│       │           ├── validation.rs   # Manifest validation
│       │           └── tests/          # Manifest tests
│       │               ├── mod.rs
│       │               ├── core.rs
│       │               ├── compose.rs
│       │               ├── forge.rs
│       │               └── kubernetes.rs
│       │
│       └── smelt-cli/                  # [ENTRY POINT] Smelt daemon binary
│           ├── Cargo.toml              # Depends on: smelt-core
│           └── src/
│               ├── main.rs             # *** Binary entry point ***
│               ├── lib.rs              # CLI library
│               ├── commands/           # CLI subcommands
│               │   ├── mod.rs          # Command dispatch
│               │   ├── init.rs         # `smelt init`
│               │   ├── list.rs         # `smelt list`
│               │   ├── serve.rs        # `smelt serve` (daemon mode)
│               │   ├── status.rs       # `smelt status`
│               │   ├── watch.rs        # `smelt watch`
│               │   └── run/            # `smelt run` subcommand group
│               │       ├── mod.rs
│               │       ├── dry_run.rs  # Dry-run mode
│               │       ├── helpers.rs  # Run helpers
│               │       └── phases.rs   # Phase execution
│               │
│               └── serve/              # Daemon server subsystem
│                   ├── mod.rs
│                   ├── config.rs       # Server configuration
│                   ├── dispatch.rs     # Job dispatch logic
│                   ├── events.rs       # Event system
│                   ├── http_api.rs     # HTTP API endpoints
│                   ├── notify.rs       # Notification system
│                   ├── queue.rs        # Job queue
│                   ├── queue_watcher.rs # Queue monitoring
│                   ├── signals.rs      # Signal handling
│                   ├── tracker.rs      # Daemon-level tracker
│                   ├── tracker_poller.rs # Tracker polling
│                   ├── tui.rs          # Daemon TUI overlay
│                   ├── types.rs        # Serve-specific types
│                   ├── github/         # GitHub webhook source
│                   │   ├── mod.rs
│                   │   ├── client.rs
│                   │   ├── source.rs
│                   │   └── mock.rs
│                   ├── linear/         # Linear webhook source
│                   │   ├── mod.rs
│                   │   ├── client.rs
│                   │   ├── source.rs
│                   │   └── mock.rs
│                   ├── ssh/            # SSH worker pool
│                   │   ├── mod.rs
│                   │   ├── client.rs
│                   │   ├── operations.rs
│                   │   └── mock.rs
│                   └── tests/          # Serve integration tests
│                       ├── mod.rs
│                       ├── config.rs
│                       ├── dispatch.rs
│                       ├── events.rs
│                       ├── http.rs
│                       ├── notify.rs
│                       ├── queue.rs
│                       ├── signals.rs
│                       └── ssh_dispatch.rs
│
├── plugins/                            # *** AGENTIC AI PLUGINS ***
│   │
│   ├── claude-code/                    # Claude Code plugin (richest)
│   │   ├── .claude-plugin/
│   │   │   └── plugin.json             # Plugin manifest
│   │   ├── .mcp.json                   # MCP server config
│   │   ├── CLAUDE.md                   # Agent instructions
│   │   ├── README.md
│   │   ├── agents/                     # (placeholder)
│   │   ├── commands/                   # (placeholder)
│   │   ├── hooks/
│   │   │   └── hooks.json              # Hook definitions
│   │   ├── scripts/
│   │   │   ├── checkpoint-hook.sh      # Post-checkpoint hook
│   │   │   ├── cycle-stop-check.sh     # Cycle termination check
│   │   │   ├── post-tool-use.sh        # Post-tool-use hook
│   │   │   └── stop-gate-check.sh      # Stop gate enforcement
│   │   └── skills/                     # 9 slash-command skills
│   │       ├── check/SKILL.md          # /check — run gate checks
│   │       ├── explore/SKILL.md        # /explore — codebase exploration
│   │       ├── focus/SKILL.md          # /focus — set working context
│   │       ├── gate-check/SKILL.md     # /gate-check — gate evaluation
│   │       ├── next-chunk/SKILL.md     # /next-chunk — next work item
│   │       ├── plan/SKILL.md           # /plan — generate work plan
│   │       ├── ship/SKILL.md           # /ship — finalize delivery
│   │       ├── spec-show/SKILL.md      # /spec-show — display spec
│   │       └── status/SKILL.md         # /status — session status
│   │
│   ├── codex/                          # OpenAI Codex plugin
│   │   ├── AGENTS.md                   # Agent instructions
│   │   ├── README.md
│   │   └── skills/                     # 9 skill definitions
│   │       ├── check.md
│   │       ├── cycle-status.md
│   │       ├── explore.md
│   │       ├── focus.md
│   │       ├── gate-check.md
│   │       ├── next-chunk.md
│   │       ├── plan.md
│   │       ├── ship.md
│   │       └── spec-show.md
│   │
│   ├── opencode/                       # Opencode plugin
│   │   ├── AGENTS.md                   # Agent instructions
│   │   ├── README.md
│   │   ├── opencode.json               # Plugin config
│   │   ├── package.json                # Node package manifest
│   │   ├── tsconfig.json               # TypeScript config
│   │   ├── commands/                   # (placeholder)
│   │   ├── plugins/                    # (placeholder)
│   │   ├── tools/                      # (placeholder)
│   │   └── skills/                     # 9 skill definitions
│   │       ├── check.md
│   │       ├── cycle-status.md
│   │       ├── explore.md
│   │       ├── focus.md
│   │       ├── gate-check.md
│   │       ├── next-chunk.md
│   │       ├── plan.md
│   │       ├── ship.md
│   │       └── spec-show.md
│   │
│   └── smelt-agent/                    # Smelt daemon agent skills
│       ├── AGENTS.md                   # Agent instructions
│       ├── skills/
│       │   ├── backend-status.md       # Backend health check
│       │   ├── peer-message.md         # Peer-to-peer messaging
│       │   ├── peer-registry.md        # Peer discovery/registry
│       │   └── run-dispatch.md         # Job dispatch skill
│       └── tests/
│           └── verify-docs.sh          # Documentation verification
│
├── schemas/                            # *** JSON SCHEMA FILES (86 files) ***
│   ├── config.schema.json              # Root config schema
│   ├── spec.schema.json                # Feature spec schema
│   ├── feature-spec.schema.json        # Feature spec (extended)
│   ├── gate.schema.json                # Gate definition
│   ├── gates-config.schema.json        # Gates configuration
│   ├── gates-spec.schema.json          # Gates specification
│   ├── gate-kind.schema.json           # Gate kind enum
│   ├── gate-result.schema.json         # Gate evaluation result
│   ├── gate-run-record.schema.json     # Gate run record
│   ├── gate-run-summary.schema.json    # Gate run summary
│   ├── gate-section.schema.json        # Gate section grouping
│   ├── gate-criterion.schema.json      # Gate criterion binding
│   ├── gate-diagnostic.schema.json     # Gate diagnostic output
│   ├── gate-eval-context.schema.json   # Gate evaluation context
│   ├── criterion.schema.json           # Criterion definition
│   ├── criterion-kind.schema.json      # Criterion kind enum
│   ├── criterion-outcome.schema.json   # Criterion outcome
│   ├── criterion-result.schema.json    # Criterion result
│   ├── when.schema.json                # Conditional "when" clause
│   ├── workflow.schema.json            # Workflow definition
│   ├── review.schema.json              # Review definition
│   ├── review-check.schema.json        # Review check
│   ├── review-check-kind.schema.json   # Review check kind
│   ├── review-report.schema.json       # Review report
│   ├── enforcement.schema.json         # Enforcement policy
│   ├── enforcement-summary.schema.json # Enforcement summary
│   ├── signal-request.schema.json      # MCP signal request
│   ├── signal-gate-summary.schema.json # Signal gate summary
│   ├── poll-signals-result.schema.json # Signal poll result
│   ├── assay-server-state.schema.json  # MCP server state
│   ├── provider_config.schema.json     # Provider configuration
│   ├── provider_kind.schema.json       # Provider kind enum
│   ├── state-backend-config.schema.json # State backend config
│   ├── harness-profile.schema.json     # Harness profile
│   ├── settings-override.schema.json   # Settings override
│   ├── session-info.schema.json        # Session info
│   ├── session-phase.schema.json       # Session phase
│   ├── sessions-config.schema.json     # Sessions config
│   ├── work-session.schema.json        # Work session
│   ├── agent-event.schema.json         # Agent event
│   ├── agent-evaluation.schema.json    # Agent evaluation
│   ├── agent-invocation.schema.json    # Agent invocation
│   ├── checkpoint-session-phase.schema.json
│   ├── team-checkpoint.schema.json     # Team checkpoint
│   ├── phase-transition.schema.json    # Phase transition
│   ├── milestone.schema.json           # Milestone definition
│   ├── milestone-status.schema.json    # Milestone status
│   ├── run-manifest.schema.json        # Run manifest
│   ├── run-summary.schema.json         # Run summary
│   ├── manifest-session.schema.json    # Manifest session
│   ├── merge-proposal.schema.json      # Merge proposal
│   ├── merge-check.schema.json         # Merge check
│   ├── merge-execute-result.schema.json # Merge execution result
│   ├── conflict-marker.schema.json     # Conflict marker
│   ├── conflict-scan.schema.json       # Conflict scan
│   ├── worktree-info.schema.json       # Worktree info
│   ├── worktree-status.schema.json     # Worktree status
│   ├── worktree-metadata.schema.json   # Worktree metadata
│   ├── worktree-config.schema.json     # Worktree config
│   ├── context types...                # (context/diagnostics/token schemas)
│   ├── token-estimate.schema.json
│   ├── diagnostics-report.schema.json
│   ├── prune-report.schema.json
│   ├── diagnostic.schema.json
│   ├── diagnostic-summary.schema.json
│   ├── validation-result.schema.json
│   ├── formatted-evidence.schema.json
│   ├── guard-config.schema.json        # Guard daemon config
│   ├── coverage-report.schema.json     # Coverage report
│   ├── confidence.schema.json          # Confidence level
│   ├── severity.schema.json            # Severity level
│   ├── evaluator-role.schema.json      # Evaluator role
│   ├── evaluator-output.schema.json    # Evaluator output
│   ├── evaluator-summary.schema.json   # Evaluator summary
│   ├── evaluator-criterion-result.schema.json
│   ├── failed-criterion-summary.schema.json
│   ├── diff-truncation.schema.json     # Diff truncation config
│   ├── tool-call-summary.schema.json   # Tool call summary
│   ├── hook-event.schema.json          # Hook event
│   ├── hook-contract.schema.json       # Hook contract
│   ├── prompt-layer.schema.json        # Prompt layer
│   ├── prompt-layer-kind.schema.json   # Prompt layer kind
│   ├── scope-violation.schema.json     # Scope violation
│   ├── scope-violation-type.schema.json
│   ├── chunk-ref.schema.json           # Chunk reference
│   └── peer-update.schema.json         # Peer update
│       peer-info.schema.json           # Peer info
│
├── examples/                           # *** USAGE EXAMPLES ***
│   └── close-the-loop/                 # End-to-end workflow example
│       ├── spec.toml                   # Feature specification
│       ├── gates.toml                  # Gate definitions
│       ├── manifest.toml               # Run manifest
│       ├── manifest-abort.toml         # Abort-path manifest
│       ├── manifest-promote.toml       # Promote-path manifest
│       ├── prompt-abort.md             # Abort prompt template
│       ├── prompt-clean.md             # Clean prompt template
│       ├── setup.sh                    # Example setup script
│       ├── reset.sh                    # Example reset script
│       ├── run-abort.sh                # Run abort scenario
│       ├── run-promote.sh              # Run promote scenario
│       └── README.md
│
├── openspec/                           # *** OPENSPEC CHANGE MANAGEMENT ***
│   ├── config.yaml                     # OpenSpec config
│   ├── explore-solo-workflow.md        # Exploration document
│   ├── workflow-current-state.md       # Current workflow analysis
│   ├── workflow-desired-state.md       # Desired workflow target
│   ├── specs/                          # (empty — specs promoted to changes/)
│   └── changes/
│       ├── archive/                    # Completed changes
│       └── solo-workflow-tighten/      # Active change proposal
│           ├── proposal.md             # Change proposal
│           ├── design.md               # Design document
│           ├── tasks.md                # Implementation tasks
│           ├── REVIEW-PROMPT.md        # Review prompt
│           └── specs/                  # 9 feature specs
│               ├── branch-isolation/spec.md
│               ├── explore-phase/spec.md
│               ├── gate-evidence-rendering/spec.md
│               ├── plan-quick/spec.md
│               ├── session-retention/spec.md
│               ├── smart-gate-routing/spec.md
│               ├── solo-skill-surface/spec.md
│               ├── spec-status/spec.md
│               └── workflow-engine/spec.md
│
├── docs/                               # Project documentation
│   ├── project-scan-report.json        # Automated scan report
│   ├── scan-assay.md                   # Assay scan analysis
│   ├── scan-plugins.md                 # Plugins scan analysis
│   └── scan-smelt.md                   # Smelt scan analysis
│
├── ide/                                # IDE configuration
│   └── README.md
│
├── .forgejo/                           # *** CI CONFIGURATION ***
│   └── workflows/
│       └── ci.yml                      # Forgejo Actions CI pipeline
│
└── .githooks/                          # *** GIT HOOKS ***
    ├── pre-commit                      # Pre-commit checks
    ├── pre-push                        # Pre-push checks
    └── post-merge                      # Post-merge actions
```

## Integration Points

| From | To | Mechanism |
|------|----|-----------|
| `assay-cli` | all assay crates | Direct Cargo dependency |
| `assay-tui` | `assay-core`, `assay-mcp`, `assay-types` | Direct Cargo dependency |
| `assay-mcp` | `assay-core`, `assay-types` | Direct Cargo dependency |
| `assay-harness` | `assay-core`, `assay-types` | Direct Cargo dependency |
| `assay-backends` | `assay-core`, `assay-types` | Direct Cargo dependency |
| `assay-core` | `assay-types` | Direct Cargo dependency |
| **`smelt-core`** | **`assay-types`** | **Cross-project path dependency** |
| `smelt-cli` | `smelt-core` | Direct Cargo dependency |
| Plugins | CLI/MCP | Shell scripts invoke `assay` binary |
| Schemas | `assay-types` | Generated from schemars derives |
| CI | `justfile` | `just ready` in Forgejo Actions |

## Entry Points

- **`crates/assay-cli/src/main.rs`** -- Primary CLI binary (`assay`)
- **`crates/assay-tui/src/main.rs`** -- TUI binary (`assay-tui`)
- **`smelt/crates/smelt-cli/src/main.rs`** -- Smelt daemon binary (`smelt`)
