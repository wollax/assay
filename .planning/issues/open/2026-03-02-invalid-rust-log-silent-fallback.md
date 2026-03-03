---
title: Provide feedback when RUST_LOG filter is invalid
area: assay-cli
priority: low
source: Phase 9 PR review
---

## Problem

In `init_mcp_tracing`, the code calls `EnvFilter::try_from_default_env().unwrap_or_else(|_| ...)` which silently falls back to the `warn` level when `RUST_LOG` contains an invalid value. The user receives no feedback that their filter was ignored and the default was used instead. This creates a silent failure mode that can be confusing during debugging.

## Solution

Emit a warning or info log message when the `RUST_LOG` environment variable is invalid and the fallback is used. This gives users visibility into what happened and helps them understand why their filter didn't take effect.
