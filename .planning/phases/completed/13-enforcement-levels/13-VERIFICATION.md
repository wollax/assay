# Phase 13 Verification

**Phase goal:** Add required/advisory enforcement to criteria so gate evaluation distinguishes blocking failures from informational warnings.

**Verification date:** 2026-03-04
**Method:** Goal-backward — verified actual code, not SUMMARY claims.
**Test suite result:** All tests pass, lint clean.

---

## ENFC-01: Criterion has an `enforcement` field with values `required` (default) and `advisory`

**PASS**

- `Enforcement` enum exists at `crates/assay-types/src/enforcement.rs:15` with `#[default]` on `Required` and `Advisory` variant, serializing as `kebab-case` (`"required"`, `"advisory"`).
- `Criterion.enforcement: Option<Enforcement>` exists at `crates/assay-types/src/criterion.rs:41` with `#[serde(skip_serializing_if = "Option::is_none", default)]` — absent field deserializes as `None`, providing backward compat.
- `GateCriterion.enforcement: Option<Enforcement>` exists at `crates/assay-types/src/gates_spec.rs:41` with identical serde attrs.

---

## ENFC-02: Gate evaluation summary separates required failures from advisory failures

**PASS**

- `EnforcementSummary` struct exists at `crates/assay-types/src/enforcement.rs:54` with `required_passed`, `required_failed`, `advisory_passed`, `advisory_failed` fields, all `usize`, all defaulting to 0 via `#[derive(Default)]`.
- `GateRunSummary.enforcement: EnforcementSummary` exists at `crates/assay-types/src/gate_run.rs:32` with `#[serde(default)]`.
- `CriterionResult.enforcement: Enforcement` exists at `crates/assay-types/src/gate_run.rs:45` — always resolved (not `Option`), defaults to `required`.

---

## Success Criterion 1: Criterion without explicit `enforcement` deserializes with `required` as default

**PASS**

- `Enforcement` derives `Default` with `#[default]` on `Required`.
- `Criterion.enforcement` is `Option<Enforcement>` with `#[serde(default)]`, so absent field → `None`.
- `resolve_enforcement(None, None)` → `Enforcement::Required` (verified in test `resolve_enforcement_precedence` at `crates/assay-core/src/gate/mod.rs:1165`).

---

## Success Criterion 2: `evaluate_all()` returns summary with required and advisory failure counts reported separately

**PASS**

- `evaluate_all()` at `crates/assay-core/src/gate/mod.rs:86` populates `enforcement_summary` per-criterion on pass, fail, and spawn-error branches.
- `evaluate_all_gates()` at `crates/assay-core/src/gate/mod.rs:177` does the same for directory-based specs.
- Skipped criteria (no `cmd`, no `path`) increment `skipped` only — not counted in `enforcement_summary` (verified by test `evaluate_all_skipped_excluded_from_enforcement`).
- Tests covering enforcement tracking: `evaluate_all_advisory_failure_does_not_block`, `evaluate_all_skipped_excluded_from_enforcement`, `evaluate_all_gates_enforcement_tracking`, `all_required_pass_advisory_failures_still_pass`.

---

## Success Criterion 3: Gate with only advisory failures reports overall `passed: true`

**PASS (indirectly — enforcement summary is the mechanism)**

The `GateRunSummary` struct does not carry a top-level `passed: bool`; the "gate passes" signal is expressed by `enforcement.required_failed == 0`. The CLI exit code at `crates/assay-cli/src/main.rs:686` exits 1 only when `counters.required_failed > 0`; the JSON path at line 843 uses `summary.enforcement.required_failed > 0`. The `--all` JSON path at line 737 uses `summaries.iter().any(|s| s.enforcement.required_failed > 0)`.

Test `evaluate_all_advisory_failure_does_not_block` confirms `required_failed == 0` when a required criterion passes and an advisory criterion fails.

---

## Success Criterion 4: Gate with any required failure reports `passed: false` (exit 1)

**PASS**

- `print_gate_summary()` (`crates/assay-cli/src/main.rs:675`) calls `std::process::exit(1)` when `counters.required_failed > 0`.
- `stream_criterion()` (`crates/assay-cli/src/main.rs:630`) increments `counters.required_failed` only when `enforcement == Enforcement::Required`.
- JSON single-spec path (`main.rs:843`) and `--all` JSON path (`main.rs:737`) both use `enforcement.required_failed > 0` for exit code.

---

## Plan 01 Artifacts

| Artifact | Status | Notes |
|---|---|---|
| `crates/assay-types/src/enforcement.rs` | PRESENT | `Enforcement`, `GateSection`, `EnforcementSummary` all defined |
| `crates/assay-types/src/lib.rs` | PRESENT | `pub mod enforcement` + re-exports `Enforcement`, `EnforcementSummary`, `GateSection`; `gate: Option<GateSection>` on `Spec` |
| `crates/assay-types/src/criterion.rs` | PRESENT | `enforcement: Option<Enforcement>` field added |
| `crates/assay-types/src/gates_spec.rs` | PRESENT | `enforcement: Option<Enforcement>` on `GateCriterion`; `gate: Option<GateSection>` on `GatesSpec` |
| `crates/assay-types/src/gate_run.rs` | PRESENT | `enforcement: Enforcement` on `CriterionResult`; `enforcement: EnforcementSummary` on `GateRunSummary` |

---

## Plan 02 Artifacts

| Artifact | Status | Notes |
|---|---|---|
| `resolve_enforcement()` | PRESENT | `crates/assay-core/src/gate/mod.rs:294`; precedence: criterion > gate section > Required |
| `evaluate_all()` | PRESENT | Populates `EnforcementSummary` correctly; skipped excluded |
| `evaluate_all_gates()` | PRESENT | Equivalent population for `GatesSpec` |
| `validate()` at-least-one-required | PRESENT | `crates/assay-core/src/spec/mod.rs:131`; rejects specs where no executable criterion resolves to Required |
| `validate_gates_spec()` at-least-one-required | PRESENT | `crates/assay-core/src/spec/mod.rs:373`; same rule for directory-based specs |

---

## Plan 03 Artifacts

| Artifact | Status | Notes |
|---|---|---|
| CLI exit code uses `required_failed` | PRESENT | `main.rs:686`, `main.rs:737`, `main.rs:843` all check `required_failed > 0` |
| `spec new` template includes `[gate]` section | PRESENT | `main.rs:551-552`: `[gate]\nenforcement = "required"` |
| `schema_snapshots__enforcement-schema.snap` | PRESENT | Correct `oneOf` with `required` and `advisory` |
| `schema_snapshots__enforcement-summary-schema.snap` | PRESENT | Correct with all four count fields |
| `schema_snapshots__gate-section-schema.snap` | PRESENT | Extra snapshot beyond plan requirement; covers `GateSection` |

---

## Deviations and Gaps

**None material.** One minor deviation: the plan listed `schema_snapshots__enforcement-schema.snap` and `schema_snapshots__enforcement-summary-schema.snap` but the implementation also added `schema_snapshots__gate-section-schema.snap`. This is additive and correct.

The plan listed `gate_run.rs` carrying `enforcement` on `CriterionResult` as "always Required or Advisory, not Option". Actual code: `#[serde(default)]` with `pub enforcement: Enforcement` — no `Option`, defaults to `Required` on deserialization. This satisfies the intent.

---

## Overall Verdict

**GOAL ACHIEVED.**

All four success criteria are met. All plan artifacts exist and are wired correctly. The full test suite passes (17 schema snapshot tests, all unit tests in `assay-core` and `assay-types`). Clippy is clean with `-D warnings`.
