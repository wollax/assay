# Phase 5: Config and Initialization - Research

**Researched:** 2026-03-01
**Domain:** TOML config loading/validation, CLI init commands, template generation, multi-error collection in Rust
**Confidence:** HIGH

## Summary

Phase 5 introduces the `assay init` CLI command and config loading/validation in `assay-core`. The technical challenges are well-understood: TOML parsing with good error messages (toml 0.8 provides line/column spans), rejecting unknown fields via `#[serde(deny_unknown_fields)]`, generating commented template files (string templates, not `toml::to_string`), collecting multiple validation errors, and adding new `AssayError` variants for config/init failures.

The existing `Config` type in `assay-types` (`project_name` + `workflows: Vec<Workflow>`) must be redesigned to match the Phase 5 schema: `project_name`, `specs_dir`, and an optional `[gates]` table. This is a breaking change to the existing type, but no downstream consumers depend on the current shape yet (all modules are stubs). The existing schema snapshot and roundtrip tests for `Config` must be updated.

**Primary recommendation:** Use string templates (not `toml::to_string`) for all generated files because the CONTEXT.md requires TOML comments throughout. Use `#[serde(deny_unknown_fields)]` for unknown key rejection. Collect validation errors into a `Vec<ConfigError>` and report all at once. Keep the init logic in `assay-core` with the CLI as a thin wrapper.

## Standard Stack

### Core (already in workspace)

| Library   | Version | Purpose                                    | Notes                                                    |
| --------- | ------- | ------------------------------------------ | -------------------------------------------------------- |
| toml      | 0.8     | Config/spec TOML parsing                   | Workspace dep; add to assay-core `[dependencies]`        |
| serde     | 1       | Derive Deserialize on config types         | Already in assay-types                                   |
| schemars  | 1       | JsonSchema derivation on config type       | Already in assay-types                                   |
| thiserror | 2       | New AssayError variants                    | Already in assay-core                                    |
| clap      | 4       | `Init` subcommand with `--name` flag       | Already in assay-cli                                     |

### No New Dependencies Required

Everything needed is already in the workspace. `toml` just needs to be added to assay-core's `[dependencies]` section (currently only in assay-types as a dev-dep).

## Architecture Patterns

### Pattern 1: Config Type in assay-types (DTO)

The `Config` type lives in `assay-types` as a pure DTO. It replaces the current placeholder `Config` that has `project_name` and `workflows`.

```rust
// crates/assay-types/src/lib.rs (or a new config.rs module)
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Top-level configuration for an Assay project.
///
/// Loaded from `.assay/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Project name (required, non-empty after trim).
    pub project_name: String,

    /// Directory containing spec files, relative to `.assay/`.
    /// Defaults to `"specs/"`.
    #[serde(default = "default_specs_dir")]
    pub specs_dir: String,

    /// Gate execution configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<GatesConfig>,
}

/// Gate execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GatesConfig {
    /// Default timeout for gate commands in seconds.
    /// Defaults to 300.
    #[serde(default = "default_timeout")]
    pub default_timeout: u64,

    /// Working directory for gate execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
}

fn default_specs_dir() -> String {
    "specs/".to_string()
}

fn default_timeout() -> u64 {
    300
}
```

**Why `deny_unknown_fields`:** CONTEXT.md locks "strict validation — unknown keys rejected." This attribute is safe here because the `Config` struct uses no `#[serde(tag)]`, no `#[serde(flatten)]`, and no `#[serde(skip)]` — all known incompatibilities with `deny_unknown_fields` are absent.

**Why `GatesConfig` is `Option<GatesConfig>`:** The `[gates]` section is entirely commented-out in the template. When absent from TOML, it deserializes as `None`. When present, all its fields have serde defaults.

### Pattern 2: Config Loading in assay-core (Free Functions)

```rust
// crates/assay-core/src/config/mod.rs

/// Parse a config from a TOML string. No validation.
pub fn from_str(s: &str) -> Result<Config> { ... }

/// Load and validate a config from `.assay/config.toml` relative to `root`.
pub fn load(root: &Path) -> Result<Config> { ... }

/// Validate a parsed config. Returns all errors at once.
pub fn validate(config: &Config) -> std::result::Result<(), Vec<ConfigError>> { ... }
```

**Key design:** `from_str()` parses only (for tests/tools). `load()` parses AND validates. `validate()` is separate and returns `Vec<ConfigError>` for multi-error collection.

### Pattern 3: Init Logic in assay-core (Not CLI)

```rust
// crates/assay-core/src/init.rs (or config/init.rs)

/// Options for project initialization.
pub struct InitOptions {
    /// Override the project name (otherwise inferred from directory name).
    pub name: Option<String>,
}

/// Initialize an Assay project at the given root directory.
///
/// Creates `.assay/config.toml`, `.assay/specs/`, `.assay/.gitignore`,
/// and an example spec file.
pub fn init(root: &Path, options: &InitOptions) -> Result<InitResult> { ... }

/// Summary of what was created during initialization.
pub struct InitResult {
    pub project_name: String,
    pub created_files: Vec<PathBuf>,
}
```

**Why in assay-core:** The CLI is a thin wrapper. MCP tools may also need to trigger init in the future. Business logic belongs in core.

### Pattern 4: CLI Init Subcommand (Thin Wrapper)

```rust
// In assay-cli's Command enum
#[derive(Subcommand)]
enum Command {
    /// Initialize a new Assay project in the current directory
    Init {
        /// Override the inferred project name
        #[arg(long)]
        name: Option<String>,
    },
    // ... existing commands
}
```

### Pattern 5: String Templates for Generated Files

Since the CONTEXT.md requires TOML comments throughout the generated config and example spec, `toml::to_string()` cannot be used (it does not support comments). Use string templates instead.

```rust
/// Generate config.toml content from a project name.
fn render_config_template(project_name: &str) -> String {
    format!(
        r#"# Assay project configuration
# See: https://assay.dev/docs/config

project_name = "{project_name}"

# Directory containing spec files (relative to .assay/)
specs_dir = "specs/"

# Gate execution configuration
# [gates]
# default_timeout = 300
# working_dir = "."
"#
    )
}
```

**Why not `include_str!`:** The project name is dynamic, so `format!` with an inline template is simpler than a separate template file with placeholder substitution.

### Pattern 6: Multi-Error Validation Collection

```rust
/// A single validation issue in a config file.
#[derive(Debug)]
pub struct ConfigError {
    /// The field path (e.g., "project_name", "[gates].default_timeout").
    pub field: String,
    /// What's wrong.
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

pub fn validate(config: &Config) -> std::result::Result<(), Vec<ConfigError>> {
    let mut errors = Vec::new();

    if config.project_name.trim().is_empty() {
        errors.push(ConfigError {
            field: "project_name".into(),
            message: "required, must not be empty".into(),
        });
    }

    if config.specs_dir.trim().is_empty() {
        errors.push(ConfigError {
            field: "specs_dir".into(),
            message: "required, must not be empty".into(),
        });
    }

    if let Some(gates) = &config.gates {
        if gates.default_timeout == 0 {
            errors.push(ConfigError {
                field: "[gates].default_timeout".into(),
                message: "must be a positive integer".into(),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

**Why `Vec<ConfigError>` not a new `AssayError` variant directly:** The validate function collects ALL issues. The caller (e.g., `load()`) wraps the Vec into an `AssayError::ConfigValidation` variant for propagation. This separates the "collect errors" concern from the "report errors" concern.

### Pattern 7: New AssayError Variants

```rust
// Added to crates/assay-core/src/error.rs
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssayError {
    // ... existing Io variant ...

    /// Config file parsing failed (invalid TOML or schema mismatch).
    #[error("parsing config `{path}`: {message}")]
    ConfigParse {
        path: PathBuf,
        message: String,
    },

    /// Config validation failed (structurally valid TOML but semantically invalid).
    #[error("invalid config `{path}`:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
    ConfigValidation {
        path: PathBuf,
        errors: Vec<ConfigError>,
    },

    /// Init refused because .assay/ already exists.
    #[error(".assay/ already exists. Remove it first to reinitialize.")]
    AlreadyInitialized,
}
```

**Why three variants, not one:** Each variant has a different structure and different downstream handling. `ConfigParse` wraps toml deserialization errors. `ConfigValidation` carries multiple semantic errors. `AlreadyInitialized` is a simple sentinel.

### Anti-Patterns to Avoid

1. **Using `toml::to_string()` for template generation.** It produces valid TOML but cannot emit comments. The CONTEXT.md requires commented templates. Use string templates.
2. **Using `create_dir_all` for `.assay/`.** The requirement says "refuse to overwrite existing". `create_dir_all` silently succeeds on existing directories. Use `create_dir` for `.assay/` (returns `AlreadyExists` error), then `create_dir` or `create_dir_all` for subdirectories.
3. **Fail-fast validation.** CONTEXT.md locks "collect all errors — report every validation issue at once." Do not return on first error.
4. **Putting init logic in the CLI binary.** Business logic goes in assay-core. The CLI just parses args and calls core functions.
5. **Adding `#[serde(flatten)]` on GatesConfig.** Flatten is incompatible with `deny_unknown_fields`. Use a named `gates` field.

## Don't Hand-Roll

| Problem                          | Don't Build                        | Use Instead                                         | Why                                                                               |
| -------------------------------- | ---------------------------------- | --------------------------------------------------- | --------------------------------------------------------------------------------- |
| TOML parsing                     | Manual parser                      | `toml::from_str::<Config>()`                        | toml crate provides line/column error spans automatically                         |
| Unknown field rejection          | Manual key iteration               | `#[serde(deny_unknown_fields)]`                     | Serde attribute; toml crate's deserializer validates struct keys                  |
| Default field values             | Manual post-parse fixup            | `#[serde(default = "fn_name")]`                     | Serde handles absent fields during deserialization                                |
| Error display for parse failures | Custom error formatting            | `toml::de::Error` Display impl                      | Includes line number, column, caret pointing to error location                    |
| CLI arg parsing for init         | Manual arg parsing                 | clap `#[derive(Subcommand)]` + `#[arg(long)]`      | Already established pattern in assay-cli                                          |
| Directory existence check        | Manual `fs::metadata` + match      | `fs::create_dir()` + match on `ErrorKind::AlreadyExists` | Single atomic operation instead of check-then-create race                    |

## Common Pitfalls

### Pitfall 1: `create_dir_all` Silently Succeeds on Existing Directory

**What goes wrong:** Using `create_dir_all(".assay")` does not return an error if `.assay/` already exists, violating CFG-04 (idempotent init — refuse to overwrite).

**Why it happens:** `create_dir_all` is designed to be idempotent by definition. It succeeds if the directory already exists.

**How to avoid:** Use `std::fs::create_dir()` for the top-level `.assay/` directory. It returns `io::ErrorKind::AlreadyExists` when the directory exists. After confirming `.assay/` was freshly created, use `create_dir` (or `create_dir_all`) for subdirectories like `specs/`.

```rust
match std::fs::create_dir(&assay_dir) {
    Ok(()) => { /* proceed */ },
    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
        return Err(AssayError::AlreadyInitialized);
    },
    Err(e) => {
        return Err(AssayError::Io {
            operation: "creating .assay directory".into(),
            path: assay_dir,
            source: e,
        });
    }
}
```

**Warning signs:** Running `assay init` twice succeeds silently or overwrites config.

### Pitfall 2: `deny_unknown_fields` Conflicts with Tagged Enums or Flatten

**What goes wrong:** `#[serde(deny_unknown_fields)]` combined with `#[serde(tag = "...")]` or `#[serde(flatten)]` causes legitimate fields to be rejected as unknown.

**Why it happens:** Serde's `deny_unknown_fields` implementation has documented incompatibilities with tagged enums (the tag field itself is seen as "unknown") and flattened structs.

**How to avoid:** The Phase 5 `Config` and `GatesConfig` structs are plain structs with no tagged enums and no flattened fields. `deny_unknown_fields` is safe. If a future phase adds tagged enums to config, this would need revisiting. Do NOT add `#[serde(flatten)]` to embed GatesConfig fields at the top level.

**Warning signs:** Deserialization errors on valid TOML like `unknown field "kind"`.

### Pitfall 3: toml::from_str Errors Lack File Path Context

**What goes wrong:** `toml::from_str()` errors include line/column information from the TOML content but not which file was being parsed.

**Why it happens:** `toml::from_str` only sees the string, not the filename. The `toml::de::Error` Display shows "TOML parse error at line X, column Y" but not the filepath.

**How to avoid:** Wrap the toml error in `AssayError::ConfigParse` which adds the file path:

```rust
pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(".assay/config.toml");
    let content = std::fs::read_to_string(&path).map_err(|source| AssayError::Io {
        operation: "reading config".into(),
        path: path.clone(),
        source,
    })?;
    let config: Config = toml::from_str(&content).map_err(|e| AssayError::ConfigParse {
        path: path.clone(),
        message: e.to_string(),
    })?;
    // validate...
    Ok(config)
}
```

**Warning signs:** Error messages like "TOML parse error at line 3, column 1" without any file path.

### Pitfall 4: Directory Name Sanitization Edge Cases

**What goes wrong:** Inferring `project_name` from `std::env::current_dir()` or a provided `Path` can produce names with special characters, leading dots, spaces, or empty strings (e.g., root directory `/`).

**Why it happens:** Unix directory names have very few restrictions. Names like `.hidden`, `my project`, or paths like `/` produce unexpected `project_name` values.

**How to avoid:** Apply sanitization when inferring from directory:
- Use `Path::file_name()` to get the leaf directory name (returns `None` for `/` or `..`)
- Convert to string with `.to_string_lossy()`
- Fallback to `"assay-project"` if the result is empty or is only whitespace
- Do NOT strip special characters aggressively — users can override with `--name`

```rust
fn infer_project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or("assay-project")
        .to_string()
}
```

**Warning signs:** Panic on `unwrap()` when parsing root directory path. Empty `project_name` passing validation.

### Pitfall 5: Config Type Redesign Breaks Existing Snapshots/Tests

**What goes wrong:** Changing the `Config` struct in `assay-types` from `{project_name, workflows}` to `{project_name, specs_dir, gates}` breaks the existing schema snapshot test, schema roundtrip test, and the generated `config.schema.json` file.

**Why it happens:** Phase 4 established schema snapshot and roundtrip tests for the current `Config` shape.

**How to avoid:** Update all three artifacts in the same plan task:
1. Update the `Config` struct in `assay-types/src/lib.rs`
2. Update `schema_roundtrip.rs` — `config_validates()` test
3. Run `cargo insta review` to accept the new `config-schema` snapshot
4. Run `just schemas` to regenerate `schemas/config.schema.json`

**Warning signs:** `just ready` fails on snapshot mismatch or schema-check.

### Pitfall 6: validate() Error Format Doesn't Match CONTEXT.md

**What goes wrong:** CONTEXT.md specifies error format: `.assay/config.toml: [gates].default_timeout: expected positive integer, got "abc"`. If `validate()` only returns field-level errors without the file path prefix, the CLI has to reconstruct the full format.

**Why it happens:** `validate()` takes a `&Config` (already parsed), so it doesn't know the file path.

**How to avoid:** Keep `validate()` returning field-path + message pairs. The `load()` function (which knows the file path) wraps them in `AssayError::ConfigValidation { path, errors }`. The error's Display impl combines path + field errors into the required format. This separates concerns: validate doesn't need IO awareness, load provides file context.

## Code Examples

### Complete Config Template (String)

```rust
fn render_config_template(project_name: &str) -> String {
    format!(
        r#"# Assay project configuration
# Documentation: https://assay.dev/docs/config

# Project name (required)
project_name = "{project_name}"

# Directory containing spec files (relative to .assay/)
specs_dir = "specs/"

# Gate execution configuration
# Uncomment and customize as needed.
# [gates]
# Maximum time (seconds) a gate command can run before being killed.
# default_timeout = 300
#
# Working directory for gate execution. See GATE-04.
# working_dir = "."
"#
    )
}
```

### Complete Example Spec Template (String)

```rust
fn render_example_spec() -> &'static str {
    r#"# Example specification
# This file demonstrates how to write an Assay spec.
# Spec files live in .assay/specs/ and use TOML format.

# The spec name (required, must be unique across all specs)
name = "hello-world"

# A human-readable description of what this spec covers
description = "A starter spec to verify your Assay setup works"

# Criteria define the acceptance conditions for this spec.
# Each criterion has a name and description.
# Add an optional `cmd` field to make it machine-evaluatable.

[[criteria]]
name = "project-builds"
description = "The project compiles without errors"
cmd = "echo 'hello from assay'"

[[criteria]]
name = "readme-exists"
description = "A README file exists in the project root"
# No `cmd` — this criterion is evaluated manually (or by an agent in future versions)
"#
}
```

### Complete Init Function Structure

```rust
use std::path::{Path, PathBuf};
use crate::error::{AssayError, Result};

pub struct InitOptions {
    pub name: Option<String>,
}

pub struct InitResult {
    pub project_name: String,
    pub created_files: Vec<PathBuf>,
}

pub fn init(root: &Path, options: &InitOptions) -> Result<InitResult> {
    let assay_dir = root.join(".assay");
    let specs_dir = assay_dir.join("specs");

    // CFG-04: Refuse to overwrite existing .assay/
    match std::fs::create_dir(&assay_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(AssayError::AlreadyInitialized);
        }
        Err(source) => {
            return Err(AssayError::Io {
                operation: "creating .assay directory".into(),
                path: assay_dir,
                source,
            });
        }
    }

    // Create specs/ subdirectory
    std::fs::create_dir(&specs_dir).map_err(|source| AssayError::Io {
        operation: "creating specs directory".into(),
        path: specs_dir.clone(),
        source,
    })?;

    let mut created_files = Vec::new();

    // CFG-02: Generate config.toml
    let project_name = options
        .name
        .clone()
        .unwrap_or_else(|| infer_project_name(root));

    let config_path = assay_dir.join("config.toml");
    let config_content = render_config_template(&project_name);
    std::fs::write(&config_path, &config_content).map_err(|source| AssayError::Io {
        operation: "writing config".into(),
        path: config_path.clone(),
        source,
    })?;
    created_files.push(config_path);

    // CFG-03: Example spec
    let spec_path = specs_dir.join("hello-world.toml");
    std::fs::write(&spec_path, render_example_spec()).map_err(|source| AssayError::Io {
        operation: "writing example spec".into(),
        path: spec_path.clone(),
        source,
    })?;
    created_files.push(spec_path);

    // .gitignore inside .assay/
    let gitignore_path = assay_dir.join(".gitignore");
    std::fs::write(&gitignore_path, render_gitignore()).map_err(|source| AssayError::Io {
        operation: "writing .gitignore".into(),
        path: gitignore_path.clone(),
        source,
    })?;
    created_files.push(gitignore_path);

    Ok(InitResult {
        project_name,
        created_files,
    })
}

fn infer_project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or("assay-project")
        .to_string()
}
```

### Complete load() + validate() Pipeline

```rust
use std::path::Path;
use assay_types::Config;
use crate::error::{AssayError, Result};

/// Parse a config from a TOML string without validation.
/// Useful for tests and tools that want to skip semantic validation.
pub fn from_str(s: &str) -> std::result::Result<Config, toml::de::Error> {
    toml::from_str(s)
}

/// Load and validate a config from `.assay/config.toml` relative to `root`.
pub fn load(root: &Path) -> Result<Config> {
    let path = root.join(".assay").join("config.toml");

    let content = std::fs::read_to_string(&path).map_err(|source| AssayError::Io {
        operation: "reading config".into(),
        path: path.clone(),
        source,
    })?;

    let config: Config = toml::from_str(&content).map_err(|e| AssayError::ConfigParse {
        path: path.clone(),
        message: e.to_string(),
    })?;

    if let Err(errors) = validate(&config) {
        return Err(AssayError::ConfigValidation {
            path,
            errors,
        });
    }

    Ok(config)
}
```

### toml::de::Error Display Format (What Users See)

The toml crate produces rich error messages with line/column location. For parse errors:

```
TOML parse error at line 1, column 10
  |
1 | 00:32:00.a999999
  |          ^
Unexpected `a`
Expected `digit`
```

For `deny_unknown_fields` violations, the error includes the unknown key name and the list of expected keys:

```
unknown field `typo_field`, expected `project_name` or `specs_dir` or `gates`
```

When wrapped in `AssayError::ConfigParse`, the full message becomes:

```
parsing config `.assay/config.toml`: TOML parse error at line 5, column 1
  |
5 | typo_field = "oops"
  | ^^^^^^^^^^
unknown field `typo_field`, expected `project_name` or `specs_dir` or `gates`
```

### .gitignore Template

```rust
fn render_gitignore() -> &'static str {
    r#"# Assay transient files
# Results from gate evaluations
results/
# Cache files
*.cache
"#
}
```

## Discretion Decisions

### Project Name Sanitization: Minimal

**Recommendation:** Use `Path::file_name()` + `to_str()` with a fallback to `"assay-project"`. Do NOT strip special characters, replace spaces, or slugify. Users who want a specific name can use `--name`.

**Rationale:** The `project_name` is a display string, not a filesystem path or identifier. Over-sanitizing ("my cool project" -> "my-cool-project") surprises users. Under-sanitizing (accepting anything that `file_name()` returns) preserves the user's directory naming choice. The only edge case to handle is "no name at all" (root directory, `..`, empty).

### Example Spec: `hello-world.toml`

**Recommendation:** Name it `hello-world.toml` with a `"hello-world"` spec name. Include two criteria: one executable (`cmd = "echo 'hello from assay'"`) and one descriptive-only (no `cmd`). This matches CONTEXT.md which requires both modes demonstrated.

**Rationale:** "hello-world" is universally understood as a starter/example convention. The runnable criterion uses `echo` which works on all platforms. The descriptive criterion demonstrates the optional-cmd pattern.

### Gitignore Contents

**Recommendation:** Ignore `results/` directory and `*.cache` files. Track `config.toml`, `specs/`, and `.gitignore` itself.

**Rationale:** Gate results are transient output (regenerated on each run). Config and specs are the source of truth and should be version-controlled. This matches the CONTEXT.md requirement to ignore "transient files (results, caches) while tracking config and specs."

### Config Template Formatting

**Recommendation:** Group config into visual sections with blank line separators. Use `#` comments above each field explaining its purpose. Commented-out `[gates]` section with explanatory comments referencing GATE-04 for `working_dir`.

**Rationale:** Self-documenting config files reduce the need for external documentation. Users opening `config.toml` for the first time should understand every field without consulting a manual.

## Open Questions

1. **Existing `Config` type removal scope**
   - What we know: The current `Config` in assay-types has `project_name` and `workflows: Vec<Workflow>`. Phase 5 replaces it with `project_name`, `specs_dir`, `gates`.
   - What's unclear: Whether the `Workflow` and `Gate` types in assay-types should also be cleaned up (they're placeholders from Phase 1).
   - Recommendation: Replace `Config` with the Phase 5 shape. Leave `Workflow` and `Gate` types as-is for now — they'll be revisited in later phases. Only update tests/schemas that directly reference `Config`.

2. **`from_str` return type**
   - What we know: CONTEXT.md says `from_str()` "just parses without validation."
   - What's unclear: Should `from_str()` return `Result<Config, toml::de::Error>` (raw toml error) or `Result<Config>` (wrapped in AssayError)?
   - Recommendation: Return `Result<Config, toml::de::Error>`. The raw toml error is more useful for tests and tools that want to inspect parse details. `load()` wraps it into `AssayError::ConfigParse`. This matches the "composable API" principle from CONTEXT.md.

3. **ConfigError type location**
   - What we know: `ConfigError` is used by `validate()` in assay-core and displayed by `AssayError::ConfigValidation`.
   - What's unclear: Should `ConfigError` live in `error.rs` alongside `AssayError`, or in `config/mod.rs`?
   - Recommendation: Put `ConfigError` in `config/mod.rs` (it's config-specific validation output) and import it into `error.rs` for the `ConfigValidation` variant. This keeps config concerns together.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_toml` — toml::de::Error structure with span, message, Display impl with line/column (queried 2026-03-01)
- Context7 `/websites/rs_clap` — derive Subcommand with optional named args, `#[arg(long)]` pattern (queried 2026-03-01)
- Codebase inspection — existing `Config` type, `AssayError` pattern, CLI structure, test patterns (direct read 2026-03-01)
- Phase 3 research — established patterns for error variants, serde hygiene, module organization
- Rust std docs — `create_dir` returns `AlreadyExists` vs `create_dir_all` silent success

### Secondary (MEDIUM confidence)
- [toml::Deserializer docs](https://docs.rs/toml/latest/toml/struct.Deserializer.html) — confirmed validate_struct_keys is internal, unknown field detection via serde
- [toml::ser::to_string_pretty](https://docs.rs/toml/latest/toml/ser/fn.to_string_pretty.html) — confirmed no comment support in toml serialization
- [serde deny_unknown_fields](https://serde.rs/field-attrs.html) — confirmed known incompatibilities with tag/flatten
- [Rust forum: accumulating errors](https://users.rust-lang.org/t/accumulating-multiple-errors-error-products/93730) — Vec collection pattern and error sink pattern

### Tertiary (LOW confidence)
- None — all findings verified with primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in workspace; no new dependencies required
- Architecture patterns: HIGH — follows established codebase conventions (free functions in core, thin CLI wrapper, error patterns from Phase 3)
- Pitfalls: HIGH — `create_dir` vs `create_dir_all` behavior verified via docs; `deny_unknown_fields` compatibility confirmed for flat structs; toml error format verified via Context7 source code
- Discretion decisions: HIGH — all grounded in CONTEXT.md constraints and existing codebase patterns
- Config type redesign: HIGH — current type shape verified, all dependent tests/schemas identified

**Research date:** 2026-03-01
**Valid until:** 2026-03-31 (stable libraries, 30-day validity)
