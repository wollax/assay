# Codebase Structure

**Analysis Date:** 2026-02-28

## Directory Layout

```
assay/
├── .github/                          # GitHub Actions workflows and config
├── .planning/                        # Kata planning documents and analysis
│   └── codebase/                     # Architecture and structure docs
├── crates/                           # Rust workspace crates
│   ├── assay-types/                  # Shared serializable types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                # Spec, Gate, Review, Workflow, Config definitions
│   ├── assay-core/                   # Domain logic and business operations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                # Module declarations
│   │       ├── spec/mod.rs           # Spec authoring and validation
│   │       ├── gate/mod.rs           # Quality gate evaluation
│   │       ├── review/mod.rs         # Work review logic
│   │       ├── workflow/mod.rs       # Workflow orchestration
│   │       └── config/mod.rs         # Configuration loading
│   ├── assay-cli/                    # CLI presentation layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs               # Clap parser and CLI entry point
│   └── assay-tui/                    # TUI presentation layer
│       ├── Cargo.toml
│       └── src/
│           └── main.rs               # Ratatui event loop and rendering
├── ide/                              # IDE plugin (future)
├── plugins/                          # AI system plugins (claude-code, codex, opencode)
├── schemas/                          # JSON schemas (generated or manual)
├── Cargo.toml                        # Workspace root configuration
├── Cargo.lock                        # Locked dependency versions
├── justfile                          # Just recipe commands (build, test, dev, ready)
├── CLAUDE.md                         # Project-specific Claude instructions
├── README.md                         # Project overview
└── [config files]                    # .editorconfig, .gitignore, .linear.toml, .mise.toml, etc.
```

## Directory Purposes

**`crates/assay-types/`**
- Pure data structures with no business logic
- Only dependencies: serde, schemars
- All types used across the workspace must be defined here
- Types are serializable (derive Serialize, Deserialize) and schema-capable (derive JsonSchema)

**`crates/assay-core/`**
- Domain logic, business rules, and operations
- Five main modules: spec, gate, review, workflow, config
- Each module (`mod.rs`) contains the implementation for its subdomain
- Only dependency besides workspace: thiserror (for structured errors)
- Presentation-agnostic; no CLI, TUI, or UI-specific code

**`crates/assay-cli/`**
- Command-line argument parsing via clap
- Single entry point: `main.rs`
- Thin wrapper that delegates to assay-core functions
- Responsible for: parsing args, calling core logic, formatting output
- No business logic; orchestrates core operations

**`crates/assay-tui/`**
- Terminal user interface via ratatui + crossterm
- Single entry point: `main.rs`
- Handles: terminal initialization, event loop, frame rendering, user input
- Delegates business logic to assay-core
- Uses color-eyre for error formatting

**`ide/`**
- Placeholder for IDE integration (not yet implemented)

**`plugins/`**
- AI system plugins for Claude Code, Codex, OpenCode
- Contains agents, commands, skills, hooks
- Integrates Assay workflows with various agentic systems

**`.planning/codebase/`**
- Kata analysis documents (ARCHITECTURE.md, STRUCTURE.md)
- Consumed by Kata CLI for planning and code generation

## Key File Locations

| File | Purpose |
|------|---------|
| `crates/assay-types/src/lib.rs` | Type definitions: Spec, Gate, Review, Workflow, Config |
| `crates/assay-core/src/lib.rs` | Module declarations for spec, gate, review, workflow, config |
| `crates/assay-core/src/spec/mod.rs` | Spec authoring and validation logic |
| `crates/assay-core/src/gate/mod.rs` | Quality gate evaluation implementation |
| `crates/assay-core/src/review/mod.rs` | Review evaluation against specs |
| `crates/assay-core/src/workflow/mod.rs` | Workflow orchestration and execution |
| `crates/assay-core/src/config/mod.rs` | Configuration loading and validation |
| `crates/assay-cli/src/main.rs` | CLI argument parser and entry point |
| `crates/assay-tui/src/main.rs` | TUI event loop and rendering entry point |
| `Cargo.toml` | Workspace root; defines all shared dependencies in [workspace.dependencies] |
| `justfile` | Development commands: build, test, lint, fmt, ready, dev, cli, tui |

## Naming Conventions

**Crate Names:**
- Kebab-case: `assay-types`, `assay-core`, `assay-cli`, `assay-tui`
- Semantic suffix: `-types` (data), `-core` (logic), `-cli` (presentation), `-tui` (presentation)

**Module Names:**
- Lowercase, single domain per module: `spec`, `gate`, `review`, `workflow`, `config`
- Each module is a directory with `mod.rs` containing implementation

**Struct Names:**
- PascalCase: `Spec`, `Gate`, `Review`, `Workflow`, `Config`
- Descriptive names matching domain terminology

**Dependency Organization:**
- All workspace dependencies declared in root `Cargo.toml` [workspace.dependencies]
- Individual crate `Cargo.toml` files reference workspace deps only: `assay-types.workspace = true`
- Never add deps to individual crates without coordinating through workspace root

## Where to Add New Code

**New Domain Logic:**
1. Define types in `crates/assay-types/src/lib.rs`
2. Implement logic in new module in `crates/assay-core/src/`
3. Export module in `crates/assay-core/src/lib.rs`

**New CLI Commands:**
1. Add command structure to `crates/assay-cli/src/main.rs` using clap attributes
2. Call appropriate `assay-core` functions from command handler
3. Format output for terminal display

**New TUI Screens/Features:**
1. Add widgets/logic to `crates/assay-tui/src/main.rs` using ratatui
2. Handle keyboard events with crossterm
3. Delegate business logic to `assay-core` functions

**New Dependencies:**
1. Add to `[workspace.dependencies]` in `/Users/wollax/Git/personal/assay/Cargo.toml`
2. Reference with `.workspace = true` in individual crate manifests
3. Ensure dependency aligns with intended crate layer

## Special Directories

**`.planning/codebase/`**
- Contains Kata analysis documents (ARCHITECTURE.md, STRUCTURE.md)
- Documents are consumed by Kata commands for code generation
- Always include file paths and patterns; be prescriptive

**`plugins/`**
- Integrations with external agentic systems
- Separate from core Assay crates
- Extensions rather than core functionality

**`schemas/`**
- Contains or will contain JSON schemas (schemars can generate these from types)
- Used for validation, documentation, IDE hints

**`ide/`**
- Future IDE plugin placeholder
- Will integrate Assay workflows into development environments

---
*Structure analysis: 2026-02-28*
