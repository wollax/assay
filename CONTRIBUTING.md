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

## Coding Conventions

- **Rust**: Follow standard Rust idioms. `cargo fmt` and `cargo clippy` are enforced in CI.
- **TypeScript**: Strict mode, ESM output.
- **Commits**: Use conventional commit messages.

## Plugin Development

Each plugin lives in `plugins/`. See the individual plugin READMEs for structure and installation instructions.

- **Claude Code**: `plugins/claude-code/README.md`
- **Codex**: `plugins/codex/README.md`
- **OpenCode**: `plugins/opencode/README.md`
