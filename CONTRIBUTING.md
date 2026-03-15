# Contributing to Assay

## Dev Setup

1. Install [mise](https://mise.jdx.dev/) if you haven't already
2. Clone the repo and run:

```bash
mise install
just build
```

## Workflow

- Create a branch from `main` with a descriptive name
- Make your changes
- Run `just ready` to verify everything passes
- Open a pull request

## Working with Local cupel

If you're developing both `assay` and `cupel` simultaneously, override the registry dependency
with your local checkout by adding this to assay's root `Cargo.toml`:

```toml
[patch.crates-io]
cupel = { path = "../cupel/crates/cupel" }
```

**Do not commit this change.** The patch is for local development only.

The local `cupel` version must satisfy the workspace version constraint (e.g., `1.x.y`).
Remove the `[patch.crates-io]` section before committing or opening a PR.

## Coding Conventions

- **Rust**: Follow standard Rust idioms. `cargo fmt` and `cargo clippy` are enforced in CI.
- **TypeScript**: Strict mode, ESM output.
- **Commits**: Use conventional commit messages.

## Plugin Development

Each plugin lives in `plugins/`. See the individual plugin READMEs for structure and installation instructions.

- **Claude Code**: `plugins/claude-code/README.md`
- **Codex**: `plugins/codex/README.md`
- **OpenCode**: `plugins/opencode/README.md`
