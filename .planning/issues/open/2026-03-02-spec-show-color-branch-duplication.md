---
title: Simplify color branch duplication in handle_spec_show
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

In `handle_spec_show`, for each row being printed, there is an `if color { println!(...) } else { println!(...) }` block that duplicates the formatting logic. The only difference is whether ANSI color codes are included. This pattern repeats for multiple rows, creating maintenance burden and reducing readability.

## Solution

Compute `type_w` (the computed column width) once before the `println!` call, then construct the output string separately from the color decision. This way, a single `println!` can be called with the formatted string, whether colors are enabled or not.
