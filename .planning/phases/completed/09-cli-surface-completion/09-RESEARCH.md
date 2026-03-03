# Phase 9: CLI Surface Completion — Research

**Researched:** 2026-03-02
**Confidence:** HIGH unless noted

## Standard Stack

No new dependencies required. Everything needed is already in the workspace:

| Concern | Solution | Notes |
|---------|----------|-------|
| CLI parsing & help | `clap 4` (derive) | Already wired; just needs attribute enrichment |
| JSON output | `serde_json 1` | Already a dependency of assay-cli |
| Color support | Raw ANSI escape codes | Existing pattern; NO_COLOR respected |
| Error types | `thiserror 2` via `AssayError` | Existing pattern in assay-core |
| Version info | `env!("CARGO_PKG_VERSION")` | Already used in `None` arm of main match |
| Config loading | `assay_core::config::load()` | Existing; needed for status display |
| Spec scanning | `assay_core::spec::scan()` | Existing; needed for status display |

**No new crates needed.** Phase 9 is purely additive refinement of existing code.

## Architecture Patterns

### 1. Clap Derive Help Enrichment

**Confidence: HIGH** (verified via Context7 clap docs)

Clap 4 derive supports these attributes for rich help text:

```rust
#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows",
    long_about = "Agentic development kit with spec-driven workflows.\n\n\
        Assay combines spec-driven development with gated quality checks.\n\
        Use it from the command line or connect via MCP for agent integration.",
    after_long_help = "EXAMPLES:\n  ...",
)]
struct Cli { ... }
```

Key attributes available:
- `about` — short description shown in parent help listings (already set)
- `long_about` — extended description shown in `--help` (currently unset; defaults to `about`)
- `after_help` — text after short help (`-h`)
- `after_long_help` — text after long help (`--help`), ideal for examples
- `before_help` — text before help output
- Doc comments on structs/variants become `about`/`long_about` automatically (first paragraph = about, rest = long_about)

For subcommands, the `///` doc comment on enum variants becomes the `about` text shown in the parent `--help`. Additional `#[command()]` attributes can be added to subcommand variants for `after_long_help` etc.

**Pattern for examples in derive mode:**

```rust
#[derive(Subcommand)]
enum Command {
    /// Initialize a new Assay project in the current directory
    #[command(after_long_help = "\
EXAMPLES:
  $ assay init
  $ assay init --name my-project")]
    Init { ... },
}
```

Clap 4 renders `after_long_help` only on `--help` (long form), not on `-h` (short form). This is the right place for examples — they expand the help without cluttering the brief view.

### 2. Bare Invocation: Status Display vs Help

**Confidence: HIGH** (codebase inspection)

Current behavior: `None` arm in main match prints `assay 0.1.0`. The `Command` field is `Option<Command>`, so bare `assay` (no args) falls through to `None`.

Desired behavior per CONTEXT.md:
- In an initialized project (`.assay/` exists): show status summary
- Outside a project: show hint + help

Implementation pattern:

```rust
None => {
    let root = std::env::current_dir().unwrap_or_else(|e| { ... });
    if root.join(".assay").exists() {
        show_status(&root);
    } else {
        // Not an assay project
        eprintln!("Not an Assay project. Run `assay init` to get started.\n");
        // Print help via clap
        Cli::command().print_help().unwrap();
    }
}
```

**Status display data sources** (all available without new core APIs):
- `assay_core::config::load(&root)` — project name
- `assay_core::spec::scan(&specs_dir)` — list of specs with criteria counts
- `.assay/` existence — project detection

Status display should be lightweight (no gate execution, just config + spec inventory).

### 3. plugin.json Version Sync

**Confidence: HIGH** (codebase inspection)

The existing `plugins/claude-code/.claude-plugin/plugin.json` has:
```json
{
  "name": "assay",
  "version": "0.1.0",
  "description": "Assay plugin for Claude Code — spec-driven workflows with gated quality checks",
  "author": "wollax"
}
```

CONTEXT.md says:
- Description should be "Agentic development kit with spec-driven workflows" (match CLI about text)
- Add `homepage` and `license` fields
- Version must auto-sync from Cargo.toml workspace version

For auto-sync, two approaches:
1. **Just recipe** — `just sync-plugin-version` reads workspace Cargo.toml version and patches plugin.json
2. **Build-time** — build.rs writes version, but adds complexity for a static manifest

Recommendation: **Just recipe** is simpler and matches the existing `just schemas` pattern. A `just ready` addition ensures CI catches drift.

The workspace version is in `Cargo.toml` root: `version = "0.1.0"` under `[workspace.package]`.

### 4. Error Message Consistency

**Confidence: HIGH** (codebase inspection)

Current error pattern in main.rs (all 16 exit points):
```
eprintln!("Error: {e}");
std::process::exit(1);
```

The prefix is already consistently `Error:` across all handlers. This is a clean, readable convention that matches Rust ecosystem norms (cargo uses `error:` lowercase, rustc uses `error[E0xxx]:` with codes).

**Existing issues to address** (from open issue `2026-03-01-cli-spec-cleanup.md`):
1. `NO_COLOR` should use `var_os().is_none()` not `var().is_err()` (item 4)
2. MCP error uses `{e:?}` (Debug) while all others use `{e}` (Display) (item 5)
3. Init arm calls `current_dir()` directly instead of `project_root()` (item 6)
4. ANSI escape byte count magic number should be a named constant (item 1)
5. Spec list alignment inconsistent for mixed description/no-description specs (item 7)

**Exit codes used currently:**
- `0` — success (implicit)
- `1` — all errors (explicit `process::exit(1)`)

The gate run handler also exits `1` when any gate fails (line 432 and 349).

### 5. JSON Error Formatting

**Confidence: MEDIUM** (design decision, no existing pattern)

When `--json` is active and an error occurs, the error should also be JSON. The current pattern just uses `eprintln!("Error: ...")` even in JSON mode. This breaks machine parsing.

Recommended pattern:

```rust
if json {
    let err_json = serde_json::json!({
        "error": true,
        "message": format!("{e}")
    });
    eprintln!("{}", serde_json::to_string(&err_json).unwrap());
    std::process::exit(1);
}
```

This keeps error output on stderr (where agents/CI can distinguish it from stdout data) while making it parseable.

## Don't Hand-Roll

| Problem | Use Instead | Why |
|---------|-------------|-----|
| Help text formatting | clap `#[command()]` attributes | clap handles wrapping, alignment, terminal width |
| Version display | clap `version` attribute + `env!("CARGO_PKG_VERSION")` | Already works; clap handles `--version` flag |
| ANSI color in help text | clap's built-in styling OR raw `\x1b` in `after_help` | clap respects `NO_COLOR` for its own output; keep custom ANSI for app output only |
| plugin.json generation | Simple `just` recipe with `sed`/`jq` or inline script | Don't build Rust tooling for manifest file updates |
| Terminal width detection | clap handles this internally | Don't add `terminal_size` crate |

## Common Pitfalls

### 1. `after_help` vs `after_long_help` Confusion

**Pitfall:** Using `after_help` puts examples in BOTH `-h` and `--help` output. The short help (`-h`) should be concise.

**Fix:** Use `after_long_help` for examples, keep `after_help` empty or very brief.

### 2. Binary Name in Help Output

**Pitfall:** The current help shows `assay-cli` as the binary name (`Usage: assay-cli [COMMAND]`). This is because the Cargo.toml package name is `assay-cli` and there's no `[[bin]]` section overriding the name.

**Fix:** Add `[[bin]]` section to `crates/assay-cli/Cargo.toml`:
```toml
[[bin]]
name = "assay"
path = "src/main.rs"
```

OR rely on the `#[command(name = "assay")]` which is already set. The `name` attribute in clap controls the display name in help text. Current code already has `name = "assay"` on the Parser struct. However, `cargo run -p assay-cli` still shows `assay-cli` because cargo overrides the binary name. When installed via `cargo install`, the binary name comes from the `[[bin]]` section.

**Verdict:** Add `[[bin]] name = "assay"` to Cargo.toml to ensure the installed binary is called `assay`, not `assay-cli`. The `#[command(name = "assay")]` already handles help text display.

### 3. `long_about = None` to Suppress Doc Comment Expansion

**Pitfall:** If you add a multi-line doc comment to the Cli struct, clap automatically uses the full comment as `long_about`. If you only want the first line in both `-h` and `--help`, set `long_about = None`.

**Fix:** Be explicit: use `long_about` attribute when you want extended text, set `long_about = None` when you don't.

### 4. Status Display Performance

**Pitfall:** Bare `assay` invocation should be fast. Don't run gate evaluation or parse all spec files deeply.

**Fix:** `config::load()` + `spec::scan()` are both fast (filesystem reads only, no command execution). The status display should use these and nothing more.

### 5. Error Prefix When NO_COLOR Is Active

**Pitfall:** If adding color to the "Error:" prefix (e.g., red "error:"), the NO_COLOR handling must apply there too.

**Fix:** Use the existing `colors_enabled()` helper consistently. Note the open issue: it should be `var_os().is_none()` not `var().is_err()`.

### 6. plugin.json Version Drift

**Pitfall:** plugin.json version gets out of sync with Cargo.toml workspace version.

**Fix:** Add version sync check to `just ready` or `just deny` step. A simple `just` recipe that compares the two and fails if different.

### 7. Exit Code After Printing Help

**Pitfall:** When printing help on bare invocation outside a project, `Cli::command().print_help()` returns `Result<()>`. The process should exit 0 (help was requested implicitly), not 1.

**Fix:** Bare invocation outside a project should exit 0 after showing the hint + help. Only error conditions should exit 1.

## Code Examples

### Rich Help with Examples (Derive)

```rust
#[derive(Parser)]
#[command(
    name = "assay",
    version,
    about = "Agentic development kit with spec-driven workflows",
    long_about = None,
    after_long_help = "\
Examples:
  Initialize a new project:
    $ assay init
    $ assay init --name my-project

  View specs and run gates:
    $ assay spec list
    $ assay spec show auth-flow
    $ assay gate run auth-flow --verbose

  Machine-readable output (for agents/CI):
    $ assay spec show auth-flow --json
    $ assay gate run auth-flow --json

  Start the MCP server:
    $ assay mcp serve"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}
```

### Subcommand Help with Examples

```rust
#[derive(Subcommand)]
enum Command {
    /// Initialize a new Assay project in the current directory
    #[command(after_long_help = "\
Examples:
  $ assay init                    # Use directory name as project name
  $ assay init --name my-project  # Override project name")]
    Init {
        /// Override the inferred project name
        #[arg(long)]
        name: Option<String>,
    },

    /// Run quality gates for a spec
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
}
```

### Bare Invocation Status Display

```rust
fn show_status(root: &std::path::Path) {
    let color = colors_enabled();

    let config = match assay_core::config::load(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let version = env!("CARGO_PKG_VERSION");
    println!("assay {version} — {}", config.project_name);
    println!();

    let specs_dir = root.join(".assay").join(&config.specs_dir);
    match assay_core::spec::scan(&specs_dir) {
        Ok(result) => {
            if result.specs.is_empty() {
                println!("  No specs found. Create one in {}", config.specs_dir);
            } else {
                println!("  Specs:");
                for (slug, spec) in &result.specs {
                    let criteria_count = spec.criteria.len();
                    let executable = spec.criteria.iter().filter(|c| c.cmd.is_some()).count();
                    println!("    {slug}  ({executable}/{criteria_count} executable)");
                }
            }
        }
        Err(e) => {
            eprintln!("  warning: could not scan specs: {e}");
        }
    }
}
```

### plugin.json Final Schema

```json
{
  "name": "assay",
  "version": "0.1.0",
  "description": "Agentic development kit with spec-driven workflows",
  "author": "wollax",
  "homepage": "https://github.com/wollax/assay",
  "license": "MIT"
}
```

### Just Recipe for Version Sync

```just
# Sync plugin.json version with workspace version
sync-plugin-version:
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    for f in plugins/claude-code/.claude-plugin/plugin.json; do
        if [ -f "$f" ]; then
            # Use a temp file for portable sed
            jq --arg v "$version" '.version = $v' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
        fi
    done
    echo "Plugin versions synced to $version"
```

**Note:** Requires `jq`. If `jq` isn't available, a simple `sed` pattern works:
```bash
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$version\"/" "$f"
```

## Open Issues Relevant to Phase 9

These existing open issues should be addressed during Phase 9 since they directly concern CLI surface quality:

1. **`2026-03-01-cli-spec-cleanup.md`** — 7 items including NO_COLOR fix, ANSI magic number, MCP error format, init code duplication, spec list alignment
2. **`2026-03-01-cli-error-propagation.md`** — main() should return Result (color-eyre is workspace dep)

The error propagation issue suggests converting to `color_eyre::Result<()>` in main. However, this changes the error display format and adds a dependency on color-eyre's formatting. Given Phase 9's scope (polish, not restructure), the simpler approach is to keep the current `eprintln!` + `process::exit(1)` pattern and defer the color-eyre migration. The current pattern is explicit and predictable.

**Recommendation:** Fix the 7 items from cli-spec-cleanup (they're small, targeted fixes). Defer the color-eyre migration to a future phase.

## Scope Boundaries

### In Scope
- Enrich help text with `after_long_help` examples on all commands and subcommands
- Fix binary name (`[[bin]] name = "assay"`)
- Bare `assay` status display (in-project) and hint+help (outside project)
- plugin.json metadata update (description, homepage, license)
- plugin.json version sync recipe
- Fix known CLI issues (NO_COLOR, ANSI constant, MCP error format, init helper, spec list alignment)
- Consistent error prefix
- Exit code documentation/consistency

### Out of Scope
- Refactoring main() to return Result (separate issue)
- Shell completions (future feature)
- Color theme customization
- Man page generation
- New CLI subcommands

---

*Phase: 09-cli-surface-completion*
*Researched: 2026-03-02*
