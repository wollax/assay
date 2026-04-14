---
title: lazy_evict not called from list_sessions despite docstring claim
area: assay-core
severity: bug
source: PR #7 code review
---

`lazy_evict` docstring says "Called lazily from `start_session` and `list_sessions`" but only `start_session` calls it. Sessions are only evicted on creation, not on listing. The tasks.md marks this as complete (3.3) but the implementation is incomplete. Add `lazy_evict(assay_dir)` call at the top of `list_sessions`.

File: `crates/assay-core/src/work_session.rs`
