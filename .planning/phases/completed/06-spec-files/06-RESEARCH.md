# Phase 6: Spec Files - Research

**Researched:** 2026-03-01
**Domain:** TOML spec file parsing, validation, directory scanning, CLI subcommands with formatted output
**Confidence:** HIGH

## Summary

Phase 6 implements spec file loading, validation, directory scanning, and two CLI commands (`spec show`, `spec list`). The technical foundation is already well-established: the `toml` crate (0.8) and `serde` with `#[serde(deny_unknown_fields)]` are proven patterns from Phase 5's config module. The `Criterion` type already exists in `assay-types` with the exact shape needed. The `Spec` type exists but needs `criteria: Vec<Criterion>` added. The `spec` module in `assay-core` is a stub ready for implementation.

The primary implementation work mirrors the config module pattern almost exactly: `from_str()` for raw parsing, `validate()` for multi-error semantic validation, `load()` composing both with file path context, and `scan()` for directory enumeration. The key new concerns are: (1) directory scanning with error handling for individual files, (2) cross-spec validation (duplicate `name` fields), and (3) CLI table-formatted output with color support.

**Primary recommendation:** Follow the Phase 5 config module pattern precisely. Use `std::fs::read_dir` for scanning (no external crate needed). For CLI table output, use manual `println!`-based formatting rather than adding a table library dependency — the output is simple enough (4 columns, no wrapping needed) that a dependency is unjustified.

## Standard Stack

### Core (already in workspace)

| Library   | Version | Purpose                                      | Notes                                                     |
| --------- | ------- | -------------------------------------------- | --------------------------------------------------------- |
| toml      | 0.8.23  | Spec TOML parsing via `toml::from_str`       | Already a dependency of `assay-core`                      |
| serde     | 1.0     | Derive `Deserialize`/`Serialize` on types    | Already in `assay-types`                                  |
| schemars  | 1       | JsonSchema derivation for updated Spec type  | Already in `assay-types`                                  |
| thiserror | 2       | New `AssayError` variants for spec errors    | Already in `assay-core`                                   |
| clap      | 4.5     | `spec show` and `spec list` subcommands      | Already in `assay-cli`                                    |
| serde_json| 1       | `--json` flag output on `spec show`          | Needs adding to `assay-cli` `[dependencies]`              |
| tempfile  | 3       | Test isolation for filesystem tests          | Already a workspace dev-dependency of `assay-core`        |

### New Dependencies Required

| Library    | Version | Purpose                        | Add To                              |
| ---------- | ------- | ------------------------------ | ----------------------------------- |
| serde_json | 1       | JSON serialization for `--json`| `assay-cli` `[dependencies]`        |

`serde_json` is already in the workspace `[workspace.dependencies]` (used by assay-types as dev-dep). It just needs adding to `assay-cli`'s `[dependencies]` section for runtime `--json` output.

### Alternatives Considered

| Instead of          | Could Use      | Tradeoff                                                                  |
| ------------------- | -------------- | ------------------------------------------------------------------------- |
| Manual table format | `comfy-table`  | Adds a dependency for 4-column table; manual formatting is ~20 lines      |
| Manual table format | `tabled`       | Derive-based table output from structs; overkill for one CLI command      |
| Manual colors       | `colored`      | Nice API but adds dependency; `\x1b[...]` escape codes are 3-4 lines     |
| Manual colors       | `termcolor`    | Cross-platform; unnecessary since macOS-only terminal target for now      |
| `std::fs::read_dir` | `walkdir`      | Recursive traversal; flat scan is the simpler choice (discretion area)    |

**Recommendation:** No new dependencies beyond `serde_json`. Manual formatting for tables and colors keeps the dependency footprint minimal and the output fully under our control.

### Installation

No new workspace dependencies needed. Only wire existing workspace deps:

```toml
# In crates/assay-cli/Cargo.toml [dependencies]
serde_json.workspace = true
```

## Architecture Patterns

### Updated Type in assay-types

The existing `Spec` struct in `assay-types/src/lib.rs` must be updated to include `criteria`:

```rust
/// A specification that defines what should be built and its acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Display name for this spec (required, non-empty after trim).
    pub name: String,

    /// Human-readable description of what this spec covers.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Acceptance criteria for this spec. Must have at least one.
    pub criteria: Vec<Criterion>,
}
```

Key changes from existing type:
- Add `#[serde(deny_unknown_fields)]` (locked decision from CONTEXT.md)
- Add `criteria: Vec<Criterion>` field
- Add `#[serde(default, skip_serializing_if = "String::is_empty")]` on `description` (brainstorm hygiene recommendation)

The `Criterion` type already has the correct shape and already has `#[serde(skip_serializing_if = "Option::is_none", default)]` on `cmd`. It needs `#[serde(deny_unknown_fields)]` added per CONTEXT.md decision.

### Recommended Module Structure

```
crates/assay-core/src/spec/
  mod.rs              # from_str(), validate(), load(), scan(), SpecError type
```

No submodule split needed — the spec module is a single concern analogous to `config/mod.rs`.

### Pattern 1: Spec Module — Mirrors Config Module

**What:** Free functions `from_str()`, `validate()`, `load()`, `scan()` in `assay_core::spec`.
**When to use:** All spec parsing operations.

```rust
// crates/assay-core/src/spec/mod.rs

use std::fmt;
use std::path::Path;

use assay_types::Spec;
use crate::error::{AssayError, Result};

/// A single validation issue in a spec file.
#[derive(Debug, Clone)]
pub struct SpecError {
    /// The field path (e.g., "name", "criteria[0].name").
    pub field: String,
    /// What's wrong.
    pub message: String,
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Parse a spec from a TOML string without validation.
pub fn from_str(s: &str) -> std::result::Result<Spec, toml::de::Error> {
    toml::from_str(s)
}

/// Validate a parsed spec for semantic correctness.
pub fn validate(spec: &Spec) -> std::result::Result<(), Vec<SpecError>> {
    let mut errors = Vec::new();

    if spec.name.trim().is_empty() {
        errors.push(SpecError {
            field: "name".into(),
            message: "required, must not be empty".into(),
        });
    }

    if spec.criteria.is_empty() {
        errors.push(SpecError {
            field: "criteria".into(),
            message: "at least one criterion is required".into(),
        });
    }

    // Check for duplicate criterion names
    let mut seen_names = std::collections::HashSet::new();
    for (i, criterion) in spec.criteria.iter().enumerate() {
        if criterion.name.trim().is_empty() {
            errors.push(SpecError {
                field: format!("criteria[{i}].name"),
                message: "required, must not be empty".into(),
            });
        } else if !seen_names.insert(criterion.name.trim()) {
            errors.push(SpecError {
                field: format!("criteria[{i}].name"),
                message: format!("duplicate criterion name `{}`", criterion.name.trim()),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Load and validate a spec from a TOML file path.
pub fn load(path: &Path) -> Result<Spec> {
    let content = std::fs::read_to_string(path).map_err(|source| AssayError::Io {
        operation: "reading spec".into(),
        path: path.to_path_buf(),
        source,
    })?;

    let spec: Spec = toml::from_str(&content).map_err(|e| AssayError::SpecParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    if let Err(errors) = validate(&spec) {
        return Err(AssayError::SpecValidation {
            path: path.to_path_buf(),
            errors,
        });
    }

    Ok(spec)
}

/// Scan a directory for all .toml spec files and parse them.
pub fn scan(specs_dir: &Path) -> Result<Vec<(String, Spec)>> {
    // Returns Vec of (filename_stem, Spec) tuples
    // ...
}
```

### Pattern 2: New AssayError Variants

```rust
// Added to crates/assay-core/src/error.rs

/// Spec file parsing failed (invalid TOML or schema mismatch).
#[error("parsing spec `{path}`: {message}")]
SpecParse {
    path: PathBuf,
    message: String,
},

/// Spec validation failed (structurally valid TOML but semantically invalid).
#[error("invalid spec `{path}`:\n{}", .errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n"))]
SpecValidation {
    path: PathBuf,
    errors: Vec<crate::spec::SpecError>,
},

/// Spec directory scanning failed.
#[error("scanning specs directory `{path}`: {source}")]
SpecScan {
    path: PathBuf,
    source: std::io::Error,
},
```

These mirror `ConfigParse` / `ConfigValidation` exactly, maintaining the established error variant naming convention.

### Pattern 3: CLI Subcommand Structure

```rust
// In assay-cli Command enum
#[derive(Subcommand)]
enum Command {
    // ... existing Init, Mcp ...

    /// Spec file operations
    Spec {
        #[command(subcommand)]
        command: SpecCommand,
    },
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Display a spec in human-readable format
    Show {
        /// Spec name (filename without .toml extension)
        name: String,

        /// Output as JSON instead of table format
        #[arg(long)]
        json: bool,
    },

    /// List all specs in the project
    List,
}
```

### Pattern 4: Table Output Format

The CLI should produce formatted table output. Manual formatting keeps the dependency count low:

```
Spec: hello-world
Description: A starter spec to verify your Assay setup works

  #  Criterion        Type          Command
  ─  ───────────────  ──────────── ─────────────────────────
  1  project-builds   executable    echo 'hello from assay'
  2  readme-exists    descriptive   —
```

Color scheme (ANSI escape codes, suppressed when `NO_COLOR` is set):
- "executable" → green
- "descriptive" → dim/gray
- Spec name → bold

### Pattern 5: NO_COLOR Handling

```rust
fn colors_enabled() -> bool {
    std::env::var("NO_COLOR").is_err()
}
```

Per the [NO_COLOR standard](https://no-color.org/), when `NO_COLOR` is present (any non-empty value), disable ANSI color output. When absent, enable colors.

### Anti-Patterns to Avoid

1. **Putting Spec as a DTO in assay-core instead of assay-types.** The `Spec` struct is a pure data type (Serialize, Deserialize, JsonSchema). It belongs in `assay-types`. Only the loading/validation functions belong in `assay-core`.

2. **Adding `#[serde(flatten)]` to embed Criterion fields directly in Spec.** Flatten is incompatible with `deny_unknown_fields`. Criteria must use `Vec<Criterion>` with `[[criteria]]` TOML array-of-tables syntax.

3. **Sorting scan results by filesystem order.** `read_dir` iteration order is platform-dependent and not guaranteed. Sort results alphabetically by filename for deterministic output.

4. **Using `unwrap()` on `read_dir` entries.** Each `DirEntry` in the iterator is `Result<DirEntry, io::Error>`. Must handle errors per-entry.

5. **Failing entire scan on one bad spec file.** If one spec in the directory has invalid TOML, the scan should still return results for the valid specs. Collect parse/validation errors and report them separately.

6. **Adding `PartialEq` on `SpecError` for test assertions.** Use field-level assertions instead, matching the pattern established by config tests.

## Don't Hand-Roll

| Problem                        | Don't Build                        | Use Instead                                      | Why                                                                |
| ------------------------------ | ---------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------ |
| TOML parsing                   | Manual TOML parser                 | `toml::from_str::<Spec>()`                       | toml crate handles `[[criteria]]` array-of-tables automatically    |
| Unknown field rejection        | Manual key iteration               | `#[serde(deny_unknown_fields)]`                  | Serde attribute; toml deserializer validates struct keys            |
| Duplicate name detection       | Complex set intersection           | `HashSet::insert()` returns `false` on duplicate | Standard library; one-liner duplicate check                        |
| Directory scanning             | `walkdir` or custom recursion      | `std::fs::read_dir()` + filter                   | Flat scan only; no recursion needed                                |
| JSON output for `--json`       | Manual JSON string building        | `serde_json::to_string_pretty(&spec)`            | Spec already derives `Serialize`                                   |
| File stem extraction           | Manual string split on `.`         | `Path::file_stem()`                              | Handles edge cases (no extension, multiple dots)                   |

**Key insight:** The `[[criteria]]` TOML syntax (array of tables) maps directly to `Vec<Criterion>` via serde. The toml crate handles this automatically — no special parsing code needed. The existing `hello-world.toml` template already uses this exact syntax.

## Common Pitfalls

### Pitfall 1: `deny_unknown_fields` on Spec Breaks Existing Serialized Schema

**What goes wrong:** Adding `#[serde(deny_unknown_fields)]` to `Spec` changes the JSON schema output. Snapshot tests for the `spec` schema will fail.

**Why it happens:** Phase 4 generated and committed schema snapshots for the original `Spec` type (with just `name` and `description`). The updated type changes the schema.

**How to avoid:** Update schemas in the same plan task: modify the `Spec` struct, run `cargo insta review` for snapshot acceptance, run `just schemas` to regenerate `schemas/spec.schema.json`. Verify with `just ready`.

**Warning signs:** `just ready` fails on snapshot mismatch.

### Pitfall 2: read_dir Ordering Is Non-Deterministic

**What goes wrong:** `spec list` output varies between runs or across platforms because `read_dir` does not guarantee alphabetical ordering.

**Why it happens:** Directory entry iteration order is filesystem-dependent (inode order on ext4, creation order on APFS, etc.).

**How to avoid:** After collecting entries from `read_dir`, sort by filename before processing or displaying:

```rust
let mut entries: Vec<_> = std::fs::read_dir(specs_dir)?
    .filter_map(|entry| entry.ok())
    .filter(|entry| {
        entry.path().extension().is_some_and(|ext| ext == "toml")
    })
    .collect();
entries.sort_by_key(|e| e.file_name());
```

**Warning signs:** Flaky tests that depend on ordering. `spec list` output differs between macOS and Linux CI.

### Pitfall 3: Cross-Spec Duplicate Name Detection Happens at Scan Time

**What goes wrong:** Two spec files both having `name = "hello-world"` is not caught by single-file validation. It can only be detected when scanning the full specs directory.

**Why it happens:** `validate()` operates on a single `Spec`. Duplicate names across files requires comparing all loaded specs.

**How to avoid:** `scan()` must perform a second validation pass after loading all specs:

```rust
// After loading all specs
let mut seen_names: HashMap<&str, &str> = HashMap::new(); // name -> filename
for (filename, spec) in &specs {
    if let Some(existing) = seen_names.insert(spec.name.trim(), filename) {
        // Report: duplicate name in `filename` already defined in `existing`
    }
}
```

**Warning signs:** Two specs with the same `name` field silently coexist, causing ambiguity in downstream phases.

### Pitfall 4: Config's `specs_dir` May Not Match Default

**What goes wrong:** `scan()` hardcodes `.assay/specs/` but the config allows `specs_dir` to be customized.

**Why it happens:** The config's `specs_dir` field (default `"specs/"`) is relative to `.assay/`. If a user changes it to `"my-specs/"`, scanning `.assay/specs/` finds nothing.

**How to avoid:** `scan()` takes an explicit `Path` parameter, not hardcoded. The CLI resolves the path from config:

```rust
let config = config::load(root)?;
let specs_dir = root.join(".assay").join(&config.specs_dir);
let specs = spec::scan(&specs_dir)?;
```

**Warning signs:** `spec list` shows nothing even though spec files exist in a custom directory.

### Pitfall 5: Empty Description Field Handling

**What goes wrong:** If `description` is optional (with `serde(default)`) and the user omits it, the spec loads fine but display output shows an empty description line.

**Why it happens:** `description` defaulting to `""` is structurally valid but visually ugly in table output.

**How to avoid:** In the CLI display code, skip the description line when empty. The `#[serde(skip_serializing_if = "String::is_empty")]` attribute ensures `--json` output also omits it when empty.

**Warning signs:** Table output shows "Description: " with nothing after it.

### Pitfall 6: load() Path Semantics Differ from Config

**What goes wrong:** Config's `load(root)` takes the project root and internally constructs `.assay/config.toml`. Spec's `load()` should take the spec file path directly (since scan provides paths).

**Why it happens:** Config has a single known file location. Specs have many files discovered at runtime.

**How to avoid:** `spec::load(path: &Path)` takes the full path to a spec file. `spec::scan(specs_dir: &Path)` takes the specs directory. This is different from `config::load(root: &Path)` — and that's correct.

**Warning signs:** Confusion about whether `spec::load` takes a directory or file path.

## Code Examples

### TOML Spec File (Already Working Template)

```toml
# From init.rs render_example_spec()
name = "hello-world"
description = "A starter spec to verify your Assay setup works"

[[criteria]]
name = "project-builds"
description = "The project compiles without errors"
cmd = "echo 'hello from assay'"

[[criteria]]
name = "readme-exists"
description = "A README file exists in the project root"
```

This parses directly into `Spec { name, description, criteria: Vec<Criterion> }` via `toml::from_str`. The `[[criteria]]` syntax is TOML's array-of-tables, which serde maps to `Vec<T>`.

### Verified: deny_unknown_fields with Array of Tables

`#[serde(deny_unknown_fields)]` on both `Spec` and `Criterion` works correctly with `[[criteria]]` because:
- The `criteria` key is a known field on `Spec`
- Each table in `[[criteria]]` deserializes as a `Criterion` with its own `deny_unknown_fields`
- No `#[serde(tag)]` or `#[serde(flatten)]` involved — no compatibility issues

```rust
// This TOML:
// [[criteria]]
// name = "test"
// description = "test desc"
// typo_field = "oops"
//
// Produces: unknown field `typo_field`, expected `name` or `description` or `cmd`
```

### Directory Scanning Pattern

```rust
use std::path::Path;

pub fn scan(specs_dir: &Path) -> Result<Vec<(String, Spec)>> {
    let mut entries: Vec<_> = std::fs::read_dir(specs_dir)
        .map_err(|source| AssayError::SpecScan {
            path: specs_dir.to_path_buf(),
            source,
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().is_some_and(|ext| ext == "toml")
        })
        .collect();

    // Sort for deterministic output
    entries.sort_by_key(|e| e.file_name());

    let mut specs = Vec::new();
    let mut errors = Vec::new();

    for entry in entries {
        let path = entry.path();
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        match load(&path) {
            Ok(spec) => specs.push((stem, spec)),
            Err(e) => errors.push(e),
        }
    }

    // Cross-spec duplicate name validation here...

    // Decision: how to handle errors — see Discretion section
    Ok(specs)
}
```

### CLI Table Formatting

```rust
fn display_spec(name: &str, spec: &Spec, colors: bool) {
    let bold = if colors { "\x1b[1m" } else { "" };
    let green = if colors { "\x1b[32m" } else { "" };
    let dim = if colors { "\x1b[2m" } else { "" };
    let reset = if colors { "\x1b[0m" } else { "" };

    println!("{bold}Spec:{reset} {name}");
    if !spec.description.is_empty() {
        println!("{bold}Description:{reset} {}", spec.description);
    }
    println!();
    println!("  {bold}#  Criterion        Type          Command{reset}");
    println!("  ─  ───────────────  ────────────  ─────────────────────────");

    for (i, criterion) in spec.criteria.iter().enumerate() {
        let (type_label, color) = if criterion.cmd.is_some() {
            ("executable", green)
        } else {
            ("descriptive", dim)
        };
        let cmd_display = criterion.cmd.as_deref().unwrap_or("—");
        println!(
            "  {:<2} {:<16} {color}{:<12}{reset}  {}",
            i + 1,
            criterion.name,
            type_label,
            cmd_display,
        );
    }
}
```

## Discretion Decisions

### Flat-only directory scan (recommended)

**Recommendation:** Scan only the top-level `specs_dir` directory, do not recurse into subdirectories.

**Rationale:** Subdirectory traversal adds complexity (cycle detection, symlink handling, nested naming) with no clear user need. The `.assay/specs/` directory is user-created and simple. If users want organization, they can use descriptive filenames. Subdirectory support can be added later as a non-breaking enhancement.

### Scan error handling: skip invalid + report (recommended)

**Recommendation:** When scanning, if one spec file fails to parse/validate, skip it and collect the error. Return all successfully loaded specs alongside a separate list of errors. The CLI decides how to present both.

**Rationale:** Failing the entire scan because of one broken spec file is hostile UX. Users may be iterating on one spec while others are stable. The scan function signature becomes:

```rust
pub struct ScanResult {
    pub specs: Vec<(String, Spec)>,
    pub errors: Vec<AssayError>,
}

pub fn scan(specs_dir: &Path) -> Result<ScanResult> { ... }
```

The outer `Result` covers directory-level failures (e.g., specs dir doesn't exist). The inner `errors` field covers per-file failures.

### Error type: dedicated SpecError (recommended)

**Recommendation:** Create a `SpecError` type in `spec/mod.rs` mirroring `ConfigError` in `config/mod.rs`. Add `SpecParse`, `SpecValidation`, and `SpecScan` variants to `AssayError`.

**Rationale:** This follows the established convention. `SpecError` is the validation error unit (field + message). `AssayError` variants wrap these with file path context for propagation. Keeping them parallel to config makes the codebase predictable.

### Validation severity: errors only, no warnings (recommended)

**Recommendation:** All validation issues are errors. No warning tier.

**Rationale:** Phase 6 has clear binary rules (name required, non-empty, unique criteria names, at least one criterion). None of these are "maybe wrong" — they're all "definitely wrong." A warning tier adds complexity (return type, display logic, exit codes) with no current use case. Can be added in a later phase if needed.

## State of the Art

| Old Approach (assay-types today) | Current Approach (Phase 6)         | Impact                                     |
| -------------------------------- | ---------------------------------- | ------------------------------------------ |
| `Spec { name, description }`    | `Spec { name, description, criteria }` | Breaking type change; update schemas/tests |
| No `deny_unknown_fields` on Spec | `#[serde(deny_unknown_fields)]` on both | Strict parsing catches typos               |
| `spec` module is stub           | Full `from_str/validate/load/scan` | Complete spec loading pipeline             |
| CLI has Init + Mcp only         | CLI adds Spec { Show, List }       | First user-facing query commands           |

**Deprecated/outdated:**
- The current `Spec` struct shape (no `criteria`) is a Phase 1 placeholder that must be replaced.
- The `Gate` struct in `assay-types` is also a placeholder but is NOT touched in this phase (deferred to Phase 7).

## Open Questions

1. **`description` field: required or optional?**
   - What we know: CONTEXT.md says `Spec struct: name, description, criteria: Vec<Criterion>`. The existing type has `pub description: String` (required). The brainstorm recommended `skip_serializing_if` hygiene.
   - What's unclear: Should `description` be `String` (required, can be empty) or `Option<String>` (truly optional)?
   - Recommendation: Keep as `String` with `#[serde(default, skip_serializing_if = "String::is_empty")]`. This means it defaults to `""` when omitted from TOML, and is excluded from JSON when empty. This is the simplest approach and matches the existing pattern.

2. **`Criterion.description` field: same question**
   - What we know: Currently `pub description: String` (required).
   - What's unclear: Whether criterion descriptions should also default when omitted.
   - Recommendation: Keep as `String` with `#[serde(default, skip_serializing_if = "String::is_empty")]` for consistency. Criteria must have a `name`; `description` is helpful but not mandatory.

3. **scan() duplicate name: which file wins?**
   - What we know: CONTEXT.md says "Duplicate name field values across spec files is an error — reject at scan time."
   - What's unclear: Whether to reject both specs or just the second one encountered.
   - Recommendation: Report both filenames in the error message ("duplicate spec name `X` in files `a.toml` and `b.toml`"). Neither "wins" — both are flagged. The user must fix one. This goes into `ScanResult.errors`.

4. **Schema snapshot updates**
   - What we know: Changing `Spec` breaks the schema snapshot from Phase 4.
   - What's unclear: Whether `Criterion` schema also changes (adding `deny_unknown_fields` might affect schema output).
   - Recommendation: Run `cargo insta review` after type changes and inspect diffs. The `deny_unknown_fields` attribute is a serde attribute, not a schemars attribute — it should NOT change the JSON schema. But test to confirm.

## Sources

### Primary (HIGH confidence)
- Context7 `/websites/rs_toml` — `toml::from_str` signature, `toml::de::Error` structure, array-of-tables deserialization (queried 2026-03-01)
- Context7 `/websites/rs_clap` — derive `Subcommand` with nested enums, `#[arg(long)]` for flags (queried 2026-03-01)
- Codebase inspection — existing `Spec` type, `Criterion` type, `config` module pattern, `AssayError` variants, CLI structure, `init.rs` templates, Phase 5 research/plans (direct read 2026-03-01)
- Phase 5 RESEARCH.md — established patterns for `from_str`/`validate`/`load`, `ConfigError`, error variant naming, test patterns

### Secondary (MEDIUM confidence)
- [Rust std::fs::read_dir docs](https://doc.rust-lang.org/std/fs/fn.read_dir.html) — iteration order not guaranteed, `DirEntry` is `Result`, verified via official docs
- [NO_COLOR standard](https://no-color.org/) — `NO_COLOR` env var convention; when present (non-empty), disable ANSI color
- [serde deny_unknown_fields issues](https://github.com/serde-rs/serde/issues/2666) — incompatibility with `tag`/`flatten` confirmed; plain structs are safe
- [comfy-table](https://github.com/Nukesor/comfy-table) and [tabled](https://github.com/zhiburt/tabled) — evaluated and rejected as unnecessary for this phase's simple table output

### Tertiary (LOW confidence)
- None — all findings verified with primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in workspace or trivially wired; no version conflicts
- Architecture patterns: HIGH — directly mirrors Phase 5 config module; patterns proven in production
- Pitfalls: HIGH — ordering, duplicate detection, path semantics all verified with std docs and codebase patterns
- Discretion decisions: HIGH — all grounded in CONTEXT.md constraints and established conventions
- Type changes: HIGH — existing `Spec` and `Criterion` types inspected; schema/test impact identified

**Research date:** 2026-03-01
**Valid until:** 2026-03-31 (stable libraries, established patterns, 30-day validity)
