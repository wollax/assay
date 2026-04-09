---
phase: 60
slug: process-safety
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-08
---

# Phase 60 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | `Cargo.toml` workspace (no separate test config) |
| **Quick run command** | `cargo test -p assay-core -p assay-tui 2>/dev/null` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-core -p assay-tui 2>/dev/null`
- **After every plan wave:** Run `just test`
- **Before `/kata:verify-work`:** `just ready` (fmt-check + lint + test + deny) must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 60-01-01 | 01 | 1 | SAFE-01 | unit | `cargo test -p assay-core kill_helper` | ✅ (needs pgid update) | ⬜ pending |
| 60-01-02 | 01 | 1 | SAFE-01 | unit | `cargo test -p assay-core kill_helper_terminates_long_running_process` | ✅ (needs pgid update) | ⬜ pending |
| 60-02-01 | 02 | 1 | SAFE-02 | unit | `cargo test -p assay-core test_auto_promote_already_verified_is_noop` | ❌ W0 | ⬜ pending |
| 60-03-01 | 03 | 1 | SAFE-03 | unit | `cargo test -p assay-core launch_agent_crash_includes_stderr` | ❌ W0 | ⬜ pending |
| 60-04-01 | 04 | 1 | SAFE-04 | unit | `cargo test -p assay-core relay_panic_is_logged` | ❌ W0 | ⬜ pending |
| 60-05-01 | 05 | 1 | SAFE-05 | unit | `cargo test -p assay-tui sanitize_strips_ansi` | ❌ W0 | ⬜ pending |
| 60-05-02 | 05 | 1 | SAFE-05 | unit | `cargo test -p assay-tui sanitize_strips_csi_color` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Test for SAFE-02: `test_auto_promote_already_verified_is_noop` — double-promote produces info log, not error
- [ ] Test for SAFE-03: `launch_agent_crash_includes_stderr` — spawn child that writes stderr + exits non-zero; assert crash error contains stderr
- [ ] Test for SAFE-04: `relay_panic_is_logged` — relay thread panics; assert tracing log contains panic message, function returns -1
- [ ] Test for SAFE-05: `sanitize_strips_ansi` — `\x1b[31mred\x1b[0m` → `red`; `\x1b[?25l` → ``; plain text passes through

*Existing infrastructure covers SAFE-01 — tests exist but need pgid update.*

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
