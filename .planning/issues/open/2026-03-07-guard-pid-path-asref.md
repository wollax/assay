---
created: 2026-03-07T08:00
title: pid_file_path could accept AsRef<Path> instead of &Path
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/guard/pid.rs:9
---

## Problem

`pid_file_path` takes `&Path` which requires callers with `PathBuf` to explicitly borrow. Using `AsRef<Path>` would make the API more ergonomic and consistent with Rust standard library conventions.

## Solution

Change the signature to `pub fn pid_file_path(assay_dir: impl AsRef<Path>) -> PathBuf` and use `assay_dir.as_ref()` internally.
