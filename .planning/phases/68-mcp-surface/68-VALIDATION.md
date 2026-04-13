---
phase: 68
slug: mcp-surface
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-12
---

# Phase 68 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`#[test]`, `#[tokio::test]`) + `serial_test` |
| **Config file** | `Cargo.toml` workspace configuration — no separate test config |
| **Quick run command** | `cargo test -p assay-mcp -- --test-threads=1` |
| **Full suite command** | `just test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-mcp -- --test-threads=1`
- **After every plan wave:** Run `just test`
- **Before `/kata:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 68-01-01 | 01 | 1 | WIZM-01 | unit | `cargo test -p assay-mcp gate_wizard_tool_in_router -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-01-02 | 01 | 1 | WIZM-01 | integration | `cargo test -p assay-mcp gate_wizard_writes_gates_toml -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-01-03 | 01 | 1 | WIZM-01 | integration | `cargo test -p assay-mcp gate_wizard_rejects_duplicate -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-02-01 | 02 | 1 | WIZM-02 | unit | `cargo test -p assay-mcp criteria_list_tool_in_router -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-02-02 | 02 | 1 | WIZM-02 | integration | `cargo test -p assay-mcp criteria_list_empty_project -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-02-03 | 02 | 1 | WIZM-02 | unit | `cargo test -p assay-mcp criteria_get_tool_in_router -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-02-04 | 02 | 1 | WIZM-02 | integration | `cargo test -p assay-mcp criteria_get_returns_library -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-02-05 | 02 | 1 | WIZM-02 | integration | `cargo test -p assay-mcp criteria_get_not_found -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-03-01 | 03 | 1 | CLIB-04 | unit | `cargo test -p assay-mcp criteria_create_tool_in_router -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-03-02 | 03 | 1 | CLIB-04 | integration | `cargo test -p assay-mcp criteria_create_writes_library -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-03-03 | 03 | 1 | CLIB-04 | integration | `cargo test -p assay-mcp criteria_create_rejects_duplicate -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-04-01 | 04 | 1 | WIZM-03 | unit | `cargo test -p assay-mcp spec_resolve_tool_in_router -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-04-02 | 04 | 1 | WIZM-03 | integration | `cargo test -p assay-mcp spec_resolve_returns_resolved_gate -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-04-03 | 04 | 1 | WIZM-03 | integration | `cargo test -p assay-mcp spec_resolve_not_found -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 68-04-04 | 04 | 1 | WIZM-03 | integration | `cargo test -p assay-mcp spec_resolve_shadow_warnings -- --test-threads=1` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

All test functions listed above are new — they will be added inline to `server.rs`'s `mod tests` block as part of implementation. No separate test files needed (all tests live in `server.rs` per convention). No additional framework install needed — `serial_test` and `tempfile` are already workspace dev-dependencies.

- [ ] All 15 test functions in the Per-Task Verification Map above — added to `mod tests` in `server.rs`

*Existing infrastructure covers framework requirements.*

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
