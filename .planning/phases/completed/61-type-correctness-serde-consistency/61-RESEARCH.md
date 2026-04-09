# Phase 61: Type Correctness & Serde Consistency - Research

**Researched:** 2026-04-09
**Domain:** Rust serde attributes, enum type design, internal tagging, backward-compat deserialization
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Criterion.when ambiguity (TYPE-01)
- Make `when` non-optional: change `Option<When>` to `When` with `#[serde(default)]`
- Omit `when` from serialized output when value is `SessionEnd` (use `skip_serializing_if`)
- Update all consumers — compiler will flag every site via type error; fix them all in this phase
- Update pre-M024 roundtrip test to assert `when == When::SessionEnd` instead of `when == None`

#### SessionPhase rename (TYPE-02, TYPE-05)
- Rename `review::SessionPhase` to `review::CheckpointPhase` at the type level (not just alias)
- Remove the `as CheckpointPhase` alias in `pipeline_checkpoint.rs` — use direct import
- Merge `AtEvent` and `OnEvent` into single `OnEvent` variant (semantically identical)
- Add `#[serde(alias = "at_event")]` on `OnEvent` for backward compat with persisted `GateDiagnostic` JSON
- Rename schema registry entry from `"checkpoint-session-phase"` to `"checkpoint-phase"`

#### Serde tagging alignment (TYPE-06)
- Align `CriterionKind` to internally tagged: add `#[serde(tag = "type", rename_all = "snake_case")]`
- Add serde aliases for old PascalCase format (`#[serde(alias = "AgentReport")]` etc.) for backward compat with existing spec TOML files and gate run JSON
- Keep PascalCase for the `Display` impl (human-readable output, different purpose than serde)

#### Validation (TYPE-03)
- Reject `AfterToolCalls { n: 0 }` at deserialize time via `#[serde(deserialize_with = "nonzero_u32")]`
- Error message: "AfterToolCalls.n must be >= 1 (got 0)"

#### Timeout overrides (TYPE-04)
- Keep single driver timeout parameter (per-criterion timeouts already handled in `gate/mod.rs`)
- Document timeout precedence: CLI > config > spec-level > default

#### SessionEnd no-op documentation (TYPE-07)
- Add doc comment on `evaluate_checkpoint` explaining SessionEnd is a no-op (criteria never match)
- Add `debug_assert!` that SessionEnd phase is never passed to `evaluate_checkpoint`

### Claude's Discretion
- Exact `nonzero_u32` deserializer implementation approach (custom fn vs newtype)
- Whether `is_session_end` helper is a free function or method on `When`
- Test organization for new serde alias roundtrip tests

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TYPE-01 | `Criterion.when: Option<When>` representational ambiguity resolved | `When` has `#[default]` on `SessionEnd`; pattern `#[serde(default, skip_serializing_if = "When::is_session_end")]` works for non-Option non-primitive types |
| TYPE-02 | `review::SessionPhase` renamed to `CheckpointPhase` (type-level, not alias) | Two import sites: `pipeline_checkpoint.rs` and `gate/mod.rs`; one match site: `assay-cli/src/commands/spec.rs`; one test site: `schema_snapshots.rs` |
| TYPE-03 | `When::AfterToolCalls { n: 0 }` rejected by validation | Custom `deserialize_with` fn pattern; must return `Err` for zero before struct is constructed |
| TYPE-04 | `evaluate_checkpoint` respects CLI/config timeout overrides | `evaluate_checkpoint` currently hard-codes `None, None` for both timeout params when delegating to `evaluate_all_with_events`; fix requires threading them through |
| TYPE-05 | `review::CheckpointPhase` includes `OnEvent` variant | Merge `AtEvent` → `OnEvent`; add `#[serde(alias = "at_event")]`; update CLI match arm |
| TYPE-06 | `CriterionKind` serde tagging made consistent with `When` | Current format is untagged PascalCase string/object; new format is internally tagged `snake_case`; aliases needed for `"AgentReport"`, `"NoToolErrors"`, `"EventCount"` |
| TYPE-07 | `evaluate_checkpoint` at `SessionEnd` documents no-op behavior | Add doc comment + `debug_assert!`; no behavioral change |
</phase_requirements>

## Summary

Phase 61 is a pure correctness and consistency refactor across `assay-types` and `assay-core`. All seven requirements target serde representation mismatches, type naming collisions, and missing validation. No new features are introduced; every change is either a breaking serde schema migration (with backward-compat aliases) or a documentation/assertion addition.

The highest-effort item is TYPE-01: changing `Criterion.when` from `Option<When>` to `When` propagates to ~110 struct literal sites in production code (spec/mod.rs, spec/validate.rs, gate/mod.rs, pipeline_checkpoint.rs, criterion.rs tests). The compiler will enumerate every site via type error — the plan must allocate a dedicated task for this mechanical update sweep.

TYPE-06 is the trickiest serde migration because `CriterionKind` currently uses serde's default untagged PascalCase representation (not `internally_tagged`), but existing TOML files contain `kind = "AgentReport"` (unit string) and `kind = { EventCount = { ... } }` (struct). After converting to `#[serde(tag = "type", rename_all = "snake_case")]`, old TOML `"AgentReport"` must still deserialize via `#[serde(alias = "AgentReport")]`. Aliases on enum variants work with internally tagged enums in serde for JSON but have a TOML-specific caveat: `toml` crate uses serde's deserialize path, so variant aliases work for unit variants but may not work for struct variants with the `tag = "type"` style in TOML. This is the critical risk area for TYPE-06.

**Primary recommendation:** Implement in dependency order: TYPE-07 (doc only, zero risk) → TYPE-02+TYPE-05 (rename + variant merge) → TYPE-01 (type change, compiler-guided sweep) → TYPE-03 (validation) → TYPE-06 (serde tag migration, highest risk) → TYPE-04 (timeout threading).

## Standard Stack

### Core (all already in workspace)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.x | Serialize/Deserialize derives and attributes | Workspace dep |
| serde_json | 1.x | JSON roundtrip tests | Workspace dep |
| toml | 0.8.x | TOML roundtrip tests | Workspace dep |
| schemars | 0.8.x | JsonSchema derive | Workspace dep |
| insta | 1.46 | Snapshot tests (json feature) | Workspace dep |

No new dependencies required for this phase.

**Snapshot update command:**
```bash
INSTA_UPDATE=always cargo nextest run -p assay-types --test schema_snapshots
# then inspect and commit .snap files
```
Or interactive: `cargo insta review`

## Architecture Patterns

### Pattern 1: `skip_serializing_if` with non-Option enum default

When changing `Option<When>` to `When` (with `#[default]` = `SessionEnd`), the field must be skipped in serialized output when it equals the default. The predicate must be a free function or method path accepted by serde:

```rust
// In criterion.rs — free function approach (Claude's discretion)
fn is_session_end(w: &When) -> bool {
    matches!(w, When::SessionEnd)
}

// On the field:
#[serde(default, skip_serializing_if = "When::is_session_end")]
pub when: When,
```

Or using an inherent method on `When`:
```rust
impl When {
    pub fn is_session_end(&self) -> bool {
        matches!(self, Self::SessionEnd)
    }
}
// Field annotation:
#[serde(default, skip_serializing_if = "When::is_session_end")]
pub when: When,
```

Both approaches work. The method approach is cleaner since it lives on the type. The string passed to `skip_serializing_if` must be a path resolvable in the field's module scope.

**Backward compat guarantee:** Old TOML with no `when` field deserializes to `When::SessionEnd` (via `#[serde(default)]`). New TOML with `when = { type = "session_end" }` also deserializes correctly. Old TOML re-serialized → `when` field absent.

### Pattern 2: Internally tagged enum with variant aliases

`When` already uses `#[serde(tag = "type", rename_all = "snake_case")]`. Apply the same pattern to `CriterionKind`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CriterionKind {
    #[serde(alias = "AgentReport")]
    AgentReport,

    #[serde(alias = "EventCount")]
    EventCount {
        event_type: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },

    #[serde(alias = "NoToolErrors")]
    NoToolErrors,
}
```

**Critical TOML caveat:** The current TOML format for unit variants is `kind = "AgentReport"` (bare string). With `tag = "type"`, the new format is a table: `kind = { type = "agent_report" }`. However, the alias `"AgentReport"` in TOML context means the deserializer expects `{ type = "AgentReport" }` — not a bare string. The bare `"AgentReport"` string will NOT be readable by an internally tagged deserializer.

This is a fundamental serde limitation: internally tagged enums cannot be deserialized from a bare string — they require an object with the tag key. The backward-compat strategy for TOML unit variants needs to use `#[serde(untagged)]` combined with a custom visitor, OR accept that old TOML with bare string format will break and require spec file migration.

**Resolution approach (for TYPE-06 planning):** The spec file `self-check.toml` uses `kind = "AgentReport"` which is the existing format. After TYPE-06, this will become `kind.type = "agent_report"`. The plan must either:
1. Migrate the spec file as part of the task, OR
2. Use a two-phase deserialization (try internally tagged, fall back to untagged string via `#[serde(untagged)]`)

Option 2 requires a custom `Deserialize` impl for `CriterionKind`. The CONTEXT.md does not address this split, but the planner needs a concrete decision.

**Recommendation:** Migrate the spec file. The `.assay/specs/self-check.toml` and `examples/close-the-loop/gates.toml` are in-repo files — update them in the same task. External user TOML files are outside scope; the alias `#[serde(alias = "AgentReport")]` only helps for the `{ type = "AgentReport" }` variant of internally tagged format, not bare strings.

### Pattern 3: Variant rename with serde alias (`CheckpointPhase::OnEvent`)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckpointPhase {
    AtToolCall { n: u32 },
    #[serde(alias = "at_event")]
    OnEvent { event_type: String },
    #[default]
    SessionEnd,
}
```

The `#[serde(alias = "at_event")]` on the variant means persisted JSON with `{ "type": "at_event", ... }` still deserializes to `OnEvent`. New serialized output writes `{ "type": "on_event", ... }`. This is safe for `GateDiagnostic.session_phase` backward compat (read old, write new).

### Pattern 4: Custom `deserialize_with` for validated integer

```rust
fn nonzero_u32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let n = u32::deserialize(d)?;
    if n == 0 {
        return Err(serde::de::Error::custom(
            "AfterToolCalls.n must be >= 1 (got 0)"
        ));
    }
    Ok(n)
}

// In When::AfterToolCalls:
AfterToolCalls {
    #[serde(deserialize_with = "nonzero_u32")]
    n: u32,
},
```

This is a free function. Since it's only used in one place, putting it as a private fn in `criterion.rs` is sufficient. No newtype needed.

### Pattern 5: `debug_assert!` in `evaluate_checkpoint`

```rust
pub fn evaluate_checkpoint(
    spec: &Spec,
    working_dir: &Path,
    events: &[AgentEvent],
    phase: CheckpointPhase,
) -> GateRunSummary {
    // SessionEnd is handled by evaluate_all_with_events, not this function.
    // Callers must never pass SessionEnd here — it is a no-op by design.
    debug_assert!(
        !matches!(phase, CheckpointPhase::SessionEnd),
        "evaluate_checkpoint called with SessionEnd phase — this is a no-op; \
         use evaluate_all_with_events for session-end evaluation"
    );
    // ... existing body ...
}
```

The existing `criterion_matches_phase` already returns `false` for `SessionEnd`, so the `debug_assert!` is purely a diagnostic aid.

## Affected Files (complete inventory)

### `crates/assay-types/src/criterion.rs`
- `When` enum: add `nonzero_u32` deserializer on `AfterToolCalls.n` (TYPE-03)
- `When`: add `is_session_end` method or free fn (TYPE-01)
- `Criterion.when`: change `Option<When>` → `When`, update `#[serde(...)]` attrs (TYPE-01)
- `CriterionKind`: add `#[serde(tag = "type", rename_all = "snake_case")]` + variant aliases (TYPE-06)
- All tests in `mod tests`: update `when: None` → `when: When::default()` or `when: When::SessionEnd` (TYPE-01, ~8 test struct literals)
- Update `criterion_when_roundtrip_pre_m024_fixture` assertion: `c.when == When::SessionEnd` not `None` (TYPE-01)

### `crates/assay-types/src/review.rs`
- Rename `SessionPhase` → `CheckpointPhase` (TYPE-02)
- Merge `AtEvent` → `OnEvent` with alias (TYPE-02, TYPE-05)
- Rename schema registry entry (TYPE-02)
- `GateDiagnostic.session_phase` field type: `CheckpointPhase` (TYPE-02)

### `crates/assay-types/tests/schema_snapshots.rs`
- Update `checkpoint_session_phase_schema_snapshot`: rename function, update type path to `assay_types::review::CheckpointPhase`, update snapshot name to `"checkpoint-phase-schema"` (TYPE-02)
- `criterion_kind_schema_snapshot`: snapshot will change (now internally tagged) (TYPE-06)
- `criterion_schema_snapshot`: snapshot changes (TYPE-01 changes `when` field representation)
- `gate_diagnostic_schema_snapshot`: snapshot changes (references `CheckpointPhase`) (TYPE-02)

### `crates/assay-types/tests/snapshots/`
- `schema_snapshots__checkpoint-session-phase-schema.snap`: replaced by `schema_snapshots__checkpoint-phase-schema.snap` (or same name if `assert_json_snapshot!` name is unchanged — but it's specified as a string literal)
- `schema_snapshots__criterion-kind-schema.snap`: updated (internally tagged, snake_case)
- `schema_snapshots__criterion-schema.snap`: updated (when field representation)
- `schema_snapshots__gate-diagnostic-schema.snap`: updated

### `crates/assay-core/src/pipeline_checkpoint.rs`
- Update import: `use assay_types::review::SessionPhase as CheckpointPhase` → `use assay_types::review::CheckpointPhase` (TYPE-02)
- `has_checkpoint_criteria`: update pattern `c.when` from `Some(...)` to direct match (TYPE-01)
- Test struct literals: ~18 `when: None` → `when: When::SessionEnd` or `when: When::AfterToolCalls { ... }` (TYPE-01)
- `make_spec_with_checkpoint`: `when: Some(when)` → `when` (TYPE-01)
- `make_spec_session_end_only`: `when: None` → `when: When::SessionEnd` (TYPE-01)

### `crates/assay-core/src/gate/mod.rs`
- Update import alias (TYPE-02)
- `criterion_matches_phase`: update match from `None | Some(When::SessionEnd)` to `When::SessionEnd` (TYPE-01)
- `criterion_matches_phase`: update `Some(When::AfterToolCalls { n })` to `When::AfterToolCalls { n }` (TYPE-01)
- `criterion_matches_phase`: update `Some(When::OnEvent { ... })` to `When::OnEvent { ... }` (TYPE-01)
- `evaluate_checkpoint`: add doc comment + `debug_assert!` (TYPE-07)
- `evaluate_checkpoint`: thread `cli_timeout` and `config_timeout` through (TYPE-04)
- Test struct literals: ~15 `when: None` → `when: When::SessionEnd` (TYPE-01)

### `crates/assay-core/src/spec/mod.rs`
- ~18 struct literal sites: `when: None` → `when: When::SessionEnd` (TYPE-01)

### `crates/assay-core/src/spec/validate.rs`
- ~12 struct literal sites: `when: None` → `when: When::SessionEnd` (TYPE-01)

### `crates/assay-core/src/spec/coverage.rs`
- 1 struct literal: `when: None` → `when: When::SessionEnd` (TYPE-01)

### `crates/assay-core/src/review/mod.rs`
- 1 reference to `assay_types::review::SessionPhase::SessionEnd` → `CheckpointPhase::SessionEnd` (TYPE-02)

### `crates/assay-cli/src/commands/spec.rs`
- Match arm on `assay_types::review::SessionPhase::AtEvent` → `CheckpointPhase::OnEvent` (TYPE-02, TYPE-05)
- Update type path from `SessionPhase` to `CheckpointPhase` (TYPE-02)

### `.assay/specs/self-check.toml`
- If TYPE-06 breaks bare string `kind = "AgentReport"`: update to `kind = { type = "agent_report" }` (TYPE-06)

### `examples/close-the-loop/gates.toml`
- `kind = "NoToolErrors"` → `kind = { type = "no_tool_errors" }` (TYPE-06)

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Integer validation | Custom serde type | `deserialize_with` free fn | One-liner, serde idiom |
| Enum backward compat | Data migration script | `#[serde(alias = ...)]` | Zero runtime cost |
| Snapshot comparison | String diff | `insta` + `cargo insta review` | Already in workspace |
| Schema regeneration | Manual JSON edit | `INSTA_UPDATE=always` + nextest | Automated |

## Common Pitfalls

### Pitfall 1: Internally Tagged + TOML bare string unit variants
**What goes wrong:** After adding `#[serde(tag = "type")]` to `CriterionKind`, TOML files with `kind = "AgentReport"` (bare string) fail to deserialize. The internally-tagged deserializer expects a map `{ type = "agent_report" }`, not a string.
**Why it happens:** Internal tagging requires an object with the discriminant key. TOML bare strings cannot carry a tag key.
**How to avoid:** Either (a) migrate TOML files in the same task, or (b) implement a two-phase deserializer using `#[serde(untagged)]` with a helper enum. Approach (a) is simpler and in-scope for this phase.
**Warning signs:** `criterion-kind-schema` snapshot shows `"const": "AgentReport"` for unit variants — once changed to internally tagged, unit variants become `{ type = "agent_report" }` objects.

### Pitfall 2: `skip_serializing_if` path resolution scope
**What goes wrong:** Using `"is_session_end"` without a qualifying path fails if the fn is not in scope at the field's module.
**Why it happens:** Serde macro resolves the string as a path in the field's definition scope.
**How to avoid:** Use the fully-qualified path `"When::is_session_end"` (method) or `"crate::criterion::is_session_end"` (free fn in same module — just `"is_session_end"` works too since it's in the same module).

### Pitfall 3: Missing snapshot file for renamed schema entry
**What goes wrong:** `assert_json_snapshot!("checkpoint-phase-schema", ...)` looks for `schema_snapshots__checkpoint-phase-schema.snap`, which doesn't exist. Test fails with "snapshot not found".
**Why it happens:** `insta` requires the `.snap` file to exist or `INSTA_UPDATE` mode to create it.
**How to avoid:** Run `INSTA_UPDATE=always cargo nextest run -p assay-types --test schema_snapshots` after all type changes. Commit all new/updated `.snap` files.

### Pitfall 4: `#[serde(alias)]` on struct variant with internally tagged doesn't accept old format
**What goes wrong:** `#[serde(alias = "at_event")]` on `OnEvent` in `CheckpointPhase` makes the deserializer accept `{ "type": "at_event" }` as well as `{ "type": "on_event" }`. This IS correct for the intended use case (backward compat for persisted JSON). The pitfall is believing it also accepts `{ "type": "AtEvent" }` (PascalCase) — it does not unless a second alias is added.
**How to avoid:** The existing persisted format is `at_event` (snake_case, from the original `rename_all = "snake_case"`) — the alias is correct as-is.

### Pitfall 5: `debug_assert!` in evaluate_checkpoint called from existing tests
**What goes wrong:** Existing test `fn evaluate_checkpoint_with_session_end_phase_skips_all_criteria` (line 3284 in gate/mod.rs) passes `CheckpointPhase::SessionEnd` to `evaluate_checkpoint`. With the new `debug_assert!`, this test panics in debug builds.
**Why it happens:** The test was written to verify the no-op behavior, but the assert makes that exact call illegal in debug mode.
**How to avoid:** The test should be updated to verify the behavior via `criterion_matches_phase` directly, or the test comment should document that `SessionEnd` is handled by `evaluate_all_with_events`. Check line 3284 — the test exists and must be updated.

### Pitfall 6: TYPE-01 struct literal count underestimation
**What goes wrong:** grep shows ~110 `when:` usages, ~110 of which are `when: None`. Mechanical update is required at all sites. Missing one causes a compile error (field type mismatch) — but also, any test that checks `c.when == None` must be updated to `c.when == When::SessionEnd`.
**Warning signs:** The pre-M024 fixture test checks `c.when == None` explicitly and must become `c.when == When::SessionEnd`.

## Code Examples

### Example 1: Complete `When` enum after TYPE-01 + TYPE-03

```rust
// Source: crates/assay-types/src/criterion.rs (post-phase)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum When {
    #[default]
    SessionEnd,
    AfterToolCalls {
        #[serde(deserialize_with = "nonzero_u32")]
        n: u32,
    },
    OnEvent {
        event_type: String,
    },
}

impl When {
    pub fn is_session_end(&self) -> bool {
        matches!(self, Self::SessionEnd)
    }
}

fn nonzero_u32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let n = u32::deserialize(d)?;
    if n == 0 {
        return Err(serde::de::Error::custom(
            "AfterToolCalls.n must be >= 1 (got 0)"
        ));
    }
    Ok(n)
}
```

### Example 2: `Criterion.when` field after TYPE-01

```rust
// Before:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub when: Option<When>,

// After:
#[serde(default, skip_serializing_if = "When::is_session_end")]
pub when: When,
```

### Example 3: `criterion_matches_phase` after TYPE-01

```rust
fn criterion_matches_phase(
    criterion: &Criterion,
    phase: &CheckpointPhase,
    events: &[AgentEvent],
) -> bool {
    match &criterion.when {
        When::SessionEnd => false,
        When::AfterToolCalls { n } => {
            matches!(phase, CheckpointPhase::AtToolCall { n: current } if current == n)
        }
        When::OnEvent { event_type } => events
            .last()
            .is_some_and(|last| event_serde_tag(last) == event_type),
    }
}
```

### Example 4: `CheckpointPhase` after TYPE-02 + TYPE-05

```rust
// Source: crates/assay-types/src/review.rs (post-phase)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CheckpointPhase {
    AtToolCall { n: u32 },
    #[serde(alias = "at_event")]
    OnEvent { event_type: String },
    #[default]
    SessionEnd,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "checkpoint-phase",
        generate: || schemars::schema_for!(CheckpointPhase),
    }
}
```

### Example 5: `CriterionKind` after TYPE-06 (new format)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CriterionKind {
    #[serde(alias = "AgentReport")]
    AgentReport,
    #[serde(alias = "EventCount")]
    EventCount {
        event_type: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
    },
    #[serde(alias = "NoToolErrors")]
    NoToolErrors,
}
// NOTE: Aliases only help for { type = "AgentReport" } TOML format.
// Bare string `kind = "AgentReport"` TOML must be migrated to
// kind = { type = "agent_report" } in-repo spec files.
```

### Example 6: TYPE-04 — threading timeout through `evaluate_checkpoint`

Current signature (gate/mod.rs line 1063):
```rust
pub fn evaluate_checkpoint(
    spec: &Spec,
    working_dir: &Path,
    events: &[AgentEvent],
    phase: CheckpointPhase,
) -> GateRunSummary
```

After TYPE-04:
```rust
pub fn evaluate_checkpoint(
    spec: &Spec,
    working_dir: &Path,
    events: &[AgentEvent],
    phase: CheckpointPhase,
    cli_timeout: Option<u64>,
    config_timeout: Option<u64>,
) -> GateRunSummary
```

The inner call changes from:
```rust
evaluate_all_with_events(&checkpoint_spec, working_dir, None, None, events)
```
to:
```rust
evaluate_all_with_events(&checkpoint_spec, working_dir, cli_timeout, config_timeout, events)
```

Call site in `pipeline_checkpoint.rs` (line 115) must be updated to pass timeout args from `drive_checkpoints`. The `drive_checkpoints` function signature already receives `timeout: Duration` but that is the driver-level wall-clock budget, not the per-criterion timeout. For TYPE-04, the fix is threading the spec-level `cli_timeout`/`config_timeout` into `evaluate_checkpoint`, not the driver timeout. The `drive_checkpoints` function needs two new optional parameters or the caller must pass them.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest (workspace) |
| Config file | none (nextest default) |
| Quick run command | `cargo nextest run -p assay-types -p assay-core` |
| Full suite command | `cargo nextest run --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TYPE-01 | `when: When::SessionEnd` round-trips without emitting `when` key | unit | `cargo nextest run -p assay-types` | ✅ (update existing) |
| TYPE-01 | Pre-M024 fixture: `c.when == When::SessionEnd` | unit | `cargo nextest run -p assay-types` | ✅ (update existing) |
| TYPE-02 | `CheckpointPhase::OnEvent` deserializes old `{ "type": "at_event" }` JSON | unit | `cargo nextest run -p assay-types` | ❌ Wave 0 |
| TYPE-03 | `AfterToolCalls { n: 0 }` returns deserialization error | unit | `cargo nextest run -p assay-types` | ❌ Wave 0 |
| TYPE-04 | `evaluate_checkpoint` applies cli_timeout to criterion evaluation | unit | `cargo nextest run -p assay-core` | ❌ Wave 0 |
| TYPE-05 | `OnEvent` variant exists in `CheckpointPhase`; schema includes it | unit | `cargo nextest run -p assay-types` | ❌ Wave 0 (schema snapshot) |
| TYPE-06 | `CriterionKind::AgentReport` round-trips as `{ type = "agent_report" }` | unit | `cargo nextest run -p assay-types` | ❌ Wave 0 |
| TYPE-06 | Old `{ type = "AgentReport" }` deserializes via alias | unit | `cargo nextest run -p assay-types` | ❌ Wave 0 |
| TYPE-07 | `debug_assert!` present (code review); doc comment on `evaluate_checkpoint` | manual | n/a | n/a |

### Sampling Rate
- **Per task commit:** `cargo nextest run -p assay-types -p assay-core`
- **Per wave merge:** `cargo nextest run --workspace`
- **Phase gate:** Full suite green (`just ready`) before `/kata:verify-work`

### Wave 0 Gaps
- [ ] New test: `checkpoint_phase_on_event_alias_roundtrip` in `assay-types/src/review.rs` — covers TYPE-02, TYPE-05
- [ ] New test: `when_after_tool_calls_zero_rejected` in `assay-types/src/criterion.rs` — covers TYPE-03
- [ ] New test: `criterion_kind_internally_tagged_roundtrip` in `assay-types/src/criterion.rs` — covers TYPE-06 new format
- [ ] New test: `criterion_kind_agent_report_alias_roundtrip` in `assay-types/src/criterion.rs` — covers TYPE-06 alias
- [ ] New test: `evaluate_checkpoint_threads_cli_timeout` in `assay-core/src/gate/mod.rs` — covers TYPE-04
- [ ] Update snapshot: `schema_snapshots__checkpoint-session-phase-schema.snap` → replaced by `checkpoint-phase-schema`
- [ ] Update snapshot: `schema_snapshots__criterion-kind-schema.snap` — internally tagged format
- [ ] Update snapshot: `schema_snapshots__criterion-schema.snap` — `when` field type change
- [ ] Update snapshot: `schema_snapshots__gate-diagnostic-schema.snap` — `CheckpointPhase` reference

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Option<When>` with None as default | `When` with `#[default]` + `skip_serializing_if` | This phase | Eliminates ambiguity between absent and explicit SessionEnd |
| `SessionPhase` in review module (confusing name) | `CheckpointPhase` (specific) | This phase | No more `as CheckpointPhase` alias imports needed |
| `CriterionKind` untagged PascalCase | `CriterionKind` internally tagged snake_case | This phase | Consistent with `When` enum style |

**Deprecated/outdated after this phase:**
- `AtEvent` variant: replaced by `OnEvent` (alias remains for read-compat)
- `review::SessionPhase` name: replaced by `review::CheckpointPhase` (no alias at type level)
- `kind = "AgentReport"` TOML format: replaced by `kind = { type = "agent_report" }` (in-repo specs migrated)

## Open Questions

1. **TYPE-06: TOML bare string backward compat for external users**
   - What we know: Internally tagged enums cannot deserialize from bare strings. In-repo specs can be migrated.
   - What's unclear: Are there external users with `kind = "AgentReport"` in their spec files? Phase scope doesn't include external migration guides.
   - Recommendation: Proceed with migration + add a prominent note in changelog/commit message that `kind = "AgentReport"` format changes to `kind = { type = "agent_report" }`.

2. **TYPE-04: `drive_checkpoints` signature change**
   - What we know: `drive_checkpoints` receives `timeout: Duration` (driver budget). `evaluate_checkpoint` needs separate `cli_timeout: Option<u64>` and `config_timeout: Option<u64>`.
   - What's unclear: Where do `cli_timeout`/`config_timeout` come from at the `drive_checkpoints` call site? They must be threaded from the pipeline caller.
   - Recommendation: Add `cli_timeout: Option<u64>` and `config_timeout: Option<u64>` params to `drive_checkpoints`, thread through to `evaluate_checkpoint`. The existing TYPE-04 scope is the documentation + threading; no new per-criterion timeout logic is added.

3. **`debug_assert!` and existing test at line 3284**
   - What we know: A test passes `CheckpointPhase::SessionEnd` to `evaluate_checkpoint` to verify the no-op behavior. The `debug_assert!` will panic in debug mode.
   - What's unclear: Whether to delete, comment, or restructure the test.
   - Recommendation: Replace test with a direct call to `criterion_matches_phase` with `SessionEnd` phase, asserting `false`. Or verify behavior via `evaluate_checkpoint` with a non-debug build guard. Simplest: delete the test (the no-op behavior is now enforced by assertion + docs, not a soft test).

## Sources

### Primary (HIGH confidence)
- Direct code read: `crates/assay-types/src/criterion.rs` — `When`, `Criterion`, `CriterionKind` exact definitions
- Direct code read: `crates/assay-types/src/review.rs` — `SessionPhase` exact definition
- Direct code read: `crates/assay-core/src/gate/mod.rs` — `evaluate_checkpoint`, `criterion_matches_phase`
- Direct code read: `crates/assay-core/src/pipeline_checkpoint.rs` — import alias, all call sites
- Direct code read: `crates/assay-cli/src/commands/spec.rs` — `SessionPhase` match arms
- Direct code read: `crates/assay-types/tests/schema_snapshots.rs` — snapshot test list
- grep search: all `when: None` / `when: Some` usages across workspace (~110 sites)

### Secondary (MEDIUM confidence)
- serde documentation (training knowledge, stable API): `#[serde(alias)]`, `#[serde(tag)]`, `#[serde(deserialize_with)]`, `skip_serializing_if` semantics are well-documented and stable
- TOML crate serde integration: internally tagged enum + TOML limitation is a known serde constraint (structs required for tag lookup; bare strings incompatible with internal tagging)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all workspace deps already present
- Architecture: HIGH — all patterns verified against existing codebase usages
- Pitfalls: HIGH — TOML bare string limitation is a fundamental serde constraint; debug_assert conflict verified by reading line 3284 in gate/mod.rs
- Affected files: HIGH — verified by grep across entire workspace

**Research date:** 2026-04-09
**Valid until:** 2026-05-09 (stable Rust/serde APIs; no external dependencies change)
