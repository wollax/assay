---
created: 2026-03-02T14:30
title: SpecNotFound variant declared but never constructed
area: mcp
provenance: github:wollax/assay#33
severity: important
files:
  - crates/assay-mcp/src/server.rs:243-247
  - crates/assay-core/src/error.rs:86-96
  - crates/assay-core/src/spec/mod.rs:98-118
---

## Problem

`AssayError::SpecNotFound` was forward-declared in `error.rs` for Phase 8 `spec_get` but no production code constructs it. `spec::load` returns `AssayError::Io` when the file is missing. The error message (`reading spec at '.assay/specs/foo.toml': No such file or directory`) is accurate but noisier than `spec 'foo' not found in .assay/specs/`.

Additionally, `test_domain_error_produces_error_result` in `server.rs:531` tests the dead `SpecNotFound` path, giving false confidence.

The CLI works around this by matching on `Io { source }` when `source.kind() == NotFound`. The MCP server doesn't.

## Solution

Either:
1. Construct `SpecNotFound` in `load_spec` when the Io kind is `NotFound`
2. Or have `spec::load` itself return `SpecNotFound` when the file doesn't exist
3. Update the test to use an actually-exercised error variant

## Resolution

Resolved during v0.2.0 development. `SpecNotFound` now constructed in `load_spec_entry()`.
