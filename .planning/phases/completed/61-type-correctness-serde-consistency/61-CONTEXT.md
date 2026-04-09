# Phase 61: Type Correctness & Serde Consistency - Context

**Gathered:** 2026-04-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix representational ambiguities and serde tagging inconsistencies in checkpoint/criterion types. 7 requirements (TYPE-01 through TYPE-07). No new features — purely correctness and consistency fixes across `assay-types` and `assay-core`.

</domain>

<decisions>
## Implementation Decisions

### Criterion.when ambiguity (TYPE-01)
- Make `when` non-optional: change `Option<When>` to `When` with `#[serde(default)]`
- Omit `when` from serialized output when value is `SessionEnd` (use `skip_serializing_if`)
- Update all consumers — compiler will flag every site via type error; fix them all in this phase
- Update pre-M024 roundtrip test to assert `when == When::SessionEnd` instead of `when == None`

### SessionPhase rename (TYPE-02, TYPE-05)
- Rename `review::SessionPhase` to `review::CheckpointPhase` at the type level (not just alias)
- Remove the `as CheckpointPhase` alias in `pipeline_checkpoint.rs` — use direct import
- Merge `AtEvent` and `OnEvent` into single `OnEvent` variant (semantically identical)
- Add `#[serde(alias = "at_event")]` on `OnEvent` for backward compat with persisted `GateDiagnostic` JSON
- Rename schema registry entry from `"checkpoint-session-phase"` to `"checkpoint-phase"`

### Serde tagging alignment (TYPE-06)
- Align `CriterionKind` to internally tagged: add `#[serde(tag = "type", rename_all = "snake_case")]`
- Add serde aliases for old PascalCase format (`#[serde(alias = "AgentReport")]` etc.) for backward compat with existing spec TOML files and gate run JSON
- Keep PascalCase for the `Display` impl (human-readable output, different purpose than serde)

### Validation (TYPE-03)
- Reject `AfterToolCalls { n: 0 }` at deserialize time via `#[serde(deserialize_with = "nonzero_u32")]`
- Error message: "AfterToolCalls.n must be >= 1 (got 0)"

### Timeout overrides (TYPE-04)
- Keep single driver timeout parameter (per-criterion timeouts already handled in `gate/mod.rs`)
- Document timeout precedence: CLI > config > spec-level > default

### SessionEnd no-op documentation (TYPE-07)
- Add doc comment on `evaluate_checkpoint` explaining SessionEnd is a no-op (criteria never match)
- Add `debug_assert!` that SessionEnd phase is never passed to `evaluate_checkpoint`

### Claude's Discretion
- Exact `nonzero_u32` deserializer implementation approach (custom fn vs newtype)
- Whether `is_session_end` helper is a free function or method on `When`
- Test organization for new serde alias roundtrip tests

</decisions>

<specifics>
## Specific Ideas

- Backward compatibility via serde aliases is the consistent pattern across all breaking serde changes in this phase (CheckpointPhase variant rename, CriterionKind tagging change)
- All serde changes produce new output format but accept old input — smooth migration, no data fixup scripts needed

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/assay-types/src/criterion.rs`: `When` enum and `Criterion` struct — primary targets for TYPE-01, TYPE-03
- `crates/assay-types/src/review.rs`: `SessionPhase` enum — primary target for TYPE-02, TYPE-05
- `crates/assay-core/src/pipeline_checkpoint.rs`: `CheckpointPhase` alias consumer, `drive_checkpoints`, `evaluate_checkpoint` call site
- `crates/assay-core/src/gate/mod.rs`: `evaluate_checkpoint` function — target for TYPE-04, TYPE-07
- `crates/assay-types/src/criterion.rs`: `CriterionKind` enum — target for TYPE-06
- Schema registry entries in both `criterion.rs` and `review.rs`

### Established Patterns
- Internally tagged serde: `#[serde(tag = "type", rename_all = "snake_case")]` — already used on `When`, to be applied to `CriterionKind` and `CheckpointPhase`
- `skip_serializing_if` for optional/default fields — widely used across `Criterion` fields
- `inventory::submit!` for schema registry — all modified enums have registry entries
- Pre-M024 backward compat tests (`criterion_when_roundtrip_pre_m024_fixture`) — pattern to follow for new alias tests

### Integration Points
- `pipeline_checkpoint.rs` imports `SessionPhase as CheckpointPhase` — will become direct `CheckpointPhase` import
- `gate/mod.rs` `evaluate_checkpoint` and `criterion_matches_phase` — consumers of both `When` and `CheckpointPhase`
- `GateDiagnostic.session_phase` field — persisted JSON uses old variant names, needs alias support
- Schema snapshots in `assay-types/tests/schema_snapshots.rs` — will need updating after enum changes

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 61-type-correctness-serde-consistency*
*Context gathered: 2026-04-08*
