# Contribution Guide

## Dev Setup

1. Install [mise](https://mise.jdx.dev/) for toolchain management
2. Clone the repo and run:

```bash
mise install
just build
just setup   # Install git hooks
```

## Prerequisites

- Rust stable channel (managed via `rust-toolchain.toml`)
- `just` — task runner
- `cargo-nextest` — test runner
- `cargo-deny` — license/advisory/ban checks
- `jq` — optional, for plugin hook scripts

## Branch Workflow

- Feature branches off `main`
- Push to Forgejo at `forgejo.alexwollan.com` (SSH port 2222), not GitHub
- No force-push to main

## Commit Conventions

Conventional commits with crate scope:

```
feat(assay-core): add spec validation
fix(smelt-cli): handle missing queue dir
chore: update workspace dependencies
docs(README): clarify setup steps
refactor(assay-mcp): extract handler logic
test(assay-types): add schema roundtrip tests
```

- 72-char subject line, no period
- Breaking changes: `!` suffix and `BREAKING CHANGE:` footer
- MCP tool additions are non-breaking; removals/signature changes are breaking

## Quality Gate

Run before every commit:

```bash
just ready   # fmt-check + lint + test + deny + check-plugin-version
```

Git hooks enforce this:
- **pre-commit**: fmt-check + lint + check-plugin-version
- **pre-push**: full test suite + deny

Never bypass with `--no-verify`.

## Adding Dependencies

1. Add to `[workspace.dependencies]` in root `Cargo.toml`
2. Reference as `{ workspace = true }` in crate `Cargo.toml`
3. Run `cargo deny check`

## Adding Crates

1. Place in `crates/` (assay) or `smelt/crates/` (smelt)
2. Declare in `[workspace.members]` in the same commit
3. All deps via workspace
4. Must have `lib.rs` `//!` doc comment

## Schema Workflow

After changing types with `JsonSchema`:

```bash
just schemas          # Regenerate schemas/*.json
git add schemas/
```

CI enforces via `schemas-check`.

## Plugin Version Sync

After version bumps:

```bash
just sync-plugin-version
```

## Coding Conventions

- **Types** in `assay-types`, **logic** in `assay-core`, **binaries** are thin wrappers
- Sync-first core — no async in `assay-core` or `assay-types`
- `thiserror` in library crates, `color-eyre`/`anyhow` in binary crates
- `#[non_exhaustive]` on all error enums
- `PartialEq` always paired with `Eq`
- `deny_unknown_fields` on config types
- Default to `pub(crate)` visibility
- Cognitive complexity threshold: 25

## Testing

- Test runner: `cargo nextest run` (not `cargo test`)
- Unit tests inline (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directories
- Snapshot testing via `insta` — run `cargo insta review` locally
- Docker tests skip gracefully when Docker unavailable

## Working with Local cupel

```toml
# Add to root Cargo.toml (DO NOT COMMIT)
[patch.crates-io]
cupel = { path = "../cupel/crates/cupel" }
```

## PR Process

1. Create feature branch
2. Implement changes
3. Run `just ready`
4. Push to Forgejo
5. Open pull request against `main`
