---
title: TUI line buffer uses Vec with O(n) remove(0) — consider VecDeque
severity: low
area: assay-tui/app
source: PR review (Phase 60)
---

In `app.rs:317-320`, `push_line` caps the line buffer at 10,000 entries using `lines.remove(0)`, which is O(n) for a Vec. For long-running agents with high output, this accumulates. A `VecDeque` would give O(1) `pop_front`. Low priority for typical agent runs.
