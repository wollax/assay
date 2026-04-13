---
phase: 71
slug: tui-config-fix
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-13
---

# Phase 71 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + integration test files in `tests/` |
| **Config file** | `crates/assay-tui/` (workspace Cargo.toml) |
| **Quick run command** | `rtk cargo test -p assay-tui` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `rtk cargo test -p assay-tui`
- **After every plan wave:** Run `just ready`
- **Before `/kata:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 71-01-01 | 01 | 1 | specs_dir resolution | unit/integration | `rtk cargo test -p assay-tui` | ✅ | ⬜ pending |
| 71-01-02 | 01 | 1 | slash.rs signature | unit/integration | `rtk cargo test -p assay-tui` | ✅ | ⬜ pending |
| 71-01-03 | 01 | 1 | custom specs_dir test | integration | `rtk cargo test -p assay-tui` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] New integration test verifying custom `specs_dir` honored in gate wizard write path (covers success criterion 2)

*Existing infrastructure covers regression testing of the default path (all existing test files already pass).*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
