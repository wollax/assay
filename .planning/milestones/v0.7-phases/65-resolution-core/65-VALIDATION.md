---
phase: 65
slug: resolution-core
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-11
---

# Phase 65 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` (no external test runner) |
| **Config file** | none (cargo workspace) |
| **Quick run command** | `cargo test -p assay-core spec::compose` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-core spec::compose && cargo test -p assay-types criteria_library`
- **After every plan wave:** Run `just test`
- **Before `/kata:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 65-01-01 | 01 | 1 | INHR-03 | unit | `cargo test -p assay-core compose::tests::cycle_self_extend` | ❌ W0 | ⬜ pending |
| 65-01-01 | 01 | 1 | INHR-03 | unit | `cargo test -p assay-core compose::tests::cycle_mutual_extend` | ❌ W0 | ⬜ pending |
| 65-01-01 | 01 | 1 | INHR-04 | unit | `cargo test -p assay-core compose::tests::source_annotation_own` | ❌ W0 | ⬜ pending |
| 65-01-01 | 01 | 1 | INHR-04 | unit | `cargo test -p assay-core compose::tests::source_annotation_parent` | ❌ W0 | ⬜ pending |
| 65-01-01 | 01 | 1 | INHR-04 | unit | `cargo test -p assay-core compose::tests::own_wins_merge` | ❌ W0 | ⬜ pending |
| 65-01-02 | 01 | 1 | CLIB-01 | unit | `cargo test -p assay-core compose::tests::load_library_valid` | ❌ W0 | ⬜ pending |
| 65-01-02 | 01 | 1 | CLIB-02 | unit | `cargo test -p assay-core compose::tests::resolve_includes_library` | ❌ W0 | ⬜ pending |
| 65-01-02 | 01 | 1 | CLIB-02 | unit | `cargo test -p assay-core compose::tests::resolve_invalid_include_slug` | ❌ W0 | ⬜ pending |
| 65-01-02 | 01 | 1 | CLIB-03 | unit | `cargo test -p assay-core compose::tests::save_library_roundtrip` | ❌ W0 | ⬜ pending |
| 65-01-02 | 01 | 1 | CLIB-03 | unit | `cargo test -p assay-core compose::tests::scan_libraries_missing_dir` | ❌ W0 | ⬜ pending |
| 65-01-02 | 01 | 1 | CLIB-03 | unit | `cargo test -p assay-core compose::tests::scan_libraries_finds_files` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/assay-core/src/spec/compose.rs` — module with inline `#[cfg(test)] mod tests` stubs for all requirements
- [ ] `crates/assay-types/src/resolved.rs` — ResolvedGate, ResolvedCriterion, CriterionSource types

*Existing infrastructure covers test framework — no new framework install needed.*

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
