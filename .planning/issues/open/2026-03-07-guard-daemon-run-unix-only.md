---
created: 2026-03-07T08:00
title: daemon.rs run() is #[cfg(unix)] only with no compile error guidance
area: assay-core
severity: important
files:
  - crates/assay-core/src/guard/daemon.rs:43
---

## Problem

The `run()` method on `GuardDaemon` is gated behind `#[cfg(unix)]` but there is no `#[cfg(not(unix))]` stub that produces a compile error or returns an unsupported-platform error. Code that constructs a `GuardDaemon` on non-Unix platforms compiles fine but has no way to start it, leading to a confusing API surface.

## Solution

Add a `#[cfg(not(unix))]` implementation of `run()` that returns an explicit `Err(AssayError::UnsupportedPlatform)` or uses `compile_error!` to fail at build time, making the platform constraint discoverable.
