---
title: EvictionSummary is pub but unused outside the crate
area: assay-core
severity: nitpick
source: PR #7 code review
---

`pub struct EvictionSummary` is publicly exposed but never consumed outside the crate. `lazy_evict` discards the return value. Consider making it `pub(crate)` or removing the return type from `evict_sessions`.

File: `crates/assay-core/src/work_session.rs`
