---
title: evaluate_routed doc claims two-phase ordering that implementation doesn't follow
area: assay-core
severity: suggestion
source: PR #7 code review
---

The `evaluate_routed` doc comment previously described a two-phase evaluation ("Path 1 first, then Path 3") with merging, but the implementation processes all criteria in declaration order via a single `for` loop. The doc was partially corrected but the function description at lines 280-283 still describes routing semantics that could be clearer about the single-pass nature.

File: `crates/assay-core/src/gate/mod.rs`
