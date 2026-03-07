---
created: 2026-03-04T10:00
title: Add rollback/cleanup on partial failure in handle_spec_new
area: assay-cli
severity: important
files:
  - crates/assay-cli/src/main.rs:556-563
---

## Problem

`handle_spec_new()` writes multiple files (spec.toml, gates.toml, feature.toml) without rollback if a later file write fails. This leaves the repository in an inconsistent state.

## Solution

Implement cleanup logic that removes the created spec directory on partial failure, or use transactional writes (write to temp dir, then atomic move).
