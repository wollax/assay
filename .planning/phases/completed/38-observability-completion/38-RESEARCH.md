# Phase 38: Observability Completion - Research

**Researched:** 2026-03-13
**Confidence:** HIGH (all findings from direct codebase investigation)

## Standard Stack

No new external dependencies required. This phase uses only existing crate capabilities:
- `serde` with `skip_serializing_if` for conditional field presence
- `schemars::JsonSchema` for response types that need schema generation
- Existing `assay_types::context` module for shared token/session types
- Existing `assay_core::context::tokens` for token extraction functions
- Existing `assay_core::gate::resolve_timeout` for timeout precedence logic

## Architecture Patterns

### Current `spec_get` Implementation (Confidence: HIGH)

**Location:** `crates/assay-mcp/src/server.rs`, line 573

**Current behavior:**
1. Resolves CWD, loads config, loads spec entry via `load_spec_entry_mcp`
2. Returns a `serde_json::json!()` inline object (no dedicated response struct)
3. For legacy specs: `{format, name, description, criteria}`
4. For directory specs: `{format, gates, feature_spec}`
5. Accepts only a `name: String` parameter (no `resolve` option)

**Current params struct:**
```rust
pub struct SpecGetParams {
    pub name: String,
}
```

**Key finding:** `spec_get` has NO dedicated response struct. It uses inline `serde_json::json!()`. To add `resolved` config, either:
- Add a response struct (consistent with other handlers like `GateRunResponse`)
- Continue with inline JSON construction (simpler but less type-safe)

**Recommendation:** Use inline `serde_json::json!()` to stay consistent with current `spec_get` pattern. The resolved block is additive — just merge it into the existing JSON object.

### Current `estimate_tokens` Implementation (Confidence: HIGH)

**Location:** `crates/assay-mcp/src/server.rs`, line 1167

**Current behavior:**
1. Resolves CWD, spawns blocking task
2. Finds session dir and resolves session path
3. Calls `assay_core::context::estimate_tokens(&session_path, &file_session_id)`
4. Returns `TokenEstimate` directly (serialized from assay-types)

**Current `TokenEstimate` type** (in `crates/assay-types/src/context.rs`, line 353):
```rust
pub struct TokenEstimate {
    pub session_id: String,
    pub context_tokens: u64,
    pub output_tokens: u64,
    pub context_window: u64,
    pub context_utilization_pct: f64,
    pub health: ContextHealth,
}
```

**Current core function** (in `crates/assay-core/src/context/tokens.rs`, line 112):
- Calls `quick_token_estimate(path)` which reads only the last 50KB of the session file
- Extracts the LAST assistant entry with usage data
- Computes utilization from a single usage snapshot

**Key finding:** The current implementation reads ONLY the tail of the file (50KB). For growth rate metrics, we need ALL assistant turn usage data. This requires either:
- A separate full-file parse (like `diagnose` does via `parse_session`)
- A new targeted scan function that collects only usage data from assistant entries

### Timeout Three-Tier Precedence (Confidence: HIGH)

The three tiers are already implemented in `assay_core::gate::resolve_timeout` (line 375 of `gate/mod.rs`):

```rust
pub fn resolve_timeout(
    cli_timeout: Option<u64>,
    criterion_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> Duration {
    let seconds = cli_timeout
        .or(criterion_timeout)
        .or(config_timeout)
        .unwrap_or(300);
    Duration::from_secs(seconds.max(MIN_TIMEOUT_SECS))
}
```

**The three tiers:**
1. **Spec tier** (`criterion.timeout`): Per-criterion `timeout` field in `Criterion` struct. `Option<u64>`, null when unset.
2. **Config tier** (`config.gates.default_timeout`): `GatesConfig.default_timeout`, defaults to 300 when `[gates]` section exists. `None` when `[gates]` section is absent.
3. **Default tier**: Hardcoded 300 seconds (plus a 1-second minimum floor).

**Important nuance for resolved display:** The MCP `gate_run` handler (line 736) does NOT pass CLI timeout to `resolve_timeout` — it passes `None` for `cli_timeout`:
```rust
let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);
// ...
assay_core::gate::evaluate_all(&spec, &working_dir_owned, None, config_timeout)
```

For `spec_get` resolved config, the relevant tiers are:
- **spec**: Per-criterion timeout (varies per criterion, but for the "default" cascade display, use `None` since it's per-criterion)
- **config**: `config.gates.as_ref().map(|g| g.default_timeout)` — `null` when no `[gates]` section
- **default**: `300`

**Cascade display decision:** Since spec-tier timeout varies per criterion, the resolved cascade should show the CONFIG-level effective timeout (not per-criterion). The "spec" tier in the cascade represents "does the config have a gates section with a timeout override", which maps to `config.gates.default_timeout`. The per-criterion timeout is already visible in the criteria array.

Actually, re-reading CONTEXT.md: the cascade shows `spec`, `config`, `default` — but in the actual codebase the tiers are `cli > criterion > config > default`. Since `spec_get` shows the spec definition (not a run), the meaningful cascade is:
- **spec**: Not applicable at spec level (this is per-criterion). Use `null`.
- **config**: `config.gates.default_timeout` if `[gates]` section exists, else `null`
- **default**: `300`
- **effective**: The value that would apply (first non-null wins)

### Working Directory Resolution (Confidence: HIGH)

**Location:** `crates/assay-mcp/src/server.rs`, line 1410

```rust
fn resolve_working_dir(cwd: &Path, config: &Config) -> PathBuf {
    match config.gates.as_ref().and_then(|g| g.working_dir.as_deref()) {
        Some(dir) => {
            let path = Path::new(dir);
            if path.is_absolute() { path.to_path_buf() }
            else { cwd.join(path) }
        }
        None => cwd.to_path_buf(),
    }
}
```

For the `working_dir` validation block, we need:
- `path`: The resolved absolute path (from `resolve_working_dir`)
- `exists`: `path.exists()`
- `accessible`: `path.is_dir()` (checks both existence and directory-ness with read permission)

### Session Entry Types for Turn Counting (Confidence: HIGH)

**Location:** `crates/assay-types/src/context.rs`

Assistant entries have:
- `meta.is_sidechain: bool` — must filter out sidechains
- `message.usage: Option<UsageData>` — present only on final response of a turn
- `UsageData.context_tokens()` — sum of input + cache_creation + cache_read

**Key insight for growth rate:** Each assistant entry with `usage` data represents one complete turn. The `context_tokens()` value grows monotonically (it's the cumulative context at that point). To compute growth rate:
- Collect all non-sidechain assistant entries with usage data
- Each one's `context_tokens()` is the total context at that turn
- Average tokens per turn = delta between consecutive turns, averaged
- Actually simpler: `last_context_tokens / turn_count` gives average growth
- Estimated turns remaining = `(available_context - last_context_tokens) / avg_tokens_per_turn`

### MCP Handler Patterns for Optional Parameters (Confidence: HIGH)

Several handlers already use optional parameters with `#[serde(default)]`:

```rust
// From GateRunParams:
#[serde(default)]
pub include_evidence: bool,

#[serde(default)]
pub timeout: Option<u64>,

// From SpecValidateParams:
#[serde(default)]
pub check_commands: bool,
```

Pattern for adding `resolve` to `SpecGetParams`:
```rust
pub struct SpecGetParams {
    pub name: String,
    #[serde(default)]
    pub resolve: bool,
}
```

### Response Struct Patterns (Confidence: HIGH)

Two patterns exist in the codebase:

1. **Dedicated response struct** (Serialize-only, private to server.rs): Used by `GateRunResponse`, `GateReportResponse`, `GateFinalizeResponse`, `GateHistoryListResponse`
2. **Shared type from assay-types** (Serialize + Deserialize + JsonSchema): Used by `TokenEstimate`, `WorktreeInfo`, `WorktreeStatus`, `DiagnosticsReport`

For `estimate_tokens`: The response type is `TokenEstimate` from `assay-types`. Adding growth rate means modifying this shared type. Since CONTEXT.md says breaking changes are acceptable, we can add fields directly.

For `spec_get`: Currently uses inline `serde_json::json!()`. Adding the `resolved` block is straightforward with conditional JSON merging.

## Don't Hand-Roll

- **Timeout precedence logic**: Already exists in `assay_core::gate::resolve_timeout`. Don't recreate it — extract the tier values from the same sources.
- **Session parsing**: Use existing `parse_session` or `quick_token_estimate` — don't write new JSONL parsers.
- **Sidechain filtering**: Use existing `is_sidechain()` function from `tokens.rs`.
- **Working directory resolution**: Use existing `resolve_working_dir()` helper.
- **Context window constants**: Use existing `DEFAULT_CONTEXT_WINDOW` and `SYSTEM_OVERHEAD_TOKENS`.

## Common Pitfalls

### P1: Growth rate on empty/small sessions (Confidence: HIGH)
The current `estimate_tokens` returns an error when no usage data exists. Growth rate must handle the case where fewer than 5 assistant turns exist by omitting the growth fields entirely (per success criteria: "absent, not zero").

### P2: Full file parse performance (Confidence: MEDIUM)
`estimate_tokens` currently reads only 50KB tail. Adding growth rate requires reading ALL assistant entries with usage. Two approaches:
- Full `parse_session` (used by `diagnose`) — works but parses ALL fields
- Custom scan that only extracts usage from assistant entries — faster but new code

**Recommendation:** Use `parse_session` for correctness. Performance is acceptable — `diagnose` already does this and these are typically <10MB files. If performance becomes a concern, optimize later.

### P3: Sidechain token inflation (Confidence: HIGH)
Subagent conversations have `is_sidechain: true`. Their token usage is separate from the main conversation. Growth rate MUST filter sidechains, consistent with how `extract_usage` already works.

### P4: `TokenEstimate` is a shared type (Confidence: HIGH)
`TokenEstimate` lives in `assay-types` and has `JsonSchema` derive. Adding optional growth rate fields requires:
- Adding fields with `#[serde(skip_serializing_if = "Option::is_none")]`
- Fields will automatically appear in generated JSON schema
- The `inventory::submit!` schema registration will pick up changes automatically

### P5: spec_get timeout cascade for per-criterion timeout (Confidence: HIGH)
The CONTEXT.md example shows a single "timeout" cascade, but per-criterion timeout varies across criteria. The cascade should show the **default** timeout resolution (without any criterion override), since the per-criterion values are already visible in the criteria list.

### P6: Config `[gates]` section may be absent (Confidence: HIGH)
`Config.gates` is `Option<GatesConfig>`. When absent:
- `config_timeout` is `None`
- `working_dir` falls back to CWD
- The cascade should show `config: null` in both cases

## Code Examples

### Adding `resolve` parameter to SpecGetParams

```rust
pub struct SpecGetParams {
    #[schemars(description = "Spec name (filename without .toml extension, e.g. 'auth-flow')")]
    pub name: String,

    #[schemars(description = "Include resolved configuration (effective timeouts, working_dir validation)")]
    #[serde(default)]
    pub resolve: bool,
}
```

### Building the resolved config block

```rust
// Inside spec_get handler, after loading config:
let resolved = if params.0.resolve {
    let config_timeout = config.gates.as_ref().map(|g| g.default_timeout);
    let effective_timeout = config_timeout.unwrap_or(300);
    let working_dir = resolve_working_dir(&cwd, &config);
    let wd_exists = working_dir.is_dir();

    Some(serde_json::json!({
        "timeout": {
            "effective": effective_timeout,
            "spec": null,       // spec-level is per-criterion, not a global
            "config": config_timeout,
            "default": 300
        },
        "working_dir": {
            "path": working_dir.to_string_lossy(),
            "exists": working_dir.exists(),
            "accessible": wd_exists
        }
    }))
} else {
    None
};
```

### Adding growth rate to TokenEstimate

```rust
// In assay-types/src/context.rs:
pub struct TokenEstimate {
    pub session_id: String,
    pub context_tokens: u64,
    pub output_tokens: u64,
    pub context_window: u64,
    pub context_utilization_pct: f64,
    pub health: ContextHealth,

    /// Growth rate metrics. Absent when fewer than 5 assistant turns exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_rate: Option<GrowthRate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GrowthRate {
    /// Average context tokens added per assistant turn.
    pub avg_tokens_per_turn: u64,
    /// Estimated assistant turns remaining before context window is full.
    pub estimated_turns_remaining: u64,
    /// Number of assistant turns used to compute these metrics.
    pub turn_count: u64,
}
```

### Collecting usage data from all assistant turns

```rust
// In assay-core/src/context/tokens.rs:
use super::parser::parse_session;

/// Collect context_tokens from each non-sidechain assistant turn with usage data.
fn collect_turn_tokens(path: &Path) -> crate::Result<Vec<u64>> {
    let (entries, _) = parse_session(path)?;
    let tokens: Vec<u64> = entries
        .iter()
        .filter(|e| !is_sidechain(&e.entry))
        .filter_map(|e| match &e.entry {
            SessionEntry::Assistant(a) => {
                a.message.as_ref()?.usage.as_ref().map(|u| u.context_tokens())
            }
            _ => None,
        })
        .collect();
    Ok(tokens)
}

/// Compute growth rate from turn token snapshots.
fn compute_growth_rate(turn_tokens: &[u64], context_window: u64) -> Option<GrowthRate> {
    if turn_tokens.len() < 5 {
        return None;
    }
    let turn_count = turn_tokens.len() as u64;
    let last = *turn_tokens.last()?;
    let avg = last / turn_count;  // simplified: total context / turns
    let available = context_window.saturating_sub(SYSTEM_OVERHEAD_TOKENS);
    let remaining_tokens = available.saturating_sub(last);
    let remaining_turns = if avg > 0 { remaining_tokens / avg } else { 0 };

    Some(GrowthRate {
        avg_tokens_per_turn: avg,
        estimated_turns_remaining: remaining_turns,
        turn_count,
    })
}
```

## Discretion Recommendations

### `resolve` should be opt-in (Confidence: HIGH)
Use `resolve: bool` parameter (default false). Rationale:
- Avoids breaking existing consumers who expect the current shape
- `resolve` requires filesystem checks (working_dir exists/accessible) — unnecessary overhead for agents that just want spec content
- Consistent with `include_evidence` and `check_commands` opt-in patterns

### Growth rate should be nested under `growth_rate` (Confidence: HIGH)
Use `Option<GrowthRate>` nested object. Rationale:
- Clean absence semantics: the entire block is null/omitted when <5 turns
- Avoids cluttering the top-level `TokenEstimate` with 3+ new fields
- Groups related metrics together

### Turn counting should be assistant-only (Confidence: HIGH)
Count only non-sidechain assistant entries with usage data. Rationale:
- User turns don't have usage data in JSONL format
- Each assistant usage entry represents a complete API call (the unit that consumes context)
- Sidechain entries inflate counts without contributing to main context growth

### Turns remaining should be a single integer (Confidence: HIGH)
Use a single `estimated_turns_remaining: u64`. Rationale:
- Simpler for agents to consume
- A range (min/max) adds complexity without clear benefit — the estimate is inherently approximate
- Agents need a single threshold signal, not statistical analysis

### 5-turn threshold should be hardcoded (Confidence: HIGH)
Hardcode `const MIN_TURNS_FOR_GROWTH_RATE: usize = 5`. Rationale:
- Making it configurable adds parameter complexity for minimal benefit
- 5 turns is a reasonable minimum for any meaningful average
- Can be extracted to config later if users request it

### Low-data absence: omit the section (Confidence: HIGH)
Use `skip_serializing_if = "Option::is_none"` to omit `growth_rate` entirely when <5 turns. Rationale:
- Consistent with existing patterns (e.g., `session_id` on `GateRunResponse`)
- `null` with a reason field adds unnecessary structure
- Agents can detect absence cleanly: field not present = insufficient data

### Read-only tools should NOT get Phase 35 warnings (Confidence: HIGH)
`spec_get` and `estimate_tokens` are read-only — they have no side effects that could partially fail. The warnings pattern (from Phase 35) is for mutating tools where a primary action succeeds but a secondary action (like history save) fails. Read-only tools should return errors, not warnings.

## File Inventory

Files that will be modified:

| File | Change |
|------|--------|
| `crates/assay-types/src/context.rs` | Add `GrowthRate` struct, add `growth_rate: Option<GrowthRate>` to `TokenEstimate` |
| `crates/assay-core/src/context/tokens.rs` | Add `collect_turn_tokens`, `compute_growth_rate` functions; modify `estimate_tokens` to include growth rate |
| `crates/assay-mcp/src/server.rs` | Add `resolve: bool` to `SpecGetParams`; add resolved config block to `spec_get` handler; update `estimate_tokens` handler to pass through growth rate |

Files that will NOT be modified:
- `crates/assay-mcp/src/lib.rs` — no public API changes needed
- `crates/assay-core/src/config/mod.rs` — config loading is unchanged
- `crates/assay-core/src/gate/mod.rs` — timeout resolution logic is read-only

---

*Phase: 38-observability-completion*
*Research completed: 2026-03-13*
