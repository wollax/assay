---
phase: 71-tui-config-fix
verified: 2026-04-13T00:00:00Z
status: passed
score: 3/3 must-haves verified
re_verification: false
---

# Phase 71: TUI Config Fix Verification Report

**Phase Goal:** TUI reads `config.specs_dir` instead of hardcoding `root.join(".assay").join("specs")`, so projects with non-default specs directories work correctly.
**Verified:** 2026-04-13
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | TUI resolves specs directory from `config.specs_dir`, not hardcoded `"specs"` | VERIFIED | `resolved_specs_dir()` helper at app.rs line 406 reads `self.config.as_ref().map(|c| c.specs_dir.as_str()).unwrap_or("specs/")` and is used at all 8 call sites; zero occurrences of `join("specs")` remain in `crates/assay-tui/src/` |
| 2 | Projects with custom `specs_dir` config have gates written to the correct directory via TUI | VERIFIED | `test_gate_wizard_submit_honors_custom_specs_dir` in `gate_wizard_app.rs` passes: sets `specs_dir = "custom-specs/"` in config.toml, drives gate wizard submit, asserts `custom-specs/test-gate/gates.toml` exists and `specs/test-gate/` does NOT exist |
| 3 | Projects with no `config.toml` fall back to default `"specs/"` path silently | VERIFIED | `resolved_specs_dir()` uses `unwrap_or("specs/")` when `self.config` is `None`; all 108 pre-existing tests use default fixture paths and pass without modification |

**Score:** 3/3 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/assay-tui/src/app.rs` | `resolved_specs_dir()` helper replacing 7 hardcoded sites | VERIFIED | Helper defined at line 406; called at 8 sites (7 in-method + 1 via `execute_slash_cmd`); confirmed by grep count of 9 (1 def + 8 uses) |
| `crates/assay-tui/src/slash.rs` | `execute_slash_cmd` accepts `specs_dir: &Path` parameter | VERIFIED | Signature at line 138: `pub fn execute_slash_cmd(cmd: SlashCmd, project_root: &Path, specs_dir: &Path) -> String`; no `join("specs")` inside the function body |
| `crates/assay-tui/tests/gate_wizard_app.rs` | Test `test_gate_wizard_submit_honors_custom_specs_dir` verifying custom path | VERIFIED | Full test at lines 263-313; creates `config.toml` with `specs_dir = "custom-specs/"`, drives wizard, asserts correct path written and default path absent |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/assay-tui/src/app.rs` | `self.config` | `resolved_specs_dir()` reads `config.specs_dir` with fallback | WIRED | Helper reads `self.config.as_ref().map(|c| c.specs_dir.as_str()).unwrap_or("specs/")` |
| `crates/assay-tui/src/app.rs` | `crates/assay-tui/src/slash.rs` | `execute_slash_cmd` call passes resolved `specs_dir` | WIRED | app.rs line 1005: `execute_slash_cmd(cmd, root, &self.resolved_specs_dir(&assay_dir))` |

### Requirements Coverage

No formal requirement IDs declared in PLAN frontmatter (`requirements: []`). This is a gap-closure fix (`gap_closure: true`) with no REQUIREMENTS.md entries. No orphaned requirements found.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/assay-tui/src/app.rs` | 1756 | `placeholder` identifier (UI list widget label "Starting...") | Info | Not a stub — it is an intentional UX label for an empty list state |

No blocking or warning-level anti-patterns found.

### Human Verification Required

None. All goal truths are mechanically verifiable: path construction, function signatures, and test assertions are all deterministic and confirmed via grep and test execution.

### Gaps Summary

No gaps. All three truths are fully verified:

1. The `resolved_specs_dir()` helper is substantive (reads from `self.config`, not hardcoded), defined once, and called at every site that previously hardcoded `join("specs")`.
2. The `execute_slash_cmd` signature change is complete — caller resolves the path before passing it in, matching the locked design decision.
3. The integration test exercises the full end-to-end path: config load → helper invocation → file write → assertion on the custom directory.

`just ready` (109 assay-tui tests + full workspace) passes per SUMMARY.md; both implementation commits (3061c0e, 3a59306) confirmed present on main branch.

---

_Verified: 2026-04-13_
_Verifier: Claude (kata-verifier)_
