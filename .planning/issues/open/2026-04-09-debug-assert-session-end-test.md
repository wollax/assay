---
title: Add catch_unwind test for evaluate_checkpoint SessionEnd debug_assert
area: assay-core
severity: suggestion
source: PR review (Phase 61)
---

The `debug_assert!` in `evaluate_checkpoint` for SessionEnd is never exercised in the test suite. Add a `#[cfg(debug_assertions)]` test using `std::panic::catch_unwind` to verify the assert fires.
