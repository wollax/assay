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

## CI & Release Pipeline

Development happens on Forgejo, with a push-mirror that automatically forwards all branches and tags to GitHub.

- **Forgejo CI** (`.forgejo/workflows/ci.yml`) runs on every push to `main` and every pull request — executes `just ready` (fmt-check, lint, test, deny, check-plugin-version) and validates plugin JSON manifests
- **GitHub CI** (`.github/workflows/ci.yml`) runs on mirrored pushes — same gate checks as Forgejo CI
- **Releases**: push a tag to Forgejo (e.g., `git tag v0.6.0 && git push origin v0.6.0`) → the tag mirrors to GitHub → `release.yml` triggers → multi-platform binaries are built and published as a GitHub Release

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
