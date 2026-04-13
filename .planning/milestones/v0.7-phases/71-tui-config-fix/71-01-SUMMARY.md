---
phase: 71-tui-config-fix
plan: "01"
subsystem: assay-tui
tags: [tui, config, specs_dir, bug-fix, gap-closure]
dependency_graph:
  requires: []
  provides: [tui-specs-dir-resolution]
  affects: [assay-tui/app.rs, assay-tui/slash.rs]
tech_stack:
  added: []
  patterns: [resolved_specs_dir helper, parameter-passing for resolved paths]
key_files:
  created: []
  modified:
    - crates/assay-tui/src/app.rs
    - crates/assay-tui/src/slash.rs
    - crates/assay-tui/tests/gate_wizard_app.rs
decisions:
  - "resolved_specs_dir() falls back to 'specs/' (matching default_specs_dir()) when self.config is None"
  - "execute_slash_cmd accepts specs_dir: &Path parameter — resolved by caller (App) not internally"
  - "Config validation requires project_name — integration test must include it in config.toml fixture"
metrics:
  duration: "5 minutes"
  completed: "2026-04-13"
  tasks_completed: 2
  files_modified: 3
---

# Phase 71 Plan 01: TUI specs_dir Config Fix Summary

**One-liner:** Replaced 7 hardcoded `join("specs")` sites in TUI with `resolved_specs_dir()` helper reading `config.specs_dir` with silent fallback.

## What Was Built

Fixed TUI to resolve the specs directory from loaded config instead of hardcoding `.assay/specs`. The CLI already used `assay_dir.join(&config.specs_dir)` correctly; the TUI was ignoring the config entirely.

**Changes:**

- `App::resolved_specs_dir(&self, assay_dir: &Path) -> PathBuf` — reads `self.config.specs_dir` with `"specs/"` fallback when no config loaded
- `execute_slash_cmd` signature updated to `(cmd, project_root, specs_dir: &Path)` — caller passes resolved path
- All 7 `join("specs")` sites in `app.rs` replaced with `resolved_specs_dir()` call
- 4 `&specs_dir` → `specs_dir` fixes in `slash.rs` (clippy `needless_borrow` — `specs_dir` is now `&Path` not `PathBuf`)
- New integration test `test_gate_wizard_submit_honors_custom_specs_dir` verifying end-to-end custom path

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add resolved_specs_dir helper and replace all hardcoded sites | 3061c0e | app.rs, slash.rs |
| 2 | Add integration test verifying custom specs_dir config honored | 3a59306 | gate_wizard_app.rs |

## Verification

```
# Zero hardcoded join("specs") in TUI source
grep -rn 'join("specs")' crates/assay-tui/src/ | wc -l  # → 0

# All tests pass
just ready  # → 2500 tests passed
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy needless_borrow in slash.rs after signature change**
- **Found during:** Task 1 commit (pre-commit hook)
- **Issue:** After changing `specs_dir` from `PathBuf` local to `&Path` parameter, 4 call sites used `&specs_dir` creating a double-borrow
- **Fix:** Changed `&specs_dir` → `specs_dir` at 4 call sites in slash.rs
- **Files modified:** crates/assay-tui/src/slash.rs
- **Commit:** 3061c0e (included in same commit after pre-commit fix)

**2. [Rule 2 - Missing validation] Config.toml fixture missing project_name**
- **Found during:** Task 2 TDD GREEN phase (test failed)
- **Issue:** `config::validate()` requires non-empty `project_name`; our test fixture only had `specs_dir` which caused config load to fail silently, falling back to default path
- **Fix:** Added `project_name = "test-project"` to the config.toml fixture in the test
- **Files modified:** crates/assay-tui/tests/gate_wizard_app.rs

## Decisions Made

- `resolved_specs_dir()` falls back to `"specs/"` (with trailing slash, matching `default_specs_dir()`) when `self.config` is `None`
- `execute_slash_cmd` receives `specs_dir: &Path` from App — App owns the resolution, not slash.rs
- Config validation requires `project_name` — integration tests must include it in config fixtures

## Self-Check: PASSED
