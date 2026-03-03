# Stack Research: v0.2.0 Feature Additions

**Research Date:** 2026-03-02
**Scope:** Stack additions for run history persistence, required/advisory gate enforcement, and agent gate recording
**Existing Stack:** Rust 1.93 stable (2024 edition), serde 1.x, serde_json 1.x, schemars 1.x, clap 4, rmcp 0.17, tokio 1, chrono 0.4, thiserror 2, tracing 0.1

---

## Executive Summary

The v0.2.0 features require **zero new workspace dependencies**. The existing stack (serde_json, chrono, std::fs) covers all persistence needs. Run history uses one-JSON-file-per-run written to `.assay/results/`, not an append-only log. The required/advisory gate distinction is a pure type-level change with `#[serde(default)]`. The `gate_report` MCP tool fits cleanly into the existing rmcp 0.17 server with no API changes needed. The only workspace-level change is adding `serde_json` as a runtime dependency to `assay-core` (currently dev-only).

**Confidence:** High. All answers are verified against the current codebase, crates.io versions, and rmcp 0.17.0 release notes.

---

## Question 1: Run History JSON Persistence

### What's Needed

Run history writes `GateRunSummary` results as JSON files to `.assay/results/`. Each gate run produces one file.

### Do We Need New Crates?

**No.** The existing `serde_json` 1.x (already in workspace) and `std::fs` provide everything needed.

| Capability | Provided By | Already in Workspace |
|---|---|---|
| JSON serialization | `serde_json::to_string_pretty` | Yes (`serde_json = "1"`) |
| JSON deserialization (reading history) | `serde_json::from_str` | Yes |
| File creation/write | `std::fs::write` | stdlib |
| Directory creation | `std::fs::create_dir_all` | stdlib |
| Timestamp for filenames | `chrono` 0.4 | Yes (`chrono = { version = "0.4", features = ["serde"] }`) |
| Directory listing (history queries) | `std::fs::read_dir` | stdlib |

**One change needed:** `assay-core` currently lists `serde_json` only in `[dev-dependencies]`. It needs to be promoted to `[dependencies]` since `assay-core` will own the `history::save()` and `history::list()` functions that serialize/deserialize `GateRunSummary`.

### Storage Strategy: One File Per Run (Not Append-Only Log)

**Recommendation:** Individual JSON files, not JSON Lines (NDJSON).

**File naming convention:**
```
.assay/results/{spec-name}_{ISO8601-timestamp}.json
```

Example:
```
.assay/results/auth-flow_2026-03-02T14-30-05Z.json
```

**Why individual files over append-only NDJSON:**

| Factor | Individual Files | NDJSON Append Log |
|---|---|---|
| Concurrent safety | Atomic via `write` to unique filename | Requires file locking for append |
| Partial corruption | One bad file, rest survive | Corrupted line can break tail parsing |
| Querying by spec | Filter filenames, O(n) readdir | Must parse every line to filter |
| Deletion/cleanup | `rm` individual files | Rewrite entire file |
| Git friendliness | One file per change in diff | Entire file shows as modified |
| Complexity | `std::fs::write` + `serde_json` | Need `OpenOptions::append` + `BufWriter` + newline discipline |

The NDJSON approach (via `serde-jsonlines` 0.7 or manual `serde_json::to_writer` + `\n`) would add complexity without benefit. Individual files are simpler, safer, and more appropriate for a results store with <1000 entries per project.

### Implementation Pattern

```rust
// In assay-core::history

use std::path::Path;
use chrono::Utc;

/// Persist a gate run summary to `.assay/results/`.
pub fn save(results_dir: &Path, summary: &GateRunSummary) -> Result<PathBuf> {
    std::fs::create_dir_all(results_dir)?;

    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%SZ");
    let filename = format!("{}_{}.json", summary.spec_name, timestamp);
    let path = results_dir.join(&filename);

    let json = serde_json::to_string_pretty(summary)
        .map_err(/* wrap in AssayError */)?;
    std::fs::write(&path, json)
        .map_err(/* wrap in AssayError */)?;

    Ok(path)
}
```

**Key details:**
- Hyphens in timestamps (`T14-30-05Z`) avoid colons which are invalid in Windows filenames
- `to_string_pretty` over `to_writer` for human readability (files are small, <100KB)
- `create_dir_all` is idempotent -- safe to call on every save
- No file locking needed -- filenames include timestamps so collisions require sub-second concurrent runs of the same spec

### What NOT to Add

| Crate | Why Considered | Why Rejected |
|---|---|---|
| `serde-jsonlines` 0.7 | NDJSON read/write for append-only log | Individual files are simpler and safer for this use case. No append log needed. |
| `json-lines` 0.1.2 | Alternative NDJSON implementation | Same as above. |
| `fs2` 0.4.3 / `fs4` 0.13.1 | Cross-platform file locking | Not needed with unique-filename-per-run strategy. File locking adds complexity for zero benefit. |
| `uuid` 1.21.0 | Unique identifiers for run files | Timestamps provide sufficient uniqueness and are human-readable. UUID filenames are opaque. Consider adding only if we need stable run IDs for cross-referencing (not in v0.2 scope). |
| `rusqlite` / `sled` / `redb` | Embedded database for results | Massive overkill. We have <1000 results per project. JSON files are debuggable, git-friendly, and zero-dependency. |

---

## Question 2: Required/Advisory Gates (Serde Considerations)

### The Change

Add a `severity` field to the `Criterion` type in `assay-types`:

```rust
/// Whether a criterion failure blocks progression or is informational.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum CriterionSeverity {
    /// Failure blocks the workflow. This is the default.
    #[default]
    Required,
    /// Failure is reported but does not block.
    Advisory,
}
```

### Serde Considerations for Backward Compatibility

This is the critical question: can we add `severity` to `Criterion` without breaking existing `.toml` spec files that don't have the field?

**Yes, cleanly.** Using `#[serde(default)]`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u64>,

    // NEW: defaults to Required when absent
    #[serde(default)]
    pub severity: CriterionSeverity,
}
```

**Why this works:**

1. **`#[serde(default)]` on the field** -- When deserializing TOML/JSON that omits `severity`, serde calls `CriterionSeverity::default()` which returns `Required`. Existing spec files continue to work unchanged.

2. **`#[derive(Default)]` on the enum** with `#[default]` on `Required` -- This is the standard pattern from serde docs. The `Default` trait impl is generated by the `#[default]` attribute on the variant (Rust 1.62+, well within our MSRV).

3. **`deny_unknown_fields` interaction** -- The `Criterion` struct already uses `#[serde(deny_unknown_fields)]`. This is fine because we're adding a *known* field with a default, not dealing with unknown fields. Existing files without `severity` will deserialize correctly because `default` provides the value.

4. **Serialization** -- We may want `#[serde(skip_serializing_if = "CriterionSeverity::is_required")]` to avoid cluttering spec files with `severity = "Required"` on every criterion. This keeps the output clean for the default case:

```rust
impl CriterionSeverity {
    fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }
}
```

5. **TOML representation** -- serde's default enum serialization produces `severity = "Required"` or `severity = "Advisory"` in TOML (unit variants as strings). This reads naturally in spec files:

```toml
[[criteria]]
name = "lint-warnings"
description = "No clippy warnings"
cmd = "cargo clippy -- -D warnings"
severity = "Advisory"
```

6. **JSON representation** -- Identical pattern: `"severity": "Required"` or `"severity": "Advisory"`. JsonSchema generates an enum schema with the two string values.

### No New Dependencies Needed

This is purely a type change within existing serde/schemars derives. No new crates required.

### Schema Impact

The `schemars::JsonSchema` derive will automatically generate a schema for `CriterionSeverity` as a string enum. The existing `inventory::submit!` pattern in `assay-types` should include a new entry for the severity enum schema if we want it in the schema registry.

---

## Question 3: `gate_report` MCP Tool

### rmcp Status

| Attribute | Value |
|---|---|
| **Current Version** | 0.17.0 |
| **Released** | 2026-02-27 (3 days ago) |
| **Is Latest** | Yes (verified via `cargo search rmcp`) |
| **API Breaking Changes Since Our Integration** | None -- we built on 0.17.0 |

### v0.17.0 New Features Relevant to v0.2

| Feature | Relevance | Recommendation |
|---|---|---|
| **`Json<T>` structured output** | High -- `gate_report` could return typed JSON instead of stringified JSON | Consider adopting for `gate_report` response |
| **Trait-based tool declaration** (PR #677) | Low -- alternative to `#[tool_router]`, not a replacement | Skip -- macro approach works well, no need to refactor |
| **MCP SDK conformance tests** (PR #687) | None -- internal to rmcp | No action needed |
| **Default value support in schemas** (PR #686) | Low -- for elicitation, not tool params | No action needed |

### Adding `gate_report` to Existing Server

Adding a new tool to the existing `AssayServer` requires zero API changes. The pattern is identical to the existing `gate_run` tool:

```rust
// In assay-mcp/src/server.rs

/// Parameters for the `gate_report` tool.
#[derive(Deserialize, JsonSchema)]
struct GateReportParams {
    /// Spec name to get history for.
    #[schemars(description = "Spec name (filename without .toml extension)")]
    name: String,
    /// Maximum number of recent results to return.
    #[schemars(description = "Max results to return (default: 10)")]
    #[serde(default = "default_report_limit")]
    limit: usize,
}

fn default_report_limit() -> usize { 10 }

// Inside #[tool_router] impl AssayServer:
#[tool(
    description = "Get recent gate run history for a spec. Returns an array of \
        past gate results with timestamps, pass/fail counts, and duration."
)]
async fn gate_report(
    &self,
    params: Parameters<GateReportParams>,
) -> Result<CallToolResult, McpError> {
    // delegate to assay_core::history::list()
}
```

**No rmcp upgrade needed.** The `#[tool]` + `#[tool_router]` macros from 0.17.0 support adding any number of tools to the same impl block.

### Consider: `Json<T>` Structured Output

rmcp 0.17.0's `Json<T>` wrapper places the response in `structured_content` with an auto-generated schema. This is better for agent consumption than stringified JSON in a text content block.

**Current pattern (v0.1.0):**
```rust
let json = serde_json::to_string(&response)?;
Ok(CallToolResult::success(vec![Content::text(json)]))
```

**New pattern with `Json<T>`:**
```rust
Ok(Json(response))  // response type derives Serialize + JsonSchema
```

**Recommendation:** Use `Json<T>` for `gate_report` since it returns structured data that agents need to parse. Consider migrating existing tools (`spec_list`, `spec_get`, `gate_run`) to `Json<T>` in a follow-up, but do not block v0.2 on this -- it is a quality-of-life improvement, not a requirement.

**Caveat:** `Json<T>` requires the response type to derive `JsonSchema` (in addition to `Serialize`). The `GateRunSummary` struct in `assay-core` currently only derives `Serialize`. If we want to use `Json<GateRunSummary>` from `assay-mcp`, we'd need to add `JsonSchema` to `GateRunSummary` and `CriterionResult`. This means `assay-core` would need `schemars` as a dependency. Alternatively, keep a separate response DTO in `assay-mcp` that derives both (current pattern with `GateRunResponse`/`CriterionSummary`).

### rmcp API Surface Stability

From the v0.16.0 and v0.17.0 release notes, the core tool API (`#[tool]`, `#[tool_router]`, `#[tool_handler]`, `Parameters<T>`, `CallToolResult`, `McpError`) has been stable. Changes have been in:
- Streamable HTTP transport (we don't use)
- Auth/OAuth (we don't use)
- Schema generation internals (transparent to us)
- Conformance testing (internal)

**Risk of breakage on 0.18:** Low for our stdio tool server pattern.

---

## Question 4: Audit Trail / Evidence Persistence Crates

### Assessment

There are no established Rust crates for "structured evidence persistence" or "audit trails" as a domain concept. The search surfaces:

- **`cargo-auditable`** -- Embeds dependency metadata into binaries. Unrelated.
- **`tracing` + JSON subscriber** -- Structured logging, not structured evidence storage.
- **Various database crates** (SQLite, sled, redb) -- Overkill for file-per-run storage.

### Why No External Crate is Needed

Assay's "audit trail" is simply: gate run results saved as JSON files in `.assay/results/`. This is:

1. **Self-describing** -- each JSON file contains the full `GateRunSummary` with spec name, criteria results, timestamps, stdout/stderr evidence, and duration.
2. **Queryable** -- `std::fs::read_dir` + `serde_json::from_str` to list/filter results.
3. **Immutable** -- write-once, never updated (new runs create new files).
4. **Human-readable** -- pretty-printed JSON, inspectable with `cat` or `jq`.

The `GateRunSummary` type already captures all the evidence fields needed for an audit trail:

```rust
pub struct GateResult {
    pub passed: bool,
    pub kind: GateKind,      // what was checked
    pub stdout: String,      // evidence
    pub stderr: String,      // evidence
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub truncated: bool,
    pub original_bytes: Option<u64>,
}
```

This is already a structured evidence record. No additional abstraction needed.

### What NOT to Add

| Crate | Why Considered | Why Rejected |
|---|---|---|
| `rusqlite` 0.34 | Structured queries, transactions | Introduces C dependency (sqlite3), build complexity. JSON files are sufficient for <1000 records. |
| `sled` 0.34 | Embedded key-value store | Unmaintained (last release 2022). Not suitable. |
| `redb` 2.x | Modern embedded database | Pure Rust but adds 50KB+ to binary. Overkill for simple file storage. |
| `fjall` 2.x | LSM-tree storage engine | Enterprise-grade, massive dependency tree. Wrong scale entirely. |
| Custom audit trait/framework | Abstraction over evidence recording | YAGNI. The `GateRunSummary` type IS the evidence record. Adding an abstraction layer over it adds indirection without value at this scale. |

---

## Workspace Dependency Changes Summary

### Changes to Root `Cargo.toml` `[workspace.dependencies]`

**None.** No new workspace dependencies for v0.2.0.

### Per-Crate Dependency Changes

| Crate | Change | Rationale |
|---|---|---|
| `assay-core` | Promote `serde_json` from `[dev-dependencies]` to `[dependencies]` | Core now serializes `GateRunSummary` to JSON for persistence |
| `assay-types` | None | Severity enum uses existing serde/schemars derives |
| `assay-mcp` | None | New `gate_report` tool uses existing rmcp macros |
| `assay-cli` | None | CLI may add a `history` subcommand but uses only existing deps |

### Concrete `Cargo.toml` Diff for `assay-core`

```diff
 [dependencies]
 assay-types.workspace = true
 chrono.workspace = true
 serde.workspace = true
+serde_json.workspace = true
 thiserror.workspace = true
 toml.workspace = true

 [dev-dependencies]
 tempfile.workspace = true
```

Note: `serde_json` is removed from `[dev-dependencies]` because it is now a regular dependency (cargo handles this automatically -- a crate listed in `[dependencies]` is available in tests without also being in `[dev-dependencies]`).

---

## Integration Points with Existing Stack

### History Module in `assay-core`

New module: `assay-core::history` (alongside existing `gate`, `spec`, `config`, `review`, `workflow`).

**Depends on:**
- `serde_json` -- JSON serialization of `GateRunSummary`
- `chrono` -- timestamp formatting for filenames
- `std::fs` -- file I/O
- `assay-types` -- `GateRunSummary` and related types (or keep `GateRunSummary` in `assay-core` as currently designed)

**Does NOT need:**
- `tokio` -- file I/O is sync, matching existing gate evaluation pattern
- `tracing` -- optional instrumentation, defer to implementation
- `rmcp` -- history is a domain concern, not an MCP concern

### `GateRunSummary` Serialization

`GateRunSummary` currently derives `Serialize` but not `Deserialize`. For history reading, it needs `Deserialize` added:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRunSummary { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult { ... }
```

This is a backward-compatible change. `GateRunSummary` is in `assay-core` (not `assay-types`), so the `Deserialize` derive requires `serde` (already a dependency of `assay-core`).

### MCP Tool Registration

The `gate_report` tool follows the exact same pattern as `gate_run`:
1. Define params struct with `Deserialize + JsonSchema`
2. Add `#[tool(...)]` annotated async fn to the `#[tool_router] impl AssayServer` block
3. Delegate to `assay_core::history::list()`
4. Format response and return `CallToolResult`

No changes to `ServerHandler` impl, `ServerCapabilities`, or transport configuration.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Filename timestamp collisions (sub-second runs) | Very Low | Low | Could append a 4-char random suffix if needed; not worth pre-solving |
| Large result files (huge stdout/stderr) | Low | Low | Already mitigated by existing 64KB truncation in gate evaluation |
| `deny_unknown_fields` conflict with new `severity` field | None | None | `#[serde(default)]` provides the value; this is a known field, not unknown |
| rmcp 0.18 breaking `#[tool]` macro API | Low | Medium | Pin to `0.17`, macro API has been stable across recent releases |
| `GateRunSummary` schema drift (old JSON files vs new struct) | Low | Medium | Add `#[serde(default)]` to any new fields on `GateRunSummary` for forward compatibility |

---

## Sources

- [rmcp v0.17.0 release notes](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v0.17.0) -- released 2026-02-27, confirmed latest via `cargo search`
- [rmcp v0.16.0 release notes](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v0.16.0) -- API stability reference
- [rmcp `Json<T>` structured output docs](https://docs.rs/rmcp/latest/rmcp/handler/server/wrapper/struct.Json.html) -- new in 0.17
- [rmcp trait-based tool declaration PR #677](https://github.com/modelcontextprotocol/rust-sdk/pull/677) -- alternative to macros, not a replacement
- [Serde field attributes: `#[serde(default)]`](https://serde.rs/field-attrs.html) -- backward-compatible field addition
- [Serde variant attributes: `#[serde(other)]`](https://serde.rs/variant-attrs.html) -- forward-compatible enum deserialization
- [Serde container attributes: `#[serde(default)]`](https://serde.rs/attr-default.html) -- default value patterns
- [JSONL/NDJSON specification](https://ndjson.com/definition/) -- evaluated and rejected for this use case
- [serde-jsonlines crate](https://crates.io/crates/serde-jsonlines) -- v0.7.0, evaluated and rejected
- [uuid crate](https://crates.io/crates/uuid) -- v1.21.0, evaluated and rejected
- [fs4 crate](https://crates.io/crates/fs4) -- v0.13.1, evaluated and rejected
- Current codebase: `/Users/wollax/Git/personal/assay/Cargo.toml`, `crates/assay-core/Cargo.toml`, `crates/assay-mcp/src/server.rs`

---
*Stack research completed: 2026-03-02*
