---
title: Legacy specs default to "draft" status in spec_list filter
area: assay-mcp
severity: bug
source: PR #7 code review
---

In `spec_list`, legacy specs (`SpecEntry::Legacy`) have `status: None`, which defaults to `"draft"` via `unwrap_or("draft")` during filtering. This means `spec_list(status: "draft")` returns all legacy specs regardless of their actual state. Either explicitly exclude legacy specs from status filtering or assign them an effective status.

File: `crates/assay-mcp/src/server.rs`
