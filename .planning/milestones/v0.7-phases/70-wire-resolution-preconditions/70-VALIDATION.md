---
phase: 70
slug: wire-resolution-preconditions
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-13
---

# Phase 70 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (standard Rust) |
| **Config file** | Workspace Cargo.toml |
| **Quick run command** | `cargo test -p assay-core -p assay-cli -p assay-mcp` |
| **Full suite command** | `just ready` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p assay-core -p assay-cli -p assay-mcp`
- **After every plan wave:** Run `just test`
- **Before `/kata:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 70-01-01 | 01 | 1 | INHR-02 | integration | `cargo test -p assay-cli -- gate_run_extends` | ❌ W0 | ⬜ pending |
| 70-01-02 | 01 | 1 | INHR-04 | unit | `cargo test -p assay-cli -- stream_criterion_source_tag` | ❌ W0 | ⬜ pending |
| 70-01-03 | 01 | 1 | INHR-04 | unit | `cargo test -p assay-cli -- gate_run_json_source_annotation` | ❌ W0 | ⬜ pending |
| 70-01-04 | 01 | 1 | CLIB-02 | integration | `cargo test -p assay-cli -- gate_run_include_library` | ❌ W0 | ⬜ pending |
| 70-01-05 | 01 | 1 | PREC-01 | integration | `cargo test -p assay-cli -- gate_run_precondition_requires_blocked` | ❌ W0 | ⬜ pending |
| 70-01-06 | 01 | 1 | PREC-02 | integration | `cargo test -p assay-cli -- gate_run_precondition_command_blocked` | ❌ W0 | ⬜ pending |
| 70-01-07 | 01 | 1 | PREC-03 | integration | `cargo test -p assay-cli -- gate_run_exit_code_precondition_failed` | ❌ W0 | ⬜ pending |
| 70-01-08 | 01 | 1 | PREC-03 | integration | `cargo test -p assay-mcp -- gate_run_precondition_failed_mcp` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Integration tests in `crates/assay-cli/src/commands/gate.rs` `#[cfg(test)]` — stubs for INHR-02, INHR-04, CLIB-02, PREC-01, PREC-02, PREC-03
- [ ] Integration tests in `crates/assay-mcp/src/server.rs` `#[cfg(test)]` — stubs for PREC-03 MCP path

*No framework installation needed — cargo test is already configured.*

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
