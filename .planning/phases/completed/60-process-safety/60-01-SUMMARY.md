---
phase: 60-process-safety
plan: "01"
subsystem: tui-security
tags: [sanitize, ansi, regex-lite, terminal-injection, process-safety]
requires: []
provides:
  - Full ANSI/CSI escape sequence stripping in TUI output display
affects:
  - Phase 63 (if any further display-pipeline hardening is planned)
tech-stack:
  added: []
  patterns:
    - OnceLock<Regex> for lazily-compiled static regex in Rust
key-files:
  created: []
  modified:
    - crates/assay-tui/src/app.rs
    - crates/assay-tui/Cargo.toml
decisions:
  - "Lifted sanitize() from nested function to module-level pub(crate) for testability"
  - "Used OnceLock<regex_lite::Regex> (stdlib, no extra deps) for lazy regex compilation"
  - "Pattern covers CSI sequences with ? (private modes like ?25l) and single-char Fe sequences"
metrics:
  duration: "~13 minutes"
  completed: "2026-04-09"
---

# Phase 60 Plan 01: ANSI Sanitize Rewrite Summary

**One-liner:** Regex-based full ANSI/CSI sequence stripping in TUI display using `regex-lite` + `OnceLock`, replacing per-byte ESC substitution.

## What Was Done

Replaced the existing `sanitize()` function (which only replaced the bare ESC byte with U+FFFD, leaving `[31m` garbage visible) with a proper ANSI-stripping implementation:

1. Added `regex-lite.workspace = true` to `crates/assay-tui/Cargo.toml`.
2. Lifted `sanitize()` from a nested function inside `handle_agent_event` to a module-level `pub(crate)` function in `app.rs`.
3. Used `OnceLock<regex_lite::Regex>` to compile the pattern once:
   - `\x1b\[[0-9;?]*[A-Za-z]` — CSI sequences (including private-mode variants like `\x1b[?25l`)
   - `\x1b[^\[]` — single-char Fe sequences (OSC intro and similar)
4. After stripping ANSI sequences, replaced remaining control chars (< 0x20, except tab) with U+FFFD.
5. Added 8 unit tests covering all required behaviors in `mod sanitize_tests`.

## Verification

- `rtk cargo test -p assay-tui sanitize` — 8/8 pass
- `rtk cargo clippy -p assay-tui -- -D warnings` — clean
- `rtk cargo test -p assay-tui` — 92/92 pass (full suite)

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| `397289f` | feat(60-01): rewrite sanitize() with regex-lite ANSI stripping |
