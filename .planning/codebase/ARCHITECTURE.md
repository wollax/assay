# Architecture

**Analysis Date:** 2026-02-28

## Pattern Overview
**Overall:** Layered Architecture with Domain-Driven Design

**Key Characteristics:**
- Clear separation of concerns across four crates with distinct responsibilities
- Type-first design: serializable types define the contract between layers
- Domain logic isolated from infrastructure and presentation concerns
- Multiple presentation surfaces (CLI, TUI) built on common domain foundation
- Workspace dependencies managed centrally via root `Cargo.toml`

## Layers

**Data Layer (assay-types):**
- **Purpose:** Define the serializable domain model; serve as the contract between all crates
- **Location:** `crates/assay-types/src/`
- **Contains:**
  - `Spec`: Specifications that define what should be built and acceptance criteria
  - `Gate`: Quality gates that verify work meets criteria before progression
  - `Review`: Reviews of completed work against specifications
  - `Workflow`: Pipeline combining specs, gates, and reviews
  - `Config`: Top-level project configuration
- **Depends on:** serde, schemars (external serialization/schema generation)
- **Used by:** All other crates depend on assay-types for type definitions

**Domain Layer (assay-core):**
- **Purpose:** Implement business logic and domain operations without UI or CLI concerns
- **Location:** `crates/assay-core/src/`
- **Contains:**
  - `spec/`: Spec authoring, parsing, and validation
  - `gate/`: Quality gate evaluation logic
  - `review/`: Work review evaluation against specs
  - `workflow/`: Workflow orchestration and execution
  - `config/`: Configuration loading and validation
- **Depends on:** assay-types, thiserror
- **Used by:** assay-cli, assay-tui (both presentation crates)

**Presentation Layer - CLI (assay-cli):**
- **Purpose:** Command-line interface for Assay; thin wrapper delegating to assay-core
- **Location:** `crates/assay-cli/src/main.rs`
- **Contains:** Clap-based argument parser and CLI invocation logic
- **Depends on:** assay-core, clap
- **Used by:** End users invoking `assay` command

**Presentation Layer - TUI (assay-tui):**
- **Purpose:** Terminal user interface for interactive Assay operations
- **Location:** `crates/assay-tui/src/main.rs`
- **Contains:** Ratatui-based event loop and terminal rendering
- **Depends on:** assay-core, ratatui, crossterm, color-eyre
- **Used by:** End users requiring interactive terminal experience

## Data Flow

1. **Initialization:** CLI/TUI entry points parse arguments or initialize terminal
2. **Configuration Loading:** `assay-core::config` reads project configuration from `assay-types::Config`
3. **Workflow Orchestration:** `assay-core::workflow` combines specs, gates, and reviews
4. **Spec Validation:** `assay-core::spec` validates specifications against acceptance criteria
5. **Gate Evaluation:** `assay-core::gate` checks quality gates before progression
6. **Review Process:** `assay-core::review` evaluates completed work against specs
7. **Presentation:** Results flow back to CLI or TUI for display/user interaction

## Key Abstractions

- **Spec**: Declarative specification of work with name, description, and acceptance criteria
- **Gate**: Quality checkpoint with pass/fail evaluation
- **Review**: Approval decision on work with comments
- **Workflow**: Pipeline orchestrating the entire development process
- **Config**: Project-level configuration bootstrapping the entire system

## Entry Points

- **CLI:** `crates/assay-cli/src/main.rs` - Clap parser delegates to assay-core functions
- **TUI:** `crates/assay-tui/src/main.rs` - Ratatui event loop handles user input and rendering
- **Core Library:** `crates/assay-core/src/lib.rs` - Public modules exported for use by presentation layers

## Error Handling

- **assay-core** uses `thiserror` for structured error types (infrastructure ready, currently minimal)
- **assay-tui** uses `color-eyre` for user-friendly error formatting and recovery
- **assay-cli** (minimal error handling currently; can extend with custom Result types)
- **assay-types** is error-free by design (pure data structures)

## Cross-Cutting Concerns

- **Serialization:** Centralized in `assay-types` via serde derives; enables JSON, YAML, TOML interchange
- **Schema Generation:** schemars provides JSON Schema for all types; enables validation, documentation, IDE hints
- **Workspace Dependencies:** All shared dependencies defined in root `Cargo.toml` [workspace.dependencies]; never add deps to individual crates without coordination

---
*Architecture analysis: 2026-02-28*
