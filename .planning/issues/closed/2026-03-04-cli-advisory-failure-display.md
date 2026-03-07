---
created: 2026-03-04T10:00
title: Advisory failures display as "FAILED" in streaming output, misleading exit status
area: assay-cli
severity: important
files:
  - crates/assay-cli/src/main.rs:653
  - crates/assay-cli/src/main.rs:664
---

## Problem

Advisory enforcement failures are displayed as "FAILED" in streaming output and counted in the `failed` summary, but the gate exits with code 0. This creates a mismatch between the output label and actual exit status, misleading users about whether the gate truly failed.

## Solution

Introduce a distinct label (e.g., "ADVISORY" or "WARNING") for advisory failures, or separate the advisory counter from the `failed` counter in the summary line to clarify the distinction.
