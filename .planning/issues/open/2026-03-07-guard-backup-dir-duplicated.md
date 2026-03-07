---
created: 2026-03-07T08:00
title: backup_dir path construction duplicated in soft/hard threshold handlers
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/daemon.rs:207
  - crates/assay-core/src/guard/daemon.rs:251
---

## Problem

Both `handle_soft_threshold` and `handle_hard_threshold` independently construct `self.assay_dir.join("backups")` for the backup directory path. This duplicates a path convention that could drift if one is updated without the other.

## Solution

Extract the backup directory path into a helper method (e.g., `fn backup_dir(&self) -> PathBuf`) or a constant/field on `GuardDaemon`.
