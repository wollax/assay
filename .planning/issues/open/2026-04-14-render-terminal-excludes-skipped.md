---
title: render_terminal total denominator excludes skipped criteria
area: assay-core
severity: suggestion
source: PR #7 code review
---

`render_terminal` calculates `total = s.passed + s.failed`, excluding `s.skipped`. With `evaluate_routed` skipping AgentReport criteria, a spec with 3 command + 2 skipped agent criteria shows "3/3 passed" instead of indicating that 2 criteria were not evaluated. Consider showing "3/3 passed (2 skipped)" or similar.

File: `crates/assay-core/src/gate/render.rs`
