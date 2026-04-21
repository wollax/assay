# Deployment and CI/CD Guide

## Continuous Integration

CI runs on Forgejo at `.forgejo/workflows/ci.yml`. All jobs are **path-filtered** -- they only trigger when relevant files change (crates, Cargo.toml/lock, deny.toml, justfile, rust-toolchain.toml, .cargo, .forgejo, plugins).

Concurrent runs on the same branch are cancelled automatically.

### Jobs

#### check-assay

Runs the full `just ready` pipeline (fmt-check + lint + test + deny + check-plugin-version) against all assay crates.

- Runner: `ubuntu-latest`
- Toolchain: Rust stable
- Tools installed via `cargo-binstall`: just, cargo-deny, cargo-nextest

#### check-smelt

Runs targeted checks for smelt crates:

```
fmt-check -> test-smelt -> lint-smelt -> cargo deny check
```

Same runner and toolchain as check-assay.

#### plugins

Validates plugin JSON manifests using Node.js (`JSON.parse` on each manifest file). Lightweight job with a 10-minute timeout.

### Failure Alerts

All three jobs send an [ntfy](https://ntfy.sh/) push notification on failure. Alerts are non-fatal -- if ntfy is unreachable, CI still reports the correct exit code.

## Releases

Releases use a two-stage flow across Forgejo and GitHub.

### Flow

1. **Tag push on Forgejo** -- push a `v*` tag (e.g., `v0.5.0`).
2. **Mirror to GitHub** -- Forgejo mirrors the tag to the GitHub repository.
3. **GitHub Actions `release.yml`** -- triggered by the tag, builds release binaries.
4. **GitHub Release** -- created automatically with generated release notes and binary artifacts.

### Build Matrix

The release workflow builds for 4 targets:

| Target | Runner |
|--------|--------|
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` (cross-compiled) |
| `x86_64-apple-darwin` | `macos-latest` |
| `aarch64-apple-darwin` | `macos-latest` |

Artifacts produced: `assay-cli-<target>` and `assay-tui-<target>`.

## Installation

### From source

```bash
# assay CLI
cargo install --path crates/assay-cli

# smelt CLI
cargo install --path smelt/crates/smelt-cli
```

### From GitHub Release

Download the appropriate binary for your platform from the [Releases](https://github.com/wollax/assay/releases) page and place it on your `PATH`.

## Plugin Installation

Plugins live in `plugins/` and provide integration with agentic AI systems.

### Claude Code

```bash
claude plugin add plugins/claude-code
```

### Codex

Symlink the plugin directory into your Codex plugins path:

```bash
ln -s "$(pwd)/plugins/codex" ~/.codex/plugins/codex-assay
```

### OpenCode

Register the plugin in your OpenCode configuration file, pointing to the plugin directory:

```bash
plugins/opencode/
```
