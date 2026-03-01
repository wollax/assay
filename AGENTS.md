# Assay — Agent Instructions

You are working within the Assay project, an agentic development kit built in Rust.

## Build Commands

- `just build` — build all crates
- `just test` — run all tests
- `just lint` — run clippy
- `just fmt` — format code
- `just ready` — run all checks (fmt, lint, test, deny)

## Workspace Layout

- `crates/assay-types` — shared serializable types (serde, schemars)
- `crates/assay-core` — domain logic (specs, gates, reviews, workflows)
- `crates/assay-cli` — CLI binary (clap)
- `crates/assay-tui` — TUI binary (ratatui)
- `plugins/` — plugin packages for agentic AI systems

## Conventions

- Lean towards functional and declarative patterns
- Use workspace dependencies defined in the root `Cargo.toml`
- Types shared between crates belong in `assay-types`
- Business logic belongs in `assay-core`
- Binary crates are thin wrappers that delegate to `assay-core`
