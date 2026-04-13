---
phase: 69
slug: tui-surface
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-12
---

# Phase 69 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + integration test files |
| **Config file** | `crates/assay-tui/Cargo.toml` (no separate test config — Rust standard) |
| **Quick run command** | `cargo test -p assay-tui --test gate_wizard` |
| **Full suite command** | `cargo test -p assay-tui` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-tui --test gate_wizard`
- **After every plan wave:** Run `cargo test -p assay-tui`
- **Before `/kata:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 69-01-01 | 01 | 1 | WIZT-01 | integration | `cargo test -p assay-tui --test gate_wizard_round_trip` | ❌ W0 | ⬜ pending |
| 69-01-02 | 01 | 1 | WIZT-01 | integration | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_edit_mode_prefill` | ❌ W0 | ⬜ pending |
| 69-01-03 | 01 | 1 | WIZT-01 | unit | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_cancel_returns_to_dashboard` | ❌ W0 | ⬜ pending |
| 69-01-04 | 01 | 1 | WIZT-01 | integration | `cargo test -p assay-tui --test gate_wizard_app -- test_g_key_opens_gate_wizard` | ❌ W0 | ⬜ pending |
| 69-01-05 | 01 | 1 | WIZT-01 | integration | `cargo test -p assay-tui --test gate_wizard_app -- test_slash_gate_wizard` | ❌ W0 | ⬜ pending |
| 69-01-06 | 01 | 1 | WIZT-01 | integration | `cargo test -p assay-tui --test gate_wizard_app -- test_slash_gate_edit` | ❌ W0 | ⬜ pending |
| 69-01-07 | 01 | 1 | WIZT-02 | integration | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_invalid_slug_shows_error` | ❌ W0 | ⬜ pending |
| 69-01-08 | 01 | 1 | WIZT-02 | unit | `cargo test -p assay-tui --test gate_wizard_round_trip -- test_no_tui_side_validation` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/assay-tui/tests/gate_wizard_round_trip.rs` — unit/integration tests for the state machine (step advance, assemble_gate_input, cancel, backspace)
- [ ] `crates/assay-tui/tests/gate_wizard_app.rs` — App-level integration tests for keybindings and slash commands

*Existing infrastructure covers framework installation — only test files are missing.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Visual layout renders correctly in terminal | WIZT-01 | Pixel-level rendering requires human eye | Run TUI, navigate to gate wizard, verify layout is readable |
| Multi-select toggle UX feels natural | WIZT-01 | Usability is subjective | Step through includes selection, verify space toggles, Enter confirms |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
