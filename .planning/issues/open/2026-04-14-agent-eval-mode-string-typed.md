---
title: agent_eval_mode is a String instead of an enum
area: assay-types
severity: suggestion
source: PR #7 code review
---

`agent_eval_mode` in `GatesConfig` is modeled as a `String` with string equality checks (`== "manual"`) in gate/mod.rs. The same PR introduces `AutoIsolate` as a proper enum for an equivalent config choice. CLAUDE.md says "Lean towards functional and declarative patterns." An `AgentEvalMode` enum in assay-types (similar to `AutoIsolate`) would be more type-safe and consistent.

Files: `crates/assay-types/src/lib.rs` (GatesConfig), `crates/assay-core/src/gate/mod.rs` (string comparison)
