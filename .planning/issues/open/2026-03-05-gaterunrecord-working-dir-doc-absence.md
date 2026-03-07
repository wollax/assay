---
created: 2026-03-05T00:00
title: working_dir field doc could explain when it's absent
area: assay-types
provenance: phase-14-review
files:
  - crates/assay-types/src/gate_run.rs
---

## Problem

The `working_dir: Option<String>` field has no documentation explaining the semantics of `None`. When is it absent? Is it always present during execution but omitted in certain contexts? Callers must infer the meaning.

## Solution

Add a doc comment to the `working_dir` field explaining:
- When it's `Some(path)` vs. `None`
- Whether it's always captured or only in certain conditions
- What the path represents (absolute, relative, etc.)

