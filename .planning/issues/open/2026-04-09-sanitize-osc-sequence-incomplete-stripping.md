---
title: TUI sanitize only strips OSC intro, not full OSC payload
severity: low
area: assay-tui/app
source: PR review (Phase 60)
---

The ANSI regex `\x1b[^\[]` strips single-char Fe sequences including the OSC intro (`\x1b]`), but does not handle full OSC sequences like `\x1b]0;window title\x07`. The payload (`0;window title`) and string terminator (`\x07` or `\x1b\\`) pass through as plaintext.

Low risk for agent output (agents rarely emit OSC), but worth adding a proper OSC pattern and tests for: incomplete CSI `\x1b[`, back-to-back sequences, empty string input.
