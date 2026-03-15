> **Closed:** 2026-03-15 — Won't fix. Superseded by v0.4.0 architecture (phases 35-44).


---
title: "CLI spec command cleanup"
area: assay-cli
priority: medium
source: PR review #27
---

# CLI Spec Command Cleanup

## Problem

Several code quality issues in the CLI spec commands:

1. **ANSI escape byte count magic number** — `format_criteria_type()` uses a hard-coded `9` for ANSI escape code byte overhead when calculating column widths. Should be a named constant or computed from the escape sequences.
2. **specs_dir path could double-join `.assay/`** — if `config.specs_dir` is an absolute path or already includes `.assay/`, the path construction in `handle_spec_show`/`handle_spec_list` could produce an incorrect path. Currently safe because `default_specs_dir()` returns `"specs/"`, but fragile if the default changes.
3. **spec list exits 0 even with scan errors** — when `scan()` returns partial results (some specs + some errors), the CLI prints warnings but exits 0. Consider exiting non-zero when errors are present, or at minimum document the behavior.

Additional findings from second review:

4. **`NO_COLOR` should use `var_os().is_none()` not `var().is_err()`** — non-UTF-8 env var values incorrectly enable colors. `var_os()` handles all byte sequences per the no-color.org spec.
5. **MCP error uses `{e:?}` (Debug) while all others use `{e}` (Display)** — main.rs:297 exposes Rust Debug output to users instead of human-readable messages.
6. **`current_dir()` duplicated inline in init** — init arm calls `current_dir()` directly instead of reusing the `project_root()` helper, creating maintenance divergence.
7. **spec list alignment inconsistent** — specs without descriptions don't pad slug to column width, causing misalignment when mixed with specs that have descriptions.

## Solution

- Extract ANSI byte count to a `const`
- Add a path normalization check or assertion for specs_dir
- Decide on exit code policy for partial scan results
- Switch to `var_os().is_none()` for NO_COLOR
- Fix MCP error format to `{e}` or `{e:#}`
- Route init through `project_root()`
- Always pad slug in spec list output