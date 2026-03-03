---
title: Extract .assay directory path to named constant
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

The string literal `".assay"` appears in multiple handler functions throughout the codebase (e.g., `root.join(".assay")`). Having scattered string literals makes it harder to maintain and refactor. If the directory name ever needs to change, multiple locations must be updated. This is a classic case where a named constant would improve code clarity and maintainability.

## Solution

Define a module-level constant (e.g., `const ASSAY_DIR: &str = ".assay"`) and use it throughout the handler functions instead of the raw string literal. This centralizes the value and makes it easier to maintain and reason about.
