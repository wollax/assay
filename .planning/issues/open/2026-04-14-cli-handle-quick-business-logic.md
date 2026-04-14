---
title: handle_quick in CLI constructs Criterion structs directly
area: assay-cli
severity: nitpick
source: PR #7 code review
---

`handle_quick` in `crates/assay-cli/src/commands/plan.rs` constructs `assay_types::Criterion` structs with all default fields inline. CLAUDE.md says CLI crates should be thin wrappers. Consider moving criterion construction to a helper in assay-core (e.g., `Criterion::quick(name, cmd)`) so the CLI only passes raw user input.

File: `crates/assay-cli/src/commands/plan.rs`, lines 52-88
