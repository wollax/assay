---
title: NextAction::RunGates and NextAction::PromptUat are never constructed
area: assay-core
severity: suggestion
source: PR #7 code review
---

`NextAction::RunGates` and `NextAction::PromptUat` are enum variants with doc comments describing valid workflow states, but `next_action()` never constructs them anywhere in the codebase. Either implement the logic paths that return these variants or remove them to avoid dead code.

File: `crates/assay-core/src/workflow/mod.rs`, lines 34-45
