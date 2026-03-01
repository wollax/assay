# Phase 3: Error Types and Domain Model - Research

**Researched:** 2026-03-01
**Domain:** Rust error handling, serde tagged enums, domain modeling for CLI + MCP dual-audience
**Confidence:** HIGH

## Summary

Phase 3 establishes the foundational type system that every downstream crate depends on: `AssayError` in `assay-core`, and `GateKind`, `GateResult`, `Criterion` in `assay-types`. The research covered thiserror 2.x error patterns, serde internally-tagged enum behavior with TOML and JSON, schemars 1.x compatibility with chrono for timestamps, and discretion decisions on error structure, output types, and timestamp formats.

The standard approach is straightforward: thiserror 2.x for `AssayError` with `#[non_exhaustive]` and context-rich variants, serde `#[serde(tag = "kind")]` for `GateKind` (verified to roundtrip through TOML correctly), `chrono::DateTime<Utc>` for timestamps (already transitively available via rmcp and schemars `chrono04` feature), and `String` (UTF-8 lossy) for stdout/stderr capture. All types in assay-types derive `Serialize`, `Deserialize`, `JsonSchema` with `#[serde(skip_serializing_if)]` hygiene on optional/empty fields.

**Primary recommendation:** Keep it minimal and mechanical. Every type is a simple derive-macro struct/enum. No trait objects, no generics, no custom serde. thiserror handles errors; serde + schemars handle serialization. The only research-worthy decision was chrono vs alternatives for timestamps, and chrono wins because it's already in the dependency tree.

## Standard Stack

The established libraries/tools for this domain:

### Core

| Library    | Version | Purpose                        | Why Standard                                                      |
| ---------- | ------- | ------------------------------ | ----------------------------------------------------------------- |
| thiserror  | 2       | Error enum derivation          | Workspace dep; generates Display, Error, From impls automatically |
| serde      | 1       | Serialization/deserialization  | Workspace dep; derive macros for all domain types                 |
| schemars   | 1       | JSON Schema generation         | Workspace dep; derive JsonSchema for all public types             |
| chrono     | 0.4     | Timestamp handling             | Already transitive dep via rmcp; provides DateTime<Utc> with serde + schemars support |
| toml       | 0.8+    | TOML serialization             | Needed for roundtrip tests; already in lock file via workspace    |

### Supporting

| Library    | Version | Purpose                        | When to Use                                                |
| ---------- | ------- | ------------------------------ | ---------------------------------------------------------- |
| serde_json | 1       | JSON serialization             | Workspace dep; used for MCP-facing serialization in tests  |

### Alternatives Considered

| Instead of              | Could Use                  | Tradeoff                                                                                         |
| ----------------------- | -------------------------- | ------------------------------------------------------------------------------------------------ |
| `chrono::DateTime<Utc>` | `std::time::SystemTime`    | SystemTime lacks serde derive and schemars support out of the box; chrono is already transitive   |
| `chrono::DateTime<Utc>` | `i64` epoch millis         | Loses timezone info, human-unreadable in TOML/JSON, no ISO 8601 display                          |
| `String` for stdout     | `Vec<u8>` for stdout       | Vec<u8> doesn't serialize to JSON naturally; lossy UTF-8 conversion is the standard Rust approach |
| thiserror               | anyhow                     | anyhow is for applications; thiserror is for libraries that export typed errors                    |

## Architecture Patterns

### Recommended Project Structure

```
crates/assay-types/src/
├── lib.rs          # Re-exports all public types
├── gate.rs         # GateKind, GateResult
├── criterion.rs    # Criterion
├── error.rs        # (NOT here — errors live in assay-core)
└── ...existing types (Spec, Gate, Review, etc.)

crates/assay-core/src/
├── lib.rs          # pub mod error; pub use error::{AssayError, Result};
├── error.rs        # AssayError enum, Result alias
└── ...existing modules
```

**Key principle:** assay-types = pure DTOs (Serialize, Deserialize, JsonSchema). assay-core = error types + business logic. AssayError lives in assay-core because it depends on thiserror (which is a core dep, not a types dep). Types crates should have zero behavior.

### Pattern 1: Error Type with Context Chain

thiserror 2.x generates `Display`, `Error`, and optionally `From` implementations. The `#[error("...")]` attribute supports field interpolation for context-rich messages.

```rust
// crates/assay-core/src/error.rs
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssayError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Convenience alias used throughout assay-core and downstream crates.
pub type Result<T> = std::result::Result<T, AssayError>;
```

**Why structured fields on Io (not `#[from]`):** The CONTEXT.md locks "full context chain — every layer adds context." Using `#[from] io::Error` would lose the file path. Instead, each call site constructs `AssayError::Io { path, source }` explicitly, which forces context to be provided at the point of failure.

**Why `#[non_exhaustive]`:** Locked decision. Allows adding variants in minor versions without breaking downstream `match` statements. Downstream code must have a wildcard arm.

### Pattern 2: Internally Tagged Enum for GateKind

```rust
// crates/assay-types/src/gate.rs
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum GateKind {
    Command { cmd: String },
    AlwaysPass,
}
```

**Verified behavior (tested with toml 0.8.23):**

TOML output for `Command` variant (flattened in parent struct):
```toml
name = "test"
kind = "Command"
cmd = "cargo test"
```

TOML output for `AlwaysPass` variant:
```toml
name = "always"
kind = "AlwaysPass"
```

TOML output for `Command` variant (nested as table):
```toml
name = "nested"

[kind]
kind = "Command"
cmd = "cargo test"
```

All roundtrip correctly through serialize -> deserialize. schemars 1.x correctly generates JSON Schema for internally tagged enums via the `#[serde(tag = "kind")]` attribute.

**Important:** Internally tagged enums do NOT support tuple variants (compile error). This is fine — `GateKind` only has struct and unit variants.

### Pattern 3: GateResult with Evidence Fields

```rust
// crates/assay-types/src/gate.rs
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    pub passed: bool,
    pub kind: GateKind,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub stdout: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub stderr: String,

    pub exit_code: Option<i32>,

    pub duration_ms: u64,

    pub timestamp: DateTime<Utc>,
}
```

### Pattern 4: Criterion with Optional Command

```rust
// crates/assay-types/src/criterion.rs
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Criterion {
    pub name: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub cmd: Option<String>,
}
```

### Anti-Patterns to Avoid

1. **Error variants without context:** `Io(std::io::Error)` with `#[from]` loses the file path. Always use struct variants with contextual fields.
2. **Errors in assay-types:** Error types have behavior (Display impl, source chain). They belong in assay-core, not the DTO crate.
3. **Custom Serialize/Deserialize impls:** For Phase 3 types, derive macros cover everything. Hand-written serde impls are a maintenance burden with no benefit.
4. **Speculative error variants:** "Add-as-consumed" is locked. Only add `Io` now. `Config`, `Spec`, `Gate` variants arrive in their respective phases.
5. **`#[serde(flatten)]` on GateKind in parent structs:** While flattening works for simple cases, it conflicts with `#[serde(deny_unknown_fields)]` and makes schema generation unpredictable. Use a named `kind` field instead.

## Don't Hand-Roll

| Problem                      | Don't Build                     | Use Instead                          | Why                                                                          |
| ---------------------------- | ------------------------------- | ------------------------------------ | ---------------------------------------------------------------------------- |
| Error Display formatting     | Manual `impl fmt::Display`      | thiserror `#[error("...")]`          | Derive macro handles interpolation, source chaining automatically            |
| Error source chain           | Manual `fn source()`            | thiserror `#[source]` / `#[from]`    | Automatic `Error::source()` impl                                             |
| JSON Schema for types        | Manual schema construction      | schemars `#[derive(JsonSchema)]`     | Keeps schema in sync with serde attrs automatically                          |
| Timestamp serialization      | Custom serde module             | chrono's default RFC 3339 serde      | ISO 8601 string format works for both JSON (MCP) and TOML (config)           |
| Tagged enum serialization    | Custom `Serialize` impl         | `#[serde(tag = "kind")]`            | Internally tagged repr is a single attribute, not custom code                |
| Skipping empty fields        | Custom serializer               | `#[serde(skip_serializing_if)]`      | Standard serde attribute, zero code                                          |

## Common Pitfalls

### Pitfall 1: `#[from]` Io Variant Loses Context

**What goes wrong:** Using `#[error(transparent)] Io(#[from] std::io::Error)` means `?` on io::Error auto-converts but discards which file/operation failed.

**Why it happens:** `#[from]` generates `From<io::Error>` which can't inject additional context.

**How to avoid:** Use a struct variant with explicit `path` and `source` fields. Callers use `map_err` to inject context:
```rust
std::fs::read_to_string(&path).map_err(|source| AssayError::Io {
    path: path.clone(),
    source,
})?;
```

**Warning signs:** Error messages like "No such file or directory" without any path.

### Pitfall 2: chrono Feature Not Activated on schemars in assay-types

**What goes wrong:** `DateTime<Utc>` doesn't derive `JsonSchema` when building assay-types in isolation.

**Why it happens:** schemars `chrono04` feature is activated transitively by rmcp (via assay-mcp), but assay-types itself only has schemars with default features. Cargo feature unification means it works in a full workspace build, but `cargo check -p assay-types` alone may fail.

**How to avoid:** Add `chrono` as a workspace dependency and enable `chrono04` on schemars in the workspace:
```toml
# Root Cargo.toml [workspace.dependencies]
chrono = { version = "0.4", features = ["serde"] }
schemars = { version = "1", features = ["chrono04"] }
```
Then add `chrono.workspace = true` to assay-types/Cargo.toml.

**Warning signs:** `JsonSchema` is not implemented for `DateTime<Utc>` compilation error.

### Pitfall 3: Missing `#[serde(default)]` Paired with `skip_serializing_if`

**What goes wrong:** Fields skipped during serialization can't be deserialized because they're absent in the input.

**Why it happens:** `skip_serializing_if = "Option::is_none"` omits the field from output. On deserialization, serde expects the field unless `#[serde(default)]` is set.

**How to avoid:** Always pair `skip_serializing_if` with `default`:
```rust
#[serde(skip_serializing_if = "Option::is_none")]
#[serde(default)]
pub cmd: Option<String>,
```

**Warning signs:** Deserialization errors like "missing field `cmd`" when reading output that was serialized with the skip.

### Pitfall 4: `exit_code` as `i32` Instead of `Option<i32>`

**What goes wrong:** Gates that don't execute a process (e.g., `AlwaysPass`, future `FileExists`) have no exit code.

**Why it happens:** Assuming all gates run a subprocess.

**How to avoid:** Use `Option<i32>` for `exit_code`. `AlwaysPass` sets it to `None`. `Command` sets it to `Some(code)`.

**Warning signs:** Having to invent fake exit codes (like `0` for AlwaysPass).

### Pitfall 5: GateResult Without `kind` Field

**What goes wrong:** MCP consumers receiving a `GateResult` can't tell HOW the gate was evaluated without also fetching the spec.

**Why it happens:** Treating `GateResult` as purely an outcome, not including its evaluation method.

**How to avoid:** CONTEXT.md locks this decision: include `kind: GateKind` on `GateResult`. This makes results self-describing.

**Warning signs:** Agents needing two API calls to understand a single result.

### Pitfall 6: Non-UTF-8 Process Output

**What goes wrong:** `String::from_utf8()` fails on binary output from commands.

**Why it happens:** Some commands emit non-UTF-8 bytes (binary, locale-specific encodings).

**How to avoid:** Use `String::from_utf8_lossy()` which replaces invalid UTF-8 sequences with the Unicode replacement character. This is the standard Rust approach for command output that needs to be JSON-serializable.

**Warning signs:** Panics or errors on `from_utf8().unwrap()`.

## Code Examples

### Complete AssayError (Phase 3 Scope Only)

```rust
// crates/assay-core/src/error.rs
use std::path::PathBuf;
use thiserror::Error;

/// Unified error type for all Assay operations.
///
/// New variants are added as downstream phases consume them.
/// The `#[non_exhaustive]` attribute ensures adding variants
/// is not a breaking change.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AssayError {
    /// An I/O operation failed.
    #[error("{operation} at `{path}`: {source}")]
    Io {
        /// What was being attempted (e.g., "reading config", "writing spec").
        operation: String,
        /// The file path involved.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

/// Convenience result alias for Assay operations.
pub type Result<T> = std::result::Result<T, AssayError>;
```

**Why `operation` field:** Adds a human-readable description of what was happening when the error occurred. "reading config at `/foo/config.toml`: No such file or directory" is diagnosable. "No such file or directory" is not.

### Complete GateKind and GateResult

```rust
// crates/assay-types/src/gate.rs
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The kind of evaluation used for a quality gate.
///
/// Internally tagged: serializes with a `kind` field.
/// Example TOML: `kind = "Command"` with `cmd = "cargo test"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum GateKind {
    /// A shell command to execute. Gate passes if exit code is 0.
    Command {
        /// The command string to execute via the system shell.
        cmd: String,
    },
    /// A gate that always passes. Useful for descriptive-only criteria.
    AlwaysPass,
}

/// The result of evaluating a quality gate.
///
/// Captures all evidence from gate execution for both
/// CLI display and MCP (JSON) consumption.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    /// Whether the gate passed.
    pub passed: bool,

    /// How the gate was evaluated (self-describing for MCP consumers).
    pub kind: GateKind,

    /// Standard output from the command (UTF-8 lossy).
    /// Empty string if no output or non-command gate.
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub stdout: String,

    /// Standard error from the command (UTF-8 lossy).
    /// Empty string if no output or non-command gate.
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub stderr: String,

    /// Process exit code. `None` for non-command gates (e.g., AlwaysPass).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub exit_code: Option<i32>,

    /// Wall-clock duration of gate evaluation in milliseconds.
    pub duration_ms: u64,

    /// When the gate evaluation completed (ISO 8601 / RFC 3339).
    pub timestamp: DateTime<Utc>,
}
```

### Complete Criterion

```rust
// crates/assay-types/src/criterion.rs
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An acceptance criterion within a specification.
///
/// Criteria are either descriptive (human-evaluated, no `cmd`)
/// or executable (machine-evaluated, `cmd` present).
/// Forward-compatible with future `prompt` field for agent evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Criterion {
    /// Unique name within the parent spec (freeform, trimmed non-empty).
    pub name: String,

    /// Human-readable description of what this criterion checks.
    pub description: String,

    /// Shell command to execute for machine evaluation.
    /// `None` = descriptive-only criterion (human-evaluated).
    /// `Some(cmd)` = executable criterion (gate runs this command).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub cmd: Option<String>,
}
```

### Test Pattern: TOML Roundtrip

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_kind_command_toml_roundtrip() {
        let kind = GateKind::Command { cmd: "cargo test".into() };
        let toml_str = toml::to_string(&kind).unwrap();
        assert!(toml_str.contains("kind = \"Command\""));
        assert!(toml_str.contains("cmd = \"cargo test\""));
        let roundtripped: GateKind = toml::from_str(&toml_str).unwrap();
        // Verify roundtrip matches
        let re_serialized = toml::to_string(&roundtripped).unwrap();
        assert_eq!(toml_str, re_serialized);
    }

    #[test]
    fn gate_kind_always_pass_toml_roundtrip() {
        let kind = GateKind::AlwaysPass;
        let toml_str = toml::to_string(&kind).unwrap();
        assert!(toml_str.contains("kind = \"AlwaysPass\""));
        let roundtripped: GateKind = toml::from_str(&toml_str).unwrap();
        let re_serialized = toml::to_string(&roundtripped).unwrap();
        assert_eq!(toml_str, re_serialized);
    }

    #[test]
    fn gate_result_json_skips_empty_fields() {
        let result = GateResult {
            passed: true,
            kind: GateKind::AlwaysPass,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 0,
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&result).unwrap();
        // Empty stdout, stderr, and None exit_code should be omitted
        assert!(!json.contains("stdout"));
        assert!(!json.contains("stderr"));
        assert!(!json.contains("exit_code"));
    }

    #[test]
    fn criterion_cmd_none_is_valid() {
        let c = Criterion {
            name: "code-review".into(),
            description: "Code has been reviewed by a human".into(),
            cmd: None,
        };
        let toml_str = toml::to_string(&c).unwrap();
        assert!(!toml_str.contains("cmd"));
        let roundtripped: Criterion = toml::from_str(&toml_str).unwrap();
        assert!(roundtripped.cmd.is_none());
    }

    #[test]
    fn criterion_cmd_some_is_valid() {
        let c = Criterion {
            name: "tests-pass".into(),
            description: "All tests pass".into(),
            cmd: Some("cargo test".into()),
        };
        let toml_str = toml::to_string(&c).unwrap();
        assert!(toml_str.contains("cmd = \"cargo test\""));
        let roundtripped: Criterion = toml::from_str(&toml_str).unwrap();
        assert_eq!(roundtripped.cmd, Some("cargo test".into()));
    }
}
```

## Discretion Decisions

Recommendations for the areas left to Claude's discretion in CONTEXT.md:

### Error Structure: Structured Fields

**Recommendation:** Use structured fields (typed `path: PathBuf`, `operation: String`) on error variants, not formatted strings.

**Rationale:** MCP consumers (agents) may need to programmatically inspect error details. A `path` field is machine-readable; an embedded-in-prose path is not. For v0.1 this matters less, but it's zero extra cost to do it right. The Display impl (via thiserror `#[error("...")]`) provides the human-readable format.

### Error Presentation: Single thiserror Display

**Recommendation:** Use a single `thiserror::Display` implementation for both CLI and MCP. Do NOT create separate error formatting for v0.1.

**Rationale:** For v0.1, `AssayError` has only an `Io` variant. The thiserror Display is human-readable and sufficient for both audiences. MCP can return the Display string as an error message. If future phases need structured error responses for MCP, that's a Phase 8+ concern.

### Stdout/Stderr Capture: Full Capture, No Truncation in Phase 3

**Recommendation:** Capture full output as `String` (UTF-8 lossy). No truncation logic in Phase 3.

**Rationale:** The brainstorm issue for truncation metadata (`truncated: bool`, `original_bytes: Option<u64>`) is filed as a future enhancement. Phase 3 defines the DTO shape. Phase 7 (gate evaluation) implements capture and can add truncation fields then. Adding truncation fields now would be speculative.

### Output Type: String (UTF-8 Lossy)

**Recommendation:** `String` for `stdout` and `stderr`, populated via `String::from_utf8_lossy()`.

**Rationale:** JSON serialization (for MCP) requires string data. `Vec<u8>` would need base64 encoding, adding complexity for near-zero benefit. The vast majority of command output (test runners, linters, compilers) is UTF-8. Lossy conversion replaces the rare invalid byte with a replacement character, which is acceptable for evidence capture.

### Timestamp Format: chrono DateTime<Utc> (ISO 8601 / RFC 3339)

**Recommendation:** `chrono::DateTime<Utc>` with default serde serialization (RFC 3339 format, e.g., `"2026-03-01T12:00:00Z"`).

**Rationale:**
- ISO 8601 is human-readable in both TOML and JSON output
- chrono is already a transitive dependency (via rmcp and schemars)
- schemars `chrono04` feature provides `JsonSchema` impl (activated by rmcp)
- Epoch millis would require mental conversion and loses timezone information
- TOML has native datetime support compatible with RFC 3339

**Workspace change required:** Add `chrono` as explicit workspace dep with `serde` feature, and add `chrono04` feature to workspace schemars. See Pitfall 2.

### Criterion Kind Modeling: Single Struct with Optional cmd

**Recommendation:** Single `Criterion` struct with `cmd: Option<String>`.

**Rationale:** The TOML authoring experience is cleaner: omit `cmd` for descriptive criteria, include it for executable ones. An enum (`Descriptive { name, desc }` / `Executable { name, desc, cmd }`) duplicates `name` and `description` fields across variants and makes TOML less readable. The optional field pattern is standard for "feature flags" on DTOs.

### Prompt Field: Document-Only, Do Not Reserve

**Recommendation:** Do NOT add `prompt: Option<String>` in Phase 3. Document in code comments that it's planned.

**Rationale:** Adding an unused `Option` field now means every test and constructor must deal with it. serde `default` handles forward-compatibility: future TOML files with `prompt = "..."` won't break old parsers if we add the field later. The `#[serde(default)]` + `skip_serializing_if` pattern means adding a new optional field is always backward-compatible.

### Criteria Severity: All-Equal Pass/Fail

**Recommendation:** No severity levels in v0.1. All criteria are equal pass/fail.

**Rationale:** Severity is a Phase 7+ orchestration concern (should a warning-level failure block progression?). The DTO doesn't need it until the orchestrator exists. Adding `severity: Option<Severity>` now is speculative.

### Criterion Naming: Freeform Unique

**Recommendation:** Freeform string names, validated as non-empty after trim, unique within a spec. No slug constraints.

**Rationale:** Humans author these in TOML files. Constraining to slugs (`[a-z0-9-]+`) creates friction. Uniqueness is enforced at validation time (Phase 6, SPEC-04), not at the type level. The `name` field is a `String`, not a newtype wrapper.

## State of the Art

| Old Approach                         | Current Approach                       | When Changed    | Impact                                             |
| ------------------------------------ | -------------------------------------- | --------------- | -------------------------------------------------- |
| thiserror 1.x                        | thiserror 2.x                          | Oct 2024        | Adds `no_std` support; API otherwise identical      |
| schemars 0.8 (no chrono support)     | schemars 1.x with `chrono04` feature   | June 2025       | Feature renamed from `chrono` to `chrono04`         |
| `serde(tag)` on enums (JSON only)    | Works with TOML 0.8+ too               | Ongoing         | Internally tagged enums serialize/deserialize in TOML correctly |
| Separate `RootSchema` in schemars    | Unified `Schema` type in 1.x           | June 2025       | schema_for! returns Schema, not RootSchema          |

**Deprecated/outdated:**
- `schemars` feature `chrono`: renamed to `chrono04` in schemars 1.x
- thiserror 1.x: superseded by 2.x with no API breakage for this project's usage

## Open Questions

1. **PartialEq on domain types**
   - What we know: The requirements don't mention `PartialEq` on `GateKind`, `GateResult`, or `Criterion`. Tests can compare fields individually.
   - What's unclear: Whether downstream phases (spec validation, gate comparison) will need structural equality.
   - Recommendation: Derive `PartialEq` on `GateKind` and `Criterion` (simple value types). Do NOT derive it on `GateResult` (contains `DateTime<Utc>` which makes equality semantically questionable for timestamps).

2. **toml as workspace dependency**
   - What we know: `toml` is not currently in workspace dependencies. It's needed for roundtrip tests in assay-types.
   - What's unclear: Whether it should be a workspace dep or a dev-dependency only on assay-types.
   - Recommendation: Add `toml` as a workspace dependency (it will be needed in assay-core for config/spec parsing in Phases 5-6 anyway). Use it as `[dev-dependencies]` in assay-types for now.

3. **Module organization in assay-types**
   - What we know: Current lib.rs has all types inline. Phase 3 adds 3 more types (GateKind, GateResult, Criterion) plus modifying existing ones.
   - What's unclear: Whether to split into modules now or wait.
   - Recommendation: Split into modules (`gate.rs`, `criterion.rs`) and re-export from `lib.rs`. This matches assay-core's existing module-per-domain pattern and keeps files focused.

## Sources

### Primary (HIGH confidence)
- Context7 `/dtolnay/thiserror` — thiserror 2.x derive macro, #[error], #[from], #[source], transparent, backtrace (queried 2026-03-01)
- Context7 `/websites/serde_rs` — serde internally tagged enum representation, container attributes (queried 2026-03-01)
- Context7 `/gresau/schemars` — schemars 1.x derive JsonSchema, serde attribute compatibility, tag/untagged support (queried 2026-03-01)
- Local TOML roundtrip test (`/private/tmp/assay-toml-test`) — verified `#[serde(tag = "kind")]` with struct variant, unit variant, flattened and nested, all roundtrip through toml 0.8.23
- `cargo tree -e features -i chrono` — confirmed schemars `chrono04` feature activated transitively via rmcp
- `cargo info thiserror` — confirmed v2.0.18, MIT OR Apache-2.0

### Secondary (MEDIUM confidence)
- Schemars feature flags documentation (https://graham.cool/schemars/features/) — chrono04 feature enables JsonSchema for chrono types
- chrono serde module docs (https://docs.rs/chrono/latest/chrono/serde/index.html) — default RFC 3339 format, alternative timestamp modules

### Tertiary (LOW confidence)
- None — all findings verified with primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in workspace or transitive deps; versions confirmed via cargo tree/info
- Architecture patterns: HIGH — internally tagged enums verified with local test; thiserror patterns from official docs
- Pitfalls: HIGH — chrono feature gap identified via cargo tree analysis; serde skip/default pairing from established serde documentation
- Discretion decisions: HIGH — all recommendations grounded in verified library behavior and project constraints from CONTEXT.md

**Research date:** 2026-03-01
**Valid until:** 2026-03-31 (stable libraries, 30-day validity)
