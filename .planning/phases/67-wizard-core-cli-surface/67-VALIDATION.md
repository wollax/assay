---
phase: 67
slug: wizard-core-cli-surface
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-12
---

# Phase 67 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace) |
| **Config file** | `Cargo.toml` (workspace) + per-crate `tests/` dirs |
| **Quick run command** | `rtk cargo test -p assay-core --lib` |
| **Full suite command** | `just ready` (fmt-check + lint + test + deny) |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run quick run command (scoped to modified crate)
- **After every plan wave:** Run `rtk cargo test` for affected crates
- **Before `/kata:verify-work`:** `just ready` must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| TBD — populated by planner based on task breakdown |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Confirm `dialoguer::Input::validate_with` API signature against 0.12.0 (research open question)
- [ ] Confirm `dialoguer::Input::with_initial_text` API for edit-mode defaults
- [ ] Test stub files for new `assay-core::wizard` public surface (`apply_gate_wizard`, `apply_criteria_wizard`)
- [ ] Test stub files for new CLI commands (`gate wizard`, `criteria list`, `criteria new`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Interactive TTY prompt UX | WIZC-01 / WIZC-02 | `dialoguer` requires a real TTY; headless tests exercise the underlying `apply_*_wizard` input struct, not the prompt loop | Run `cargo run -p assay-cli -- gate wizard` in an interactive terminal and complete the flow |
| Edit-mode pre-fill rendering | WIZC-02 | Same — `with_initial_text` behavior is visual | Run `assay gate wizard --edit <existing>` and verify prefilled fields |

*All non-interactive logic (input validation, TOML serialization, file I/O) is covered by automated tests.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
