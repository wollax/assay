---
title: Clarify version semantics in show_status
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

The `show_status` function has a doc comment that says it shows "version" without clarifying what kind of version. The implementation uses `CARGO_PKG_VERSION` (the binary version), not a project version. This ambiguity could confuse future contributors who might assume it displays something different, leading to incorrect modifications or assumptions about what the code should do.

## Solution

Update the `show_status` doc comment to explicitly state that it displays the binary version (`CARGO_PKG_VERSION`), not a project version. This clarifies the intent for future maintainers.
