---
created: 2026-03-07T08:00
title: handle_guard_start auto-discovers session via glob — could match wrong file
area: assay-cli
severity: important
files:
  - crates/assay-cli/src/main.rs:2434
---

## Problem

When no `--session` argument is provided, `handle_guard_start` calls `find_session_dir` and `resolve_session` which use glob-based discovery to locate a session file. In directories with multiple active sessions (e.g., concurrent Claude Code instances), this could silently pick the wrong session file with no user feedback about which file was selected.

## Solution

Print the resolved session path to stderr before starting the daemon so the user can verify. Consider also warning or erroring when multiple candidate sessions are found, requiring the user to disambiguate with `--session`.
