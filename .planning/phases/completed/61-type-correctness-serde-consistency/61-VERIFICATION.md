---
phase: 61-type-correctness-serde-consistency
verified: 2026-04-09T00:00:00Z
status: passed
score: 12/12 must-haves verified
re_verification: false
---

# Phase 61: Type Correctness & Serde Consistency — Verification Report

**Phase Goal:** Fix representational ambiguities and serde tagging inconsistencies in checkpoint/criterion types
**Verified:** 2026-04-09
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `review::CheckpointPhase` is the canonical type name — no `SessionPhase` exists | VERIFIED | `pub enum CheckpointPhase` in `crates/assay-types/src/review.rs:179`; zero matches for `review::SessionPhase` across all crates |
| 2 | `CheckpointPhase::OnEvent` deserializes from both `on_event` and `at_event` JSON tags | VERIFIED | `#[serde(alias = "at_event")]` on `OnEvent` at line 189; test `checkpoint_phase_on_event_alias_roundtrip` covers both paths |
| 3 | `evaluate_checkpoint` has a doc comment explaining SessionEnd is a no-op | VERIFIED | Doc block at lines 1062–1068 in `gate/mod.rs` with "SessionEnd is a no-op" section |
| 4 | `debug_assert!` fires if SessionEnd is passed to `evaluate_checkpoint` in debug builds | VERIFIED | `debug_assert!(!matches!(phase, CheckpointPhase::SessionEnd), ...)` at lines 1077–1081 |
| 5 | `Criterion.when` is `When` (not `Option<When>`) — compiler enforces at all sites | VERIFIED | `pub when: When` at `criterion.rs:176`; zero `Option<When>` matches across codebase |
| 6 | Omitting `when` in TOML/JSON deserializes to `When::SessionEnd` via `#[serde(default)]` | VERIFIED | `#[serde(default, skip_serializing_if = "When::is_session_end")]` on the field; `criterion_when_roundtrip_pre_m024_fixture` test covers pre-M024 fixture |
| 7 | Serializing a criterion with `When::SessionEnd` omits the `when` key | VERIFIED | `skip_serializing_if = "When::is_session_end"` on `when` field; `criterion_when_omitted_when_session_end` test asserts absence of "when" in output |
| 8 | `When::AfterToolCalls { n: 0 }` fails deserialization with clear error message | VERIFIED | `nonzero_u32` custom deserializer rejects 0 with "AfterToolCalls.n must be >= 1 (got 0)"; `when_after_tool_calls_zero_rejected` test asserts error message content |
| 9 | `CriterionKind` uses internally tagged serde with `tag = "type"` and `rename_all = "snake_case"` | VERIFIED | `#[serde(tag = "type", rename_all = "snake_case")]` at `criterion.rs:19`; test `criterion_kind_internally_tagged_roundtrip` covers all variants |
| 10 | Old PascalCase format (`{ type = "AgentReport" }`) deserializes via alias | VERIFIED | `#[serde(alias = "AgentReport")]`, `#[serde(alias = "EventCount")]`, `#[serde(alias = "NoToolErrors")]` on respective variants; `criterion_kind_alias_roundtrip` test |
| 11 | In-repo TOML spec files use new `kind = { type = "agent_report" }` format | VERIFIED | `.assay/specs/self-check.toml:30` has `kind = { type = "agent_report" }`; `examples/close-the-loop/gates.toml` uses `kind = { type = "no_tool_errors" }` and `[criteria.kind] type = "event_count"` |
| 12 | `evaluate_checkpoint` passes `cli_timeout` and `config_timeout` through to `evaluate_all_with_events` | VERIFIED | Signature at `gate/mod.rs:1069–1076` takes `cli_timeout: Option<u64>` and `config_timeout: Option<u64>`; inner call at lines 1105–1111 passes them to `evaluate_all_with_events`; `drive_checkpoints` in `pipeline_checkpoint.rs:70–79` also accepts and threads both params |

**Score:** 12/12 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/assay-types/src/review.rs` | `CheckpointPhase` enum with `OnEvent` variant and `at_event` alias | VERIFIED | Lines 179–197; `OnEvent` with `#[serde(alias = "at_event")]`; schema entry renamed to `"checkpoint-phase"` |
| `crates/assay-core/src/gate/mod.rs` | `evaluate_checkpoint` doc comment and `debug_assert` | VERIFIED | Doc block lines 1057–1068; `debug_assert!` lines 1077–1081 |
| `crates/assay-types/src/criterion.rs` | `When::is_session_end` method, `nonzero_u32` deserializer, `Criterion.when` as `When` | VERIFIED | `is_session_end` at line 96; `nonzero_u32` at line 102; `pub when: When` at line 176 |
| `crates/assay-types/src/criterion.rs` | `CriterionKind` with internally tagged serde and aliases | VERIFIED | `#[serde(tag = "type", rename_all = "snake_case")]` at line 19; three variant aliases |
| `.assay/specs/self-check.toml` | Migrated spec with `kind.type = "agent_report"` | VERIFIED | Line 30: `kind = { type = "agent_report" }` |
| `examples/close-the-loop/gates.toml` | Migrated example spec with new kind format | VERIFIED | Uses `type = "event_count"` and `kind = { type = "no_tool_errors" }` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `pipeline_checkpoint.rs` | `review.rs` | `use assay_types::review::CheckpointPhase` (direct, no alias) | VERIFIED | Line 19: `use assay_types::review::CheckpointPhase;` — no `as` alias |
| `gate/mod.rs` | `review.rs` | `use assay_types::review::CheckpointPhase` (direct) | VERIFIED | Line 1037: `::review::CheckpointPhase;` — direct import |
| `gate/mod.rs` | `criterion.rs` | `criterion.when` direct match (no `Option` unwrap) | VERIFIED | `criterion_matches_phase` at line 1120 matches `&criterion.when` with no `Option` wrappers |
| `pipeline_checkpoint.rs` | `gate/mod.rs` | `evaluate_checkpoint` call with timeout params | VERIFIED | Lines 118–125: `evaluate_checkpoint(spec, working_dir, cli_timeout, config_timeout, buffer, phase.clone())` |
| `.assay/specs/self-check.toml` | `criterion.rs` | TOML deserialization of `CriterionKind` | VERIFIED | `kind = { type = "agent_report" }` matches `#[serde(tag = "type", rename_all = "snake_case")]` on `CriterionKind` |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TYPE-01 | Plan 02 | `Criterion.when: Option<When>` ambiguity resolved | SATISFIED | `pub when: When` in criterion.rs; `#[serde(default)]` + `skip_serializing_if` replaces None/Some split |
| TYPE-02 | Plan 01 | `review::SessionPhase` renamed to `CheckpointPhase` | SATISFIED | `pub enum CheckpointPhase` in review.rs; zero remaining `review::SessionPhase` references |
| TYPE-03 | Plan 02 | `When::AfterToolCalls { n: 0 }` rejected by validation | SATISFIED | `nonzero_u32` custom deserializer; `when_after_tool_calls_zero_rejected` test |
| TYPE-04 | Plan 03 | `evaluate_checkpoint` respects CLI/config timeout overrides | SATISFIED | Signature includes `cli_timeout` and `config_timeout`; threaded to `evaluate_all_with_events` |
| TYPE-05 | Plan 01 | `review::CheckpointPhase` includes `OnEvent` variant (merged from `AtEvent`) | SATISFIED | `OnEvent` with `#[serde(alias = "at_event")]`; `AtEvent` variant no longer exists |
| TYPE-06 | Plan 03 | `CriterionKind` serde tagging consistent with `When` enum | SATISFIED | Both use `#[serde(tag = "type", rename_all = "snake_case")]` |
| TYPE-07 | Plan 01 | `evaluate_checkpoint` at `SessionEnd` documents no-op behavior | SATISFIED | Doc comment + `debug_assert!` in `gate/mod.rs:1062–1081` |

---

### Anti-Patterns Found

None detected. No TODOs, FIXMEs, stubs, or placeholder returns in the modified files.

---

### Human Verification Required

None — all must-haves are verifiable programmatically.

---

### Test Suite Result

1158 tests passed, 4 ignored across assay-types and assay-core. Zero failures.

---

## Summary

All 12 must-haves from all three plan waves are implemented, wired, and tested. The phase goal — eliminating representational ambiguities and serde tagging inconsistencies in checkpoint/criterion types — is fully achieved:

- `review::CheckpointPhase` replaces `review::SessionPhase` throughout (TYPE-02, TYPE-05)
- `Criterion.when` is non-optional with `SessionEnd` as the explicit default (TYPE-01)
- `When::AfterToolCalls { n: 0 }` is rejected at deserialization time (TYPE-03)
- `CriterionKind` now uses the same internally tagged `snake_case` format as `When` (TYPE-06)
- `evaluate_checkpoint` documents and asserts the SessionEnd no-op contract (TYPE-07)
- CLI/config timeout overrides thread through the full call chain (TYPE-04)

---

_Verified: 2026-04-09_
_Verifier: Claude (kata-verifier)_
