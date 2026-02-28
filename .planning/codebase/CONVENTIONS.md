# Coding Conventions

**Analysis Date:** 2026-02-28

## Naming Patterns

- **Structs**: PascalCase (e.g., `Spec`, `Gate`, `Review`, `Workflow`, `Config`)
- **Fields**: snake_case (e.g., `spec_name`, `project_name`, `approved`, `comments`)
- **Modules**: lowercase snake_case matching domain areas (e.g., `spec`, `gate`, `review`, `workflow`, `config`)
- **Binary crates**: Use lowercase with hyphens (e.g., `assay-cli`, `assay-tui`, `assay-types`, `assay-core`)
- **Variables and functions**: snake_case (e.g., `run()`, `_cli`)

Follow standard Rust naming conventions with no deviations. See `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs` for type naming patterns.

## Code Style

- **Edition**: Use Rust 2024 edition (configured in `rustfmt.toml` and workspace `Cargo.toml`)
- **Clippy**: Enforce all clippy warnings as errors (`cargo clippy` is enforced in CI with `-D warnings`)
- **Cognitive Complexity**: Maximum threshold of 25 per `clippy.toml`
- **Formatting**: Use `cargo fmt` (enforced by `rustfmt.toml`)
- **Derives**: Use common derives for data structures:
  - `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]` for types in `assay-types`
  - Add `#[derive(...)]` attributes on structs for serde and schema support
  - See `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs` for examples

## Import Organization

- Use grouped imports with blank lines separating external and internal crates
- External workspace imports first, then local imports
- Example from `/Users/wollax/Git/personal/assay/crates/assay-types/src/lib.rs`:
  ```rust
  use schemars::JsonSchema;
  use serde::{Deserialize, Serialize};
  ```
- Use explicit imports rather than glob imports (e.g., `use crossterm::event::{self, Event, KeyCode}`)
- For re-exports, use shorthand: `use crossterm::event::{self, ...}` pattern

## Error Handling

- Use `Result<T>` type from standard library for fallible operations
- Leverage `thiserror` crate (in workspace dependencies) for custom error types
- Use `color-eyre` for error handling in binaries (see `/Users/wollax/Git/personal/assay/crates/assay-tui/src/main.rs`):
  ```rust
  fn main() -> color_eyre::Result<()> {
      color_eyre::install()?;
      // ... operation ...
  }
  ```
- Return `Result` from fallible operations; use `?` operator for propagation
- No panics in library code; use `Result` types instead

## Logging

- Not yet implemented in codebase (no logging infrastructure present)
- When logging is needed, use standard Rust logging patterns with `log` or `tracing` crate
- Document logging requirements in module-level doc comments

## Comments

- **Module documentation**: Use `//!` for module-level docs at the start of each module file
- **Item documentation**: Use `///` for public items (currently not visible in minimal codebase)
- Module patterns from `/Users/wollax/Git/personal/assay/crates/assay-core/src/spec/mod.rs`:
  ```rust
  //! Spec authoring and validation.
  //!
  //! Handles creating, parsing, and validating specifications
  //! that define what should be built and their acceptance criteria.
  ```
- Keep docs concise with one-line summary followed by blank line and details
- Prefer self-documenting code over comments for implementation details

## Function Design

- Keep functions small and focused (cognitive complexity threshold of 25)
- Use declarative and functional patterns first, then object-oriented for brownfield/complex tasks (per CLAUDE.md)
- Use descriptive parameter and return types
- Prefer `Result<T>` return types for fallible operations
- Use minimal function signatures; avoid many parameters
- Example from `/Users/wollax/Git/personal/assay/crates/assay-tui/src/main.rs`:
  ```rust
  fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
      // loop-based implementation
  }
  ```

## Module Design

- **Workspace structure**: Follow dependency graph in CLAUDE.md:
  - `assay-types`: Serializable types only, no business logic
  - `assay-core`: Domain logic (specs, gates, reviews, workflows)
  - `assay-cli`, `assay-tui`: Thin binary wrappers delegating to `assay-core`
- **Module organization**: Create subdirectories for domain areas (`spec/`, `gate/`, `review/`, `workflow/`, `config/`)
- **Public API**: Only expose necessary public items; use `pub` strategically
- **Dependencies**: Add all workspace dependencies to root `Cargo.toml`, never to individual crates directly
- Use workspace package metadata (version, edition, license, repository) shared across all crates

---
*Convention analysis: 2026-02-28*
