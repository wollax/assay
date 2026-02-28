# Codebase Concerns

**Analysis Date:** 2026-02-28

## Tech Debt

### Incomplete Module Implementations
All domain logic modules are currently stub implementations with only documentation comments:
- `/crates/assay-core/src/spec/mod.rs` - Spec authoring module has no implementation
- `/crates/assay-core/src/gate/mod.rs` - Gate evaluation logic is missing
- `/crates/assay-core/src/review/mod.rs` - Review logic is absent
- `/crates/assay-core/src/workflow/mod.rs` - Workflow orchestration is empty
- `/crates/assay-core/src/config/mod.rs` - Configuration loading is not implemented

**Impact:** Core business logic is not realized. These modules are declared but contain zero functional code.

### Minimal CLI Implementation
`/crates/assay-cli/src/main.rs` defines a Clap parser with an empty struct and only prints the version. No actual CLI commands, subcommands, or argument handling exists.

**Impact:** The CLI binary is non-functional and cannot process user inputs.

### Incomplete TUI State Management
`/crates/assay-tui/src/main.rs` has a basic event loop that only responds to 'q' key and renders a title. No state management, workflow navigation, or actual feature implementation.

**Impact:** The TUI is a skeleton. Real workflows, data display, and user interaction are absent.

## Known Bugs

None identified at this stage due to incomplete implementations.

## Security Considerations

### Unused Tokio Feature
Workspace dependency includes `tokio` with `["full"]` features enabled in `/Cargo.toml` but neither CLI nor TUI imports or uses tokio.

**Impact:** Unnecessary binary bloat and dependency surface area.

### No Input Validation
`assay-types` Spec, Gate, Review, and Workflow structs accept arbitrary strings with no validation constraints (name, description fields are unbounded `String`).

**Impact:** Serialization/deserialization could accept malformed or adversarial data without constraints.

### Missing Error Handling in TUI
`/crates/assay-tui/src/main.rs` unwraps IO operations (`event::read()`, `terminal.draw()`) without graceful error recovery.

**Impact:** Terminal state corruption or panic on I/O failures.

## Performance Bottlenecks

None identified. Codebase is too early-stage to have performance concerns. Collection types are appropriately sized (Vec for specs, gates, workflows).

## Fragile Areas

### Unimplemented Domain Types
`assay-types/src/lib.rs` defines Review.approved as boolean without capturing review state (pending, approved, rejected, or commenting).

**Impact:** Review workflow cannot represent all necessary states; forced into binary decision model.

### Hardcoded Workflow Assumptions
Workflow struct in `assay-types` couples specs and gates at the workflow level, making it difficult to apply gates selectively or compose workflows.

**Impact:** Workflow composition and reuse are constrained.

### No Type-Level Validation
Configuration validation is declared in `/crates/assay-core/src/config/mod.rs` but not implemented. No runtime validation of Workflow dependencies or circular references.

**Impact:** Invalid configurations can load successfully.

## Scaling Limits

### In-Memory Only
No persistence layer, database, or file I/O implementation. Workflows exist only in memory during program execution.

**Impact:** Cannot save/restore state; single-session only.

### No Concurrency Model
`tokio` is imported but unused. No async/await patterns or concurrent workflow execution designed.

**Impact:** Limited to single-threaded, synchronous execution.

### No Event Streaming
No event bus, webhooks, or integration points for external systems (CI/CD, agents, notifications).

**Impact:** Cannot integrate with agentic workflows or larger development ecosystems.

## Dependencies at Risk

### Edition 2024 Stability
`Cargo.toml` specifies `edition = "2024"` which is experimental and not yet stabilized in Rust.

**Impact:** Potential breaking changes or deprecations as the edition evolves.

### Unreleased/Unstable Dependencies
All dependencies are at stable versions (serde 1, clap 4, ratatui 0.30, etc.) but check for MSRV alignment.

**Impact:** Lower risk currently, but Edition 2024 is a potential blocker.

## Missing Critical Features

### No Error Types
`assay-core` imports `thiserror` but defines no error types. Domain errors are not modeled.

**Impact:** Cannot propagate or handle domain-specific errors.

### No Serialization/Deserialization Logic
`assay-types` derives Serde but no code uses it. No from_json, to_json, or file loading implemented.

**Impact:** Cannot load/save configurations or workflows.

### No Agent Integration
Project claims to be "agentic development kit" but no integration with AI agents, LLMs, or agentic patterns exists.

**Impact:** Core mission not realized.

### No Review Feedback Loop
Review comments are stored as Vec<String> but no mechanism to apply feedback or iterate on specs.

**Impact:** Review process is one-directional; cannot close the loop.

### No Gate Implementation
Gates are defined as boolean passed flag. No actual gate logic (test running, linting, build checks) is implemented.

**Impact:** Quality gates are placeholders.

## Test Coverage Gaps

### Zero Tests
No test modules found in any crate. No unit tests, integration tests, or doc tests.

**Impact:** No automated verification of behavior. Cannot confidently refactor or extend.

### No Example Workflows
No example configurations, specs, or workflow definitions to demonstrate intended usage.

**Impact:** New developers cannot understand how to use the system.

---
*Concerns audit: 2026-02-28*
