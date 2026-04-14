---
title: evict_sessions count-based logic overcounts when protected sessions exist
area: assay-core
severity: bug
source: PR #7 code review
---

In `evict_sessions`, the `remaining` vector includes protected sessions (which can't be deleted), inflating the `excess` count. The inner loop skips protected sessions, so fewer are deleted than intended. Fix: subtract protected session count from `remaining.len()` before computing `excess`, or filter protected sessions out of `remaining`.

File: `crates/assay-core/src/work_session.rs`
