# Phase 61: Type Correctness & Serde Consistency — UAT

**Date:** 2026-04-09
**Phase:** 61-type-correctness-serde-consistency
**Status:** PASSED (9/9)

## Tests

| # | Test | Status | Notes |
|---|------|--------|-------|
| 1 | SessionPhase fully renamed to CheckpointPhase (no old refs in review code) | PASS | All remaining SessionPhase refs are work_session::SessionPhase (unrelated type) |
| 2 | CheckpointPhase::OnEvent deserializes from `at_event` JSON tag | PASS | `#[serde(alias = "at_event")]` on OnEvent, roundtrip test at review.rs:211 |
| 3 | Criterion.when is non-optional When (no Option<When> in code) | PASS | Zero matches for Option<When>, when: None, when: Some across all crates |
| 4 | AfterToolCalls { n: 0 } rejected at deserialization | PASS | nonzero_u32 deserializer rejects with "must be >= 1" message |
| 5 | CriterionKind uses internally tagged serde | PASS | `#[serde(tag = "type", rename_all = "snake_case")]` at criterion.rs:19 |
| 6 | Old PascalCase format still deserializes via alias | PASS | AgentReport, NoToolErrors, EventCount aliases on all variants |
| 7 | In-repo TOML specs migrated to new kind format | PASS | self-check.toml and gates.toml use `kind = { type = "..." }` |
| 8 | evaluate_checkpoint has debug_assert for SessionEnd | PASS | debug_assert at gate/mod.rs:1077, doc comment at line 1062 |
| 9 | evaluate_checkpoint threads cli_timeout/config_timeout | PASS | Signature accepts both params at lines 1072-1073, precedence documented |
