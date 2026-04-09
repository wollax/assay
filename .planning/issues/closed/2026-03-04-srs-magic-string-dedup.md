---
created: 2026-03-04T10:00
title: Extract [srs] magic string to constant
area: assay-cli
severity: suggestion
files:
  - crates/assay-cli/src/main.rs:303,476,945
---

## Problem

The string literal `"[srs]"` is repeated in 3 places without being a named constant. This makes it difficult to change uniformly and reduces code readability.

## Solution

Define a module-level constant (e.g., `const SRS_PREFIX: &str = "[srs]"`) and use it in all three locations.
