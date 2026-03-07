---
created: 2026-03-01T04:30
title: CLI main() should return Result for error propagation
area: assay-cli
provenance: github:wollax/assay#13
files:
  - crates/assay-cli/src/main.rs:11
---

## Problem

`main()` returns `()` instead of `Result`. Future subcommand handlers that return errors have no propagation path — developers will be forced to use `process::exit(1)` inline or ad hoc `eprintln!` calls. Errors may exit with code 0, causing CI pipelines to treat failures as successes.

## Solution

Change `main()` to return `color_eyre::Result<()>` (color-eyre is already a workspace dependency). Add `color_eyre::install()?` and return `Ok(())`.

## Resolution

Resolved during Phase 18 (CLI hardening). `main()` uses async with `run() -> Result<i32>`.
