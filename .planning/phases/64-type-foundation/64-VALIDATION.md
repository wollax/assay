---
phase: 64
slug: type-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-11
---

# Phase 64 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + insta 1.46 |
| **Config file** | `crates/assay-types/Cargo.toml` (dev-deps: insta, toml, jsonschema) |
| **Quick run command** | `cargo test -p assay-types` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-types`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/kata:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 64-01-01 | 01 | 1 | INHR-01 | unit (TOML roundtrip) | `cargo test -p assay-types gates_spec_with_extends` | ❌ W0 | ⬜ pending |
| 64-01-02 | 01 | 1 | INHR-02 | unit (TOML roundtrip) | `cargo test -p assay-types gates_spec_with_include` | ❌ W0 | ⬜ pending |
| 64-01-03 | 01 | 1 | SAFE-03 | unit (backward compat) | `cargo test -p assay-types legacy_toml_without_composability` | ❌ W0 | ⬜ pending |
| 64-01-04 | 01 | 1 | SAFE-03 | snapshot | `cargo test -p assay-types schema_snapshots` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] New test functions in `crates/assay-types/src/gates_spec.rs` — covers INHR-01, INHR-02, SAFE-03 (backward compat)
- [ ] New test functions in `crates/assay-types/src/criteria_library.rs` — covers CriteriaLibrary roundtrip
- [ ] New test functions in `crates/assay-types/tests/schema_snapshots.rs` — snapshot tests for new types
- [ ] Run `cargo insta review` after implementing — regenerates `gates-spec-schema` snapshot and creates new snapshots
- [ ] Schema roundtrip tests in `crates/assay-types/tests/schema_roundtrip.rs` — validate new types against their schemas

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
