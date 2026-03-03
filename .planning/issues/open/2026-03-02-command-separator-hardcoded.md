---
title: Command column separator should be data-driven
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

In `handle_spec_show`, the separator for the "Command" column is hardcoded as `"\u{2500}".repeat(7)` regardless of the actual command lengths. Other columns compute their separator width based on the data, but the Command column uses a fixed magic number. This is inconsistent and makes the output fragile if command widths change.

## Solution

Compute the Command column separator width dynamically based on the actual command content, similar to how other columns are handled. Store the maximum command width and use it to determine the separator length.
