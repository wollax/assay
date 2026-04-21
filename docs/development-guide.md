# Development Guide

## Prerequisites

| Tool | Purpose | Install |
|------|---------|---------|
| Rust (stable) | Compiler — pinned via `rust-toolchain.toml` | `rustup` or `mise` |
| [mise](https://mise.jdx.dev/) | Toolchain management | `brew install mise` |
| [just](https://github.com/casey/just) | Task runner | `cargo binstall just` |
| [cargo-nextest](https://nexte.st/) | Test runner (replaces `cargo test`) | `cargo binstall cargo-nextest` |
| [cargo-deny](https://embarkstudios.github.io/cargo-deny/) | License/advisory/ban checks | `cargo binstall cargo-deny` |

## Setup

```bash
git clone <repo-url>
cd assay
mise install          # install toolchain versions
just setup            # configure git hooks (.githooks/)
just build            # build all workspace crates
```

## Git Hooks

Hooks live in `.githooks/` and are activated by `just setup` (sets `core.hooksPath`).

| Hook | Checks |
|------|--------|
| **pre-commit** | `fmt-check` + `lint` + `check-plugin-version` |
| **pre-push** | All of the above + `test-assay` + `test-smelt-unit` + `deny` |

Pre-push uses `test-smelt-unit` (not `test-smelt`) so Docker-dependent integration tests don't block pushes on machines without Docker.

## Just Commands

### Workspace-wide

| Command | Description |
|---------|-------------|
| `just build` | Build all crates |
| `just test` | Run all tests (cargo-nextest) |
| `just lint` | Clippy with `-D warnings` |
| `just fmt` | Format all code |
| `just fmt-check` | Check formatting without modifying |
| `just deny` | License/advisory/ban checks |
| `just ready` | Full CI check: fmt-check + lint + test + deny + check-plugin-version |
| `just schemas` | Regenerate JSON schemas from types |
| `just schemas-check` | Verify schemas are up to date |

### Per-project

| Assay | Smelt |
|-------|-------|
| `just build-assay` | `just build-smelt` |
| `just test-assay` | `just test-smelt` |
| `just lint-assay` | `just lint-smelt` |
| `just fmt-assay` | `just fmt-smelt` |
| | `just test-smelt-unit` (excludes Docker integration tests) |

### Other

| Command | Description |
|---------|-------------|
| `just dev` | Watch mode — rebuild on changes |
| `just cli <ARGS>` | Run assay-cli |
| `just tui` | Run assay-tui |
| `just sweep` | Remove build artifacts older than 14 days |
| `just sync-plugin-version` | Sync plugin.json version with workspace version |
| `just check-plugin-version` | Verify plugin version matches workspace |

## Test Runner

The project uses **cargo-nextest**, not `cargo test`. The workspace has ~836 tests.

```bash
just test              # all tests
just test-assay        # assay crates only
just test-smelt        # smelt crates (includes Docker integration tests)
just test-smelt-unit   # smelt crates without Docker integration tests
```

## Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/) with a crate scope:

```
feat(assay-core): add gate composition support
fix(smelt-core): handle container timeout correctly
chore(assay-types): update serde dependency
docs(assay-cli): improve help text
refactor(assay-backends): extract common HTTP client
test(assay-harness): add spec execution edge cases
```

## Coding Rules

### Architecture

- **Sync-first core**: No async in `assay-core` or `assay-types`. Async lives in backends and binary crates.
- **Types in `assay-types`**: All shared serializable types go here.
- **Logic in `assay-core`**: Business/domain logic lives here.
- **Thin binaries**: `assay-cli`, `assay-tui`, `smelt-cli` are thin wrappers that delegate to core crates.

### Error Handling

- `thiserror` in library crates.
- `color-eyre` or `anyhow` in binary crates.
- `#[non_exhaustive]` on all error enums.

### Types

- `deny_unknown_fields` on all config/deserialization types.
- `PartialEq` and `Eq` always together (never `PartialEq` alone).
- Cognitive complexity threshold: **25**.

## Adding Dependencies

1. Add the dependency to `[workspace.dependencies]` in the root `Cargo.toml`.
2. Reference it in the crate's `Cargo.toml` with `{ workspace = true }`.
3. Run `cargo deny check` to verify license/advisory compliance.

```toml
# Root Cargo.toml
[workspace.dependencies]
some-crate = "1.0"

# crates/assay-core/Cargo.toml
[dependencies]
some-crate = { workspace = true }
```

## Adding Crates

1. Create the crate under `crates/` (assay) or `smelt/crates/` (smelt).
2. Workspace members are auto-discovered via `members = ["crates/*", "smelt/crates/*"]`.
3. The crate's `lib.rs` must have a doc comment (`//!` at the top).
4. If it's an assay crate and `just fmt-assay` is used, add the crate to the explicit `-p` list in the justfile (cargo fmt lacks `--exclude`).

## Schema Workflow

Types deriving `JsonSchema` have generated JSON schema files in `schemas/`.

1. Make changes to types in `assay-types`.
2. Run `just schemas` to regenerate.
3. Commit the updated `schemas/*.json` files alongside the code change.

CI runs `just schemas-check` to verify schemas are current.

## Plugin Version Sync

Plugin manifests must match the workspace version in `Cargo.toml`.

After bumping the workspace version:

```bash
just sync-plugin-version   # update plugin.json
just check-plugin-version  # verify (also runs in pre-commit hook)
```

## Working with Local Cupel

To develop against a local checkout of the `cupel` crate:

```toml
# Root Cargo.toml — add temporarily, NEVER commit
[patch.crates-io]
cupel = { path = "../cupel" }
```
